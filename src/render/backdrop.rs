//! Bounded GPU-only backdrop cache for optical controls. No readback or capture process.
use crate::{damage, raster::Size, renderer::Draw, state::Rect};
use gl::types::*;

pub const REACH: f32 = 14.;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Base,
    Panel,
}
impl Layer {
    pub const fn index(self) -> usize {
        match self {
            Self::Base => 0,
            Self::Panel => 1,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    pub opacity: f32,
    pub backdrop: Option<Layer>,
}
impl Default for Effect {
    fn default() -> Self {
        Self {
            opacity: 1.,
            backdrop: Some(Layer::Base),
        }
    }
}

pub fn size(drawable: Size, max_texture: u32) -> Size {
    let units = (drawable.width.min(1920).min(max_texture) / 16)
        .min(drawable.height.min(1080).min(max_texture) / 9)
        .max(1);
    Size::new(units * 16, units * 9)
}
pub struct Plan {
    pub end: usize,
    pub region: Rect,
    pub coverage: Vec<Rect>,
    pub key: Vec<Draw>,
}
fn intersection(a: Rect, b: Rect) -> Option<Rect> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let r = Rect::new(
        x,
        y,
        (a.x + a.w).min(b.x + b.w) - x,
        (a.y + a.h).min(b.y + b.h) - y,
    );
    (r.w > 0. && r.h > 0.).then_some(r)
}
fn contains(a: Rect, b: Rect) -> bool {
    a.x <= b.x && a.y <= b.y && a.x + a.w >= b.x + b.w && a.y + a.h >= b.y + b.h
}
/// Only lower content contributes. Foreground labels and content outside all lenses
/// do not refresh the cache, even when they happen to lie between two glass controls.
pub fn plan(commands: &[Draw], layer: Layer) -> Option<Plan> {
    let screen = Rect::new(0., 0., 1920., 1080.);
    let mut coverage = vec![];
    let mut clip = None;
    for command in commands {
        if let Draw::Clip(next) = command {
            clip = *next;
        }
        if let Draw::Glass(rect, _, _, _, effect) = command
            && effect.backdrop == Some(layer)
            && effect.opacity > 0.
        {
            let visible = intersection(*rect, clip.unwrap_or(screen));
            if let Some(r) = visible {
                let r = r.inset(-REACH);
                // Coarse outward coverage avoids refreshes for tiny clock/lens movements.
                let x = (r.x / 32.).floor() * 32.;
                let y = (r.y / 32.).floor() * 32.;
                let r = Rect::new(
                    x,
                    y,
                    ((r.x + r.w) / 32.).ceil() * 32. - x,
                    ((r.y + r.h) / 32.).ceil() * 32. - y,
                );
                if let Some(r) = intersection(r, screen) {
                    coverage.push(r);
                }
            }
        }
    }
    let end = match layer {
        Layer::Base => commands.iter().position(|d| matches!(d, Draw::Glass(..)))?,
        Layer::Panel => commands
            .iter()
            .position(|d| matches!(d, Draw::Backdrop(Layer::Panel)))?,
    };
    let region = coverage.iter().copied().reduce(|a, b| {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        Rect::new(
            x,
            y,
            (a.x + a.w).max(b.x + b.w) - x,
            (a.y + a.h).max(b.y + b.h) - y,
        )
    })?;
    let key = commands[..end]
        .iter()
        .filter(|d| {
            damage::bounds(d).is_none_or(|r| coverage.iter().any(|c| damage::intersects(*c, r)))
        })
        .cloned()
        .collect();
    Some(Plan {
        end,
        region,
        coverage,
        key,
    })
}

#[derive(Default)]
pub struct Cache {
    pub target: Option<Target>,
    pub key: Vec<Draw>,
    pub region: Rect,
    pub coverage: Vec<Rect>,
    pub valid: bool,
    pub generation: u64,
    pub dependency: u64,
    pub failed_size: Option<Size>,
}
impl Cache {
    pub fn matches(&self, plan: &Plan, size: Size, dependency: u64) -> bool {
        self.valid
            && self
                .target
                .as_ref()
                .is_some_and(|target| target.size == size)
            && self.dependency == dependency
            && contains(self.region, plan.region)
            && plan
                .coverage
                .iter()
                .all(|r| self.coverage.iter().any(|cached| contains(*cached, *r)))
            && damage::same_commands(&self.key, &plan.key)
    }
    pub fn bytes(&self) -> usize {
        self.target
            .as_ref()
            .map_or(0, |t| t.size.width as usize * t.size.height as usize * 4)
    }
}

pub struct Target {
    pub texture: GLuint,
    pub framebuffer: GLuint,
    pub size: Size,
}
impl Target {
    pub fn new(size: Size) -> Result<Self, String> {
        if !gl::GenFramebuffers::is_loaded()
            || !gl::BindFramebuffer::is_loaded()
            || !gl::FramebufferTexture2D::is_loaded()
            || !gl::CheckFramebufferStatus::is_loaded()
            || !gl::DeleteFramebuffers::is_loaded()
        {
            return Err("Framebuffer API unavailable".into());
        }
        let mut texture = 0;
        let mut framebuffer = 0;
        let mut previous_fbo = 0;
        let mut previous_active = 0;
        let mut previous_texture = 0;
        let mut status = 0;
        // SAFETY: the renderer owns a current GL context; dimensions are capped before entry.
        unsafe {
            gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut previous_fbo);
            gl::GetIntegerv(gl::ACTIVE_TEXTURE, &mut previous_active);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut previous_texture);
            gl::GenTextures(1, &mut texture);
            if texture != 0 {
                gl::BindTexture(gl::TEXTURE_2D, texture);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    size.width as i32,
                    size.height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                gl::GenFramebuffers(1, &mut framebuffer);
                if framebuffer != 0 {
                    gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
                    gl::FramebufferTexture2D(
                        gl::FRAMEBUFFER,
                        gl::COLOR_ATTACHMENT0,
                        gl::TEXTURE_2D,
                        texture,
                        0,
                    );
                    status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
                }
            }
            gl::BindFramebuffer(gl::FRAMEBUFFER, previous_fbo as u32);
            gl::BindTexture(gl::TEXTURE_2D, previous_texture as u32);
            gl::ActiveTexture(previous_active as u32);
        }
        let target = Self {
            texture,
            framebuffer,
            size,
        };
        if texture == 0 || framebuffer == 0 || status != gl::FRAMEBUFFER_COMPLETE {
            return Err("Optical backdrop unavailable".into());
        }
        Ok(target)
    }
}
impl Drop for Target {
    fn drop(&mut self) {
        // SAFETY: cache targets are dropped with the renderer while its GL context is current.
        unsafe {
            gl::DeleteFramebuffers(1, &self.framebuffer);
            gl::DeleteTextures(1, &self.texture);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_resolution_stays_hd_in_4k_and_fits_small_drivers() {
        assert_eq!(size(Size::UHD, 4096), Size::HD);
        assert_eq!(size(Size::HD, 1024), Size::new(1024, 576));
    }
    #[test]
    fn changing_a_foreground_label_does_not_invalidate_its_base_backdrop() {
        use crate::renderer::TextStyle;
        let base = Draw::Shape(Rect::new(0., 0., 1920., 1080.), 0., 0., [0.1, 0.2, 0.3, 1.]);
        let glass = Draw::Glass(
            Rect::new(500., 50., 400., 60.),
            30.,
            [0.2; 4],
            [0.3; 4],
            Effect::default(),
        );
        let label = |s: &str| {
            Draw::Text(
                s.into(),
                Rect::new(520., 65., 200., 30.),
                TextStyle::new(20.),
                [1.; 4],
                false,
            )
        };
        let a = plan(&[base.clone(), glass.clone(), label("Home")], Layer::Base).unwrap();
        let b = plan(&[base, glass, label("Settings")], Layer::Base).unwrap();
        assert!(damage::same_commands(&a.key, &b.key));
        assert_eq!(a.end, 1);
    }
}

#[cfg(test)]
mod invalidation_tests {
    use super::*;
    fn glass(rect: Rect) -> Draw {
        Draw::Glass(rect, 20., [0.2; 4], [0.3; 4], Effect::default())
    }
    #[test]
    fn content_in_the_gap_between_lenses_is_not_a_dependency() {
        let prefix = |x| Draw::Shape(Rect::new(x, 400., 100., 60.), 8., 0., [1.; 4]);
        let a = plan(
            &[
                prefix(600.),
                glass(Rect::new(500., 40., 400., 60.)),
                glass(Rect::new(500., 800., 400., 80.)),
            ],
            Layer::Base,
        )
        .unwrap();
        let b = plan(
            &[
                prefix(700.),
                glass(Rect::new(500., 40., 400., 60.)),
                glass(Rect::new(500., 800., 400., 80.)),
            ],
            Layer::Base,
        )
        .unwrap();
        assert!(damage::same_commands(&a.key, &b.key));
    }
    #[test]
    fn opaque_backplate_is_captured_before_its_labels_and_controls() {
        let lower = Draw::Shape(Rect::new(1100., 100., 700., 800.), 40., 0., [0.1; 4]);
        let optical = Draw::Glass(
            Rect::new(1200., 200., 200., 90.),
            30.,
            [0.2; 4],
            [0.3; 4],
            Effect {
                opacity: 1.,
                backdrop: Some(Layer::Panel),
            },
        );
        let p = plan(
            &[lower, Draw::Backdrop(Layer::Panel), optical],
            Layer::Panel,
        )
        .unwrap();
        assert_eq!(p.end, 1);
        assert_eq!(p.key.len(), 1);
    }
}
