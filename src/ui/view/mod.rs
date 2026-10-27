//! Screen geometry and styling mirror the web UI's 1920×1080 / 16px-rem layout.
use crate::{
    i18n::Language,
    model::{App, BACKGROUNDS, CATEGORIES, Input, InputMode},
    renderer::{Color, Draw, TextStyle, WHITE},
    state::{
        Control, HOME_FADE_LEFT, HOME_FADE_RIGHT, KEYS, Overlay, Page, Rect, SYSTEM_SECTION, State,
        Subpage, Target, TextField, WEATHER_SECTION,
    },
    weather::{self, Unit},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

const MAIN_TOP: f32 = 108.8;
const FACE_HEIGHT: f32 = 154.4;
const RECENTS_BACKDROP_OPACITY: f32 = 0.86;

mod control_center;
mod profile;
mod recents_view;
mod slider;

pub struct Assets {
    pub root: PathBuf,
    pub version: String,
    pub metrics: BTreeMap<String, BTreeMap<char, f32>>,
    pub artwork: crate::app_artwork::Service,
}
impl Assets {
    pub fn load(root: PathBuf, webos: bool) -> Result<Self, String> {
        let load = |file: &str| {
            std::fs::read(root.join(file))
                .map_err(|e| format!("Native assets are missing: {e}. Run pnpm assets."))
        };
        let version = String::from_utf8(load("version.txt")?).map_err(|e| e.to_string())?;
        let metrics =
            serde_json::from_slice(&load("font-metrics.json")?).map_err(|e| e.to_string())?;
        Ok(Self {
            root,
            version,
            metrics,
            artwork: crate::app_artwork::Service::new(webos),
        })
    }
    fn path(&self, path: impl AsRef<Path>) -> String {
        self.root.join(path).to_string_lossy().into_owned()
    }

    fn width(&self, value: &str, style: TextStyle) -> f32 {
        let key = format!(
            "{}{}",
            if style.size >= 32. { "display-" } else { "" },
            style.weight
        );
        let advances = self.metrics.get(&key);
        let fallback = self.metrics.get(if style.weight >= 500 {
            "fallback-500"
        } else {
            "fallback-400"
        });
        value
            .chars()
            .map(|c| {
                (advances
                    .and_then(|a| a.get(&c))
                    .or_else(|| fallback.and_then(|a| a.get(&c)))
                    .copied()
                    .unwrap_or(if c == ' ' { 0.28 } else { 0.53 })
                    + style.tracking)
                    * style.size
            })
            .sum()
    }

    fn fit_line(&self, value: &str, mut style: TextStyle, width: f32) -> TextStyle {
        // Shrinking below 32 px selects the body metrics instead of the display
        // font. Remeasure after that boundary rather than applying one ratio.
        for _ in 0..3 {
            let measured = self.width(value, style);
            if measured <= width {
                break;
            }
            style.size *= width / measured;
        }
        style
    }

    fn lines(&self, value: &str, style: TextStyle, width: f32) -> usize {
        let key = format!(
            "{}{}",
            if style.size >= 32. { "display-" } else { "" },
            style.weight
        );
        let advances = self.metrics.get(&key);
        let fallback = self.metrics.get(if style.weight >= 500 {
            "fallback-500"
        } else {
            "fallback-400"
        });
        let advance = |c: char| {
            (advances
                .and_then(|a| a.get(&c))
                .or_else(|| fallback.and_then(|a| a.get(&c)))
                .copied()
                .unwrap_or(if c == ' ' { 0.28 } else { 0.53 })
                + style.tracking)
                * style.size
        };
        let mut lines = 1;
        let mut used = 0.;
        for word in value.split_inclusive(char::is_whitespace) {
            let w: f32 = word.chars().map(advance).sum();
            if used > 0. && used + w > width {
                lines += 1;
                used = 0.;
            }
            for c in word.chars() {
                if c == '\n' {
                    lines += 1;
                    used = 0.;
                    continue;
                }
                let w = advance(c);
                if used > 0. && used + w > width {
                    lines += 1;
                    used = 0.;
                }
                used += w;
            }
        }
        lines
    }
}

pub struct Scene {
    pub draw: Vec<Draw>,
    pub controls: Vec<Control>,
    pub modal: Option<Rect>,
    accent: crate::features::appearance::Accent,
    liquid_glass: bool,
    modal_chrome: bool,
    glass_layer: Option<crate::render::backdrop::Layer>,
}
impl Scene {
    fn new(accent: crate::features::appearance::Accent, liquid_glass: bool) -> Self {
        Self {
            draw: Vec::new(),
            controls: Vec::new(),
            modal: None,
            accent,
            liquid_glass,
            modal_chrome: false,
            glass_layer: Some(crate::render::backdrop::Layer::Base),
        }
    }
    fn shape(&mut self, rect: Rect, radius: f32, fill: Color) {
        self.draw.push(Draw::Shape(rect, radius, 0., fill));
    }
    fn border(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        self.draw.push(Draw::Shape(rect, radius, width, color));
    }
    fn text(
        &mut self,
        text: impl Into<String>,
        rect: Rect,
        style: TextStyle,
        color: Color,
        center: bool,
    ) {
        self.draw
            .push(Draw::Text(text.into(), rect, style, color, center));
    }
    fn image(&mut self, path: String, rect: Rect, radius: f32, contain: bool) {
        self.draw.push(Draw::Image(path, rect, radius, contain));
    }
    fn icon(&mut self, assets: &Assets, name: &str, rect: Rect, color: Color) {
        self.draw.push(Draw::TintedImage(
            assets.path(format!("ui-icons/{name}.lvec")),
            rect,
            0.,
            true,
            color,
        ));
    }
    fn control(&mut self, target: Target, rect: Rect, scrollable: bool) {
        self.controls.push(Control {
            target,
            rect,
            scrollable,
        });
    }
    fn ring(&mut self, rect: Rect, radius: f32) {
        self.border(
            rect.inset(-6.9),
            radius + 6.9,
            3.7,
            self.accent_color(c(0xffffff, 0.10)),
        );
        self.border(
            rect.inset(-3.2),
            radius + 3.2,
            3.2,
            self.accent_color(c(0xf0f0f0, 1.)),
        );
    }
    fn accent_color(&self, neutral: Color) -> Color {
        if self.accent == crate::features::appearance::Accent::Neutral {
            neutral
        } else {
            c(self.accent.rgb(), neutral[3])
        }
    }
    fn glass(&mut self, rect: Rect, radius: f32, top: Color, bottom: Color) {
        self.draw.push(Draw::Glass(
            rect,
            radius,
            top,
            bottom,
            crate::render::backdrop::Effect {
                opacity: 1.,
                backdrop: self.glass_layer,
            },
        ));
    }
    fn selection_surface(&mut self, rect: Rect, radius: f32) {
        let opacity = if self.liquid_glass { 0.90 } else { 1. };
        let top = self.accent_color(c(0xfafafa, opacity));
        let bottom = if self.accent == crate::features::appearance::Accent::Neutral {
            c(0xe5e5e5, if self.liquid_glass { 0.86 } else { 1. })
        } else {
            [
                top[0] * 0.92,
                top[1] * 0.92,
                top[2] * 0.92,
                if self.liquid_glass { 0.86 } else { 1. },
            ]
        };
        if self.liquid_glass {
            self.glass(rect, radius, top, bottom);
        } else {
            self.shape(rect, radius, top);
        }
    }
    fn shadow(&mut self, rect: Rect, radius: f32, blur: f32, y: f32, alpha: f32) {
        self.draw.push(Draw::Shadow(
            Rect::new(rect.x, rect.y + y, rect.w, rect.h),
            radius,
            blur,
            c(0, alpha),
        ));
    }
    /// Shared control material. Glass is a single paint quad, including its rim.
    fn material(&mut self, rect: Rect, radius: f32, focused: bool, raised: bool) {
        if focused {
            self.selection_surface(rect, radius);
        } else if self.liquid_glass {
            let (top, bottom) = if raised {
                (c(0x34444f, 0.56), c(0x16212b, 0.68))
            } else {
                (c(0x414d57, 0.22), c(0x1b232c, 0.34))
            };
            self.glass(rect, radius, top, bottom);
        } else {
            self.shape(
                rect,
                radius,
                if raised {
                    c(0x20262d, 1.)
                } else {
                    c(0x171c20, 1.)
                },
            );
            self.border(rect, radius, 1., c(0xffffff, 0.08));
        }
    }
    /// Clear chrome keeps white content over a locally tinted lens. Selection,
    /// hover/remote focus and press have distinct borders, tint and depth.
    fn chrome(
        &mut self,
        rect: Rect,
        radius: f32,
        focused: bool,
        selected: bool,
        pressed: bool,
    ) -> Color {
        if self.liquid_glass {
            self.shadow(
                rect,
                radius,
                if focused { 18. } else { 10. },
                if pressed { 1. } else { 4. },
                if focused { 0.28 } else { 0.12 },
            );
            let (top, bottom) = if self.modal_chrome {
                if pressed {
                    (c(0x182b3e, 0.54), c(0x0c1a2b, 0.64))
                } else if focused {
                    (self.accent_color(c(0x9fbcd8, 0.40)), c(0x344e69, 0.52))
                } else {
                    (c(0x3e5369, 0.34), c(0x152839, 0.50))
                }
            } else if pressed {
                (c(0x142536, 0.32), c(0x101d2b, 0.38))
            } else if focused {
                (self.accent_color(c(0xbad4eb, 0.24)), c(0x586d88, 0.24))
            } else if selected {
                (c(0xb2c6da, 0.18), c(0x52677e, 0.20))
            } else {
                (c(0x899cb3, 0.10), c(0x24374a, 0.18))
            };
            self.glass(rect, radius, top, bottom);
            self.border(
                rect,
                radius,
                if focused { 2. } else { 1. },
                if focused {
                    self.accent_color(c(0xffffff, if pressed { 0.65 } else { 0.95 }))
                } else {
                    c(0xe4efff, if selected { 0.46 } else { 0.22 })
                },
            );
            if focused && !pressed {
                self.border(rect.inset(4.), (radius - 4.).max(0.), 1., c(0xffffff, 0.18));
            }
            WHITE
        } else {
            self.material(rect, radius, focused || selected, false);
            if pressed {
                self.shape(rect, radius, c(0, 0.16));
            }
            if focused || selected {
                c(0x20242a, 1.)
            } else {
                WHITE
            }
        }
    }
    /// Reuse the existing modal backdrop target. Inner rows composite analytically
    /// on the panel, avoiding recursive refraction of the same foreground labels.
    fn modal_panel(&mut self, rect: Rect, radius: f32) {
        self.modal = Some(rect);
        self.draw
            .push(Draw::Backdrop(crate::render::backdrop::Layer::Panel));
        self.glass_layer = Some(crate::render::backdrop::Layer::Panel);
        self.chrome(rect, radius, false, false, false);
        self.glass_layer = None;
    }
    fn pressed_feedback(&mut self, state: &State, target: &Target, rect: Rect, radius: f32) {
        if state.pressed.as_ref() == Some(target) {
            self.shape(rect, radius, c(0, 0.16));
            self.border(rect.inset(3.), (radius - 3.).max(0.), 1., c(0xffffff, 0.30));
        }
    }
    fn pill(&mut self, rect: Rect, focused: bool, selected: bool) {
        if self.liquid_glass {
            self.shadow(rect, rect.h / 2., 12., 3., 0.08);
        }
        self.material(rect, rect.h / 2., focused, selected);
    }
    fn fade(&mut self, from: usize, alpha: f32, brightness: f32) {
        for draw in &mut self.draw[from..] {
            let apply = |color: &mut Color| {
                for channel in &mut color[..3] {
                    *channel *= brightness;
                }
                color[3] *= alpha;
            };
            match draw {
                Draw::Shape(_, _, _, color)
                | Draw::Shadow(_, _, _, color)
                | Draw::Text(_, _, _, color, _)
                | Draw::TintedImage(_, _, _, _, color)
                | Draw::AppImage(_, _, color) => apply(color),
                Draw::RoundedGradient(_, _, a, b) => {
                    apply(a);
                    apply(b);
                }
                Draw::Glass(_, _, a, b, effect) => {
                    for color in [a, b] {
                        for channel in &mut color[..3] {
                            *channel *= brightness;
                        }
                    }
                    effect.opacity *= alpha;
                }
                Draw::Image(path, rect, radius, contain) => {
                    *draw = Draw::TintedImage(
                        path.clone(),
                        *rect,
                        *radius,
                        *contain,
                        [brightness, brightness, brightness, alpha],
                    );
                }
                Draw::Clip(_) | Draw::EdgeFade(_) | Draw::EdgeFadeX(_) | Draw::Backdrop(_) => {}
            }
        }
    }
}

fn c(rgb: u32, alpha: f32) -> Color {
    [
        ((rgb >> 16) & 255) as f32 / 255.,
        ((rgb >> 8) & 255) as f32 / 255.,
        (rgb & 255) as f32 / 255.,
        alpha,
    ]
}
fn text(size: f32, weight: u16, tracking: f32) -> TextStyle {
    TextStyle::new(size).weight(weight).tracking(tracking)
}

pub struct Navigation {
    pub surface: Rect,
    pub items: [Rect; 3],
}
pub fn navigation(assets: &Assets, language: Language) -> Navigation {
    let widths = ["home", "apps", "settings"]
        .map(|key| (assets.width(language.t(key), text(20.8, 550, -0.02)) + 46.4).max(99.2));
    let width = widths.iter().sum::<f32>() + 25.6 + 19.6;
    let mut x = (1920. - width) / 2. + 9.8;
    let items = widths.map(|width| {
        let rect = Rect::new(x, 52.3, width, 49.);
        x += width + 12.8;
        rect
    });
    Navigation {
        surface: Rect::new((1920. - width) / 2., 42.5, width, 68.6),
        items,
    }
}

/// Fully shown Recents is drawn instead of the page, which is then not part of the scene.
pub fn recents_replaces_page(state: &State) -> bool {
    state.recents_open() && state.recents.visibility == 1.
}

pub fn scene_with_pill(
    state: &State,
    assets: &Assets,
    pill_x: f32,
    pill_width: f32,
    now: OffsetDateTime,
) -> Scene {
    let mut out = Scene::new(
        state.preferences.appearance.accent,
        state.preferences.appearance.liquid_glass,
    );
    let blur_suffix = wallpaper_blur(state);
    if let Some((previous, start)) = &state.previous_background {
        paint_wallpaper(&mut out, state, assets, previous, blur_suffix, 1.);
        paint_wallpaper(
            &mut out,
            state,
            assets,
            state.current_background(),
            blur_suffix,
            (start.elapsed().as_secs_f32() / 0.4).min(1.),
        );
    } else {
        paint_wallpaper(
            &mut out,
            state,
            assets,
            state.current_background(),
            blur_suffix,
            1.,
        );
    }
    let dimming = state.preferences.appearance.dimming.alpha();
    if dimming > 0. {
        out.shape(Rect::new(0., 0., 1920., 1080.), 0., c(0, dimming));
    }
    if state.screensaver.active {
        if state.preferences.screensaver.photos && !state.wallpapers.photos.is_empty() {
            let photo = &state.wallpapers.photos
                [(state.screensaver.elapsed() / 30) as usize % state.wallpapers.photos.len()];
            paint_wallpaper(&mut out, state, assets, &photo.id, "", 1.);
            out.shape(Rect::new(0., 0., 1920., 1080.), 0., c(0, 0.72));
        } else {
            out.shape(Rect::new(0., 0., 1920., 1080.), 0., c(0, 1.));
        }
        let step = state.screensaver.elapsed() / 15;
        let x = 120. + ((step * 317) % 1240) as f32;
        let y = 120. + ((step * 193) % 650) as f32;
        out.text(
            state
                .preferences
                .language
                .clock(now, state.preferences.hour_24, false),
            Rect::new(x, y, 440., 130.),
            text(76., 400, 0.),
            c(0xcccccc, 0.7),
            true,
        );
        out.controls.clear();
        return out;
    }
    let content_start = out.draw.len();
    if recents_replaces_page(state) {
        out.shape(
            Rect::new(0., 0., 1920., 1080.),
            0.,
            c(0x080b12, RECENTS_BACKDROP_OPACITY),
        );
        recents_view::render(&mut out, state, assets);
        if let Some((notice, _)) = &state.notice {
            toast(&mut out, state, assets, notice);
        }
        return out;
    }
    let viewport = state.content_viewport();
    let main_bottom = viewport.y + viewport.h;
    out.draw.push(Draw::Clip(Some(viewport)));
    if state.page == Page::Apps {
        out.draw.push(Draw::EdgeFade(Some([
            MAIN_TOP,
            152.8,
            main_bottom - 24.,
            main_bottom,
        ])));
    }
    match state.page {
        Page::Home => home(&mut out, state, assets, now),
        Page::Apps => library(&mut out, state, assets),
        Page::Settings => settings(&mut out, state, assets),
    }
    if let Some(error) = &state.error
        && !state.inventory.apps.is_empty()
        && !out
            .controls
            .iter()
            .any(|control| control.target == Target::Retry)
    {
        let rect = Rect::new(84., main_bottom - 66., 1752., 54.);
        out.shape(rect, 11.2, c(0xad7c3e, 0.07));
        out.border(rect, 11.2, 1., c(0xedc596, 0.19));
        out.icon(
            assets,
            "alert",
            Rect::new(104., rect.y + 18., 18., 18.),
            c(0xf3ddbd, 1.),
        );
        out.text(
            error,
            Rect::new(138., rect.y + 16., 1500., 35.),
            text(14.4, 400, 0.),
            c(0xf3ddbd, 1.),
            false,
        );
        let retry = Rect::new(1720., rect.y + 8., 96., 38.);
        out.text(
            state.preferences.language.t("retry"),
            Rect::new(retry.x, retry.y + 10., retry.w, 20.),
            text(14.4, 500, 0.),
            c(0xf3ddbd, 1.),
            true,
        );
        if state.focus == Target::Retry {
            out.border(retry, 6.4, 2., out.accent_color(WHITE));
        }
        out.control(Target::Retry, retry, false);
    }
    out.draw.push(Draw::EdgeFade(None));
    out.draw.push(Draw::Clip(None));
    header(&mut out, state, assets, now, pill_x, pill_width);
    if !matches!(state.overlay, Some(Overlay::ControlCenter { .. })) {
        footer(&mut out, state, assets);
    }
    if let Some(
        overlay @ (Overlay::Options { .. }
        | Overlay::Keyboard
        | Overlay::RemovalConfirm { .. }
        | Overlay::RemovalStatus { .. }
        | Overlay::ControlConfirm { .. }
        | Overlay::ControlCenter { .. }
        | Overlay::Profile),
    ) = &state.overlay
    {
        out.controls.clear();
        out.glass_layer = None;
        let floating = matches!(
            overlay,
            Overlay::ControlCenter { .. } | Overlay::Profile | Overlay::Options { .. }
        );
        if floating {
            // Fade the actual parent before capturing it: sharp Settings labels
            // must not compete with the modal's labels through clear lenses.
            out.fade(content_start, 0.12, 0.65);
        }
        out.modal_chrome = floating;
        let start = out.draw.len();
        out.shape(
            Rect::new(0., 0., 1920., 1080.),
            0.,
            c(
                0x080b12,
                if floating {
                    if state.preferences.appearance.liquid_glass {
                        0.38
                    } else {
                        0.50
                    }
                } else {
                    0.70
                },
            ),
        );
        match overlay {
            Overlay::Options { id, removal, .. } => {
                options(&mut out, state, assets, id, removal.as_ref())
            }
            Overlay::RemovalConfirm { target, title, .. } => {
                removal_dialog(&mut out, state, &target.id, title, true)
            }
            Overlay::RemovalStatus { .. } => {
                if let Some(target) = &state.management.target {
                    removal_dialog(&mut out, state, &target.id, &state.management.title, false);
                }
            }
            Overlay::Profile => profile::render(&mut out, state, assets),
            Overlay::ControlConfirm { command, .. } => control_dialog(&mut out, state, *command),
            Overlay::ControlCenter { .. } => {
                control_center::render(&mut out, state, assets);
                out.modal_chrome = false;
                out.glass_layer = Some(crate::render::backdrop::Layer::Base);
                out.draw
                    .push(Draw::Backdrop(crate::render::backdrop::Layer::Base));
                header_identity(&mut out, state, assets, now);
                control_center_button(&mut out, state, assets);
            }
            Overlay::Keyboard => keyboard(&mut out, state, assets),
            Overlay::SystemKeyboard | Overlay::Recents { .. } => {}
        }
        if let Some(opened) = state.panel_started {
            out.fade(start, (opened.elapsed().as_secs_f32() / 0.12).min(1.), 1.);
        }
        out.modal_chrome = false;
    }
    if state.recents_open() || state.recents.visibility > 0. {
        // Crossfade the actual parent interface as well as the dimmed stage.
        // Retain outgoing visuals after dismissal, while input immediately
        // belongs to the restored parent. No geometry or hit targets move.
        let parent_controls = std::mem::take(&mut out.controls);
        let parent_modal = out.modal;
        out.fade(content_start, 1. - state.recents.visibility, 1.);
        let start = out.draw.len();
        out.shape(
            Rect::new(0., 0., 1920., 1080.),
            0.,
            c(0x080b12, RECENTS_BACKDROP_OPACITY),
        );
        recents_view::render(&mut out, state, assets);
        out.fade(start, state.recents.visibility, 1.);
        // Dim the wallpaper beneath the outgoing interface rather than over it. While both
        // fade, the backdrop and Recents' full-screen gradient then lie directly on the
        // wallpaper, and the renderer folds them into it instead of two more full-screen
        // passes per frame of the crossfade.
        let fills = out.draw[start..]
            .iter()
            .take_while(|draw| full_screen_fill(draw))
            .count();
        let moved: Vec<_> = out.draw.drain(start..start + fills).collect();
        out.draw.splice(content_start..content_start, moved);
        if !state.recents_open() {
            out.controls = parent_controls;
            out.modal = parent_modal;
        }
    }
    if let Some((notice, _)) = &state.notice {
        toast(&mut out, state, assets, notice);
    }
    out
}

pub(crate) fn wallpaper_blur(state: &State) -> &'static str {
    // Clear glass keeps the chosen wallpaper sharp unless the user requests blur.
    state.preferences.appearance.blur.suffix()
}

fn header_identity(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    use crate::features::header::{Avatar, Side};
    let clock = state.preferences.language.clock(
        now,
        state.preferences.hour_24,
        state.preferences.show_seconds,
    );
    let right = state.preferences.header.side == Side::Right;
    let focused = state.focus == Target::Profile;
    let clock_width = assets.width(&clock, text(20., 500, -0.03)).ceil().min(260.);
    let width = clock_width + 84.;
    let x = if right { 1694. - width } else { 84. };
    let avatar = if state.preferences.header.combined {
        let capsule = Rect::new(x, 52., width, 50.);
        let color = out.chrome(
            capsule,
            25.,
            focused,
            false,
            state.pressed.as_ref() == Some(&Target::Profile),
        );
        out.text(
            &clock,
            Rect::new(x + 20., 65., clock_width, 25.),
            text(20., 500, -0.03),
            color,
            false,
        );
        Rect::new(x + width - 46., 57., 40., 40.)
    } else {
        out.text(
            &clock,
            Rect::new(
                if right {
                    1694. - clock_width - 68.
                } else {
                    84.
                },
                65.,
                clock_width,
                25.,
            ),
            text(20., 500, -0.03),
            WHITE,
            false,
        );
        let avatar = Rect::new(1644., 52., 50., 50.);
        out.chrome(
            avatar,
            25.,
            focused,
            false,
            state.pressed.as_ref() == Some(&Target::Profile),
        );
        avatar.inset(5.)
    };
    out.shape(avatar, avatar.w / 2., out.accent_color(c(0xc9b6ff, 0.95)));
    let initials = crate::features::header::initials(&state.preferences.greeting_name);
    if state.preferences.header.avatar == Avatar::Initials && !initials.is_empty() {
        out.text(
            initials,
            Rect::new(avatar.x, avatar.y + 10., avatar.w, 24.),
            text(17., 600, 0.),
            c(0x20242a, 1.),
            true,
        );
    } else {
        out.icon(assets, "user", avatar.inset(9.), c(0x20242a, 1.));
    }
    // Profile owns only its own avatar hit area; it never opens quick controls.
    if state.overlay.is_none() {
        out.control(Target::Profile, avatar.inset(-5.), false);
    }
}
fn control_center_button(out: &mut Scene, state: &State, assets: &Assets) {
    let rect = Rect::new(1712., 52., 50., 50.);
    let focused = state.focus == Target::ControlCenter;
    let open = matches!(state.overlay, Some(Overlay::ControlCenter { .. }));
    let color = out.chrome(
        rect,
        25.,
        focused,
        open,
        state.pressed.as_ref() == Some(&Target::ControlCenter),
    );
    out.icon(assets, "control-center", rect.inset(13.), color);
    out.control(Target::ControlCenter, rect, false);
}

fn header(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    now: OffsetDateTime,
    pill_x: f32,
    pill_width: f32,
) {
    let language = state.preferences.language;
    header_identity(out, state, assets, now);
    let layout = navigation(assets, language);
    let nav = layout.surface;
    out.chrome(nav, nav.h / 2., false, false, false);
    let pill = Rect::new(pill_x, 52.3, pill_width, 49.);
    let selected_target = Target::Nav(state.page_index());
    let selected_color = out.chrome(
        pill,
        pill.h / 2.,
        state.focus == selected_target,
        true,
        state.pressed.as_ref() == Some(&selected_target),
    );
    if let Target::Nav(index) = state.focus
        && index != state.page_index()
        && let Some(rect) = layout.items.get(index)
    {
        out.chrome(
            *rect,
            rect.h / 2.,
            true,
            false,
            state.pressed.as_ref() == Some(&state.focus),
        );
    }
    if matches!(state.focus, Target::Nav(_)) && !state.pointer {
        // Follow the pill's animated position and width instead of jumping to
        // the destination tab's control rectangle.
        out.border(
            pill.inset(-5.),
            37.,
            2.,
            out.accent_color(c(0xf0f0f0, 0.95)),
        );
    }
    for (index, key) in ["home", "apps", "settings"].into_iter().enumerate() {
        let rect = layout.items[index];
        out.text(
            language.t(key),
            Rect::new(rect.x, 64.3, rect.w, 25.),
            text(20.8, 550, -0.02),
            if state.page_index() == index {
                selected_color
            } else if state.focus == Target::Nav(index) {
                if out.liquid_glass {
                    WHITE
                } else {
                    c(0x20242a, 1.)
                }
            } else {
                c(0xffffff, 0.86)
            },
            true,
        );
        out.control(Target::Nav(index), rect, false);
    }
    control_center_button(out, state, assets);
    let rect = Rect::new(1786.4, 52., 49.6, 49.6);
    let focused = state.focus == Target::TvSettings;
    let color = out.chrome(
        rect,
        rect.h / 2.,
        focused,
        false,
        state.pressed.as_ref() == Some(&Target::TvSettings),
    );
    out.icon(
        assets,
        "settings",
        Rect::new(1799.6, 65.2, 23.2, 23.2),
        color,
    );
    out.control(Target::TvSettings, rect, false);
}

fn tile(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    app: &App,
    rect: Rect,
    names: bool,
    pinned: bool,
) {
    let caption_height = if names {
        caption_height(state, assets, app, rect.w)
    } else {
        0.
    };
    let label_height = if names { 18.4 + caption_height } else { 0. };
    out.control(
        Target::App(app.id.clone()),
        Rect::new(rect.x, rect.y, rect.w, rect.h + label_height),
        true,
    );
    let viewport = if state.page == Page::Home {
        state.favorites_viewport()
    } else {
        state.content_viewport()
    };
    if rect.y + rect.h + label_height < MAIN_TOP
        || rect.y > 1054.
        || rect.x + rect.w + 40. < viewport.x
        || rect.x - 40. > viewport.x + viewport.w
    {
        return;
    }
    let start = out.draw.len();
    let focused = state.focus == Target::App(app.id.clone());
    let pressed = state.pressed.as_ref() == Some(&Target::App(app.id.clone()));
    let rect = if focused && !pressed && !state.preferences.reduced_motion {
        rect.inset(-5.)
    } else {
        rect
    };
    out.shadow(
        rect,
        19.2,
        if focused { 34. } else { 22. },
        if focused { 14. } else { 9. },
        if focused { 0.4 } else { 0.165 },
    );
    app_face(out, assets, app, rect, state.preferences.language, 19.2);
    if pinned {
        out.icon(
            assets,
            "heart-filled",
            Rect::new(rect.x + rect.w - 27.2, rect.y + 11.2, 16., 16.),
            c(0xffffff, 0.9),
        );
    }
    if state.moving.as_ref() == Some(&app.id) {
        let indicator = Rect::new(
            rect.x + rect.w / 2. - 37.6,
            rect.y + rect.h - 33.6,
            75.2,
            24.,
        );
        out.shape(indicator, 16., c(0x101318, 0.79));
        out.icon(
            assets,
            "left",
            Rect::new(indicator.x + 8.8, indicator.y + 4., 16., 16.),
            c(0xededed, 1.),
        );
        out.icon(
            assets,
            "arrow",
            Rect::new(indicator.x + 40., indicator.y + 4., 16., 16.),
            c(0xededed, 1.),
        );
    }
    if focused {
        out.ring(rect, 19.2);
    }
    if state.page == Page::Apps && state.filter == 5 {
        out.fade(start, 0.45, 1.);
    }
    if state.pressed.as_ref() == Some(&Target::App(app.id.clone())) {
        out.fade(start, 1., 0.85);
    }
    if names {
        out.text(
            format!(
                "{}{}",
                app.title,
                if state.filter == 5 {
                    format!(" · {}", state.preferences.language.t("hiddenLabel"))
                } else {
                    String::new()
                }
            ),
            Rect::new(
                rect.x + 4.,
                rect.y + rect.h + 18.4,
                rect.w - 8.,
                caption_height,
            ),
            text(17.6, if focused { 550 } else { 450 }, -0.02).leading(1.4),
            if focused {
                c(0xffffff, 1.)
            } else {
                c(0xececec, 0.66)
            },
            true,
        );
    }
    if state.moving.is_some() && state.moving.as_ref() != Some(&app.id) {
        out.fade(start, 0.48, 1.);
    }
}

/// Shared card artwork for Home, Apps and Recents. Curated vectors always take
/// precedence over installed artwork, with the same relative logo proportions.
fn app_face(
    out: &mut Scene,
    assets: &Assets,
    app: &App,
    rect: Rect,
    language: Language,
    radius: f32,
) {
    let logo = crate::branding::for_key(&app.brand);
    let artwork = logo
        .is_none()
        .then(|| {
            app.icon
                .as_ref()
                .and_then(|source| assets.artwork.get(source))
        })
        .flatten();
    if let Some([top, bottom]) = artwork.as_ref().map(|image| image.backdrop) {
        let color = |rgb: [u8; 3]| rgb.map(|c| c as f32 / 255.);
        let [top, bottom] = [color(top), color(bottom)];
        if top == bottom {
            out.shape(rect, radius, [top[0], top[1], top[2], 1.]);
        } else {
            out.draw.push(Draw::RoundedGradient(
                rect,
                radius,
                [top[0], top[1], top[2], 1.],
                [bottom[0], bottom[1], bottom[2], 1.],
            ));
        }
    } else {
        out.image(
            assets.path(format!("surfaces/{}.png", app.brand)),
            rect,
            radius,
            false,
        );
    }
    out.border(rect, radius, 1., c(0xffffff, 0.063));
    if let Some(brand) = logo {
        let (ratio, height) = brand.artwork_size();
        let height = height * rect.h / FACE_HEIGHT;
        out.image(
            assets.path(format!("logos/{}.lvec", app.brand)),
            Rect::new(
                rect.x + rect.w * (1. - ratio) / 2.,
                rect.y + (rect.h - height) / 2.,
                rect.w * ratio,
                height,
            ),
            0.,
            true,
        );
    } else if let Some(image) = artwork {
        out.draw.push(Draw::AppImage(
            image,
            Rect::new(
                rect.x + rect.w * 0.16,
                rect.y + rect.h * 0.16,
                rect.w * 0.68,
                rect.h * 0.68,
            ),
            [1.; 4],
        ));
    } else {
        brand_mark(out, assets, app, rect, language);
    }
}

fn caption_height(state: &State, assets: &Assets, app: &App, width: f32) -> f32 {
    let title = format!(
        "{}{}",
        app.title,
        if state.filter == 5 {
            format!(" · {}", state.preferences.language.t("hiddenLabel"))
        } else {
            String::new()
        }
    );
    let style = text(
        17.6,
        if state.focus == Target::App(app.id.clone()) {
            550
        } else {
            450
        },
        -0.02,
    )
    .leading(1.4);
    (assets.lines(&title, style, width - 8.) as f32 * style.size * style.line_height).max(49.28)
}

fn brand_mark(out: &mut Scene, assets: &Assets, app: &App, rect: Rect, language: Language) {
    let scale = rect.h / FACE_HEIGHT;
    let rect = Rect::new(
        rect.x / scale,
        rect.y / scale,
        rect.w / scale,
        rect.h / scale,
    );
    let start = out.draw.len();
    let (cx, cy) = rect.center();
    match app.brand.as_str() {
        "plex" => {
            out.text(
                "ple",
                Rect::new(cx - 63., cy - 35.2, 130., 70.4),
                text(70.4, 650, -0.08).leading(1.),
                WHITE,
                false,
            );
            out.text(
                "x",
                Rect::new(cx + 20., cy - 35.2, 60., 70.4),
                text(70.4, 800, -0.08).leading(1.),
                c(0xe6aa25, 1.),
                false,
            );
        }
        "spotify" => {
            out.icon(
                assets,
                "spotify",
                Rect::new(cx - 77., cy - 20., 40., 40.),
                c(0x5ae08c, 1.),
            );
            out.text(
                "Spotify",
                Rect::new(cx - 29., cy - 14.4, 120., 36.),
                text(28.8, 650, -0.04).leading(1.),
                c(0x5ae08c, 1.),
                false,
            );
        }
        "kodi" => {
            out.icon(
                assets,
                "kodi",
                Rect::new(cx - 76., cy - 22.5, 45., 45.),
                WHITE,
            );
            out.text(
                "kodi",
                Rect::new(cx - 18., cy - 24.8, 130., 60.),
                text(49.6, 350, -0.04).leading(1.),
                WHITE,
                false,
            );
        }
        brand => {
            let (icon, label) = match brand {
                "live-tv" => ("tv", language.t("liveTV")),
                "homebrew" => ("coffee", language.t("homebrew")),
                "browser" => ("globe", language.t("browser")),
                "moonlight" => ("moonlight", "Moonlight"),
                "gaming" => ("gaming", app.title.as_str()),
                _ => ("generic", app.title.as_str()),
            };
            let font = if brand == "generic" {
                text(18.4, 500, -0.035).leading(1.35)
            } else {
                text(24., 500, -0.035).leading(1.2)
            };
            let w = (assets.width(label, font) + 54.4).min(rect.w - 32.);
            let label_w = rect.w - 54.4 - 32.;
            let label_h =
                (assets.lines(label, font, label_w) as f32 * font.size * font.line_height)
                    .min(rect.h - 32.);
            out.icon(
                assets,
                icon,
                Rect::new(cx - w / 2., cy - 20.8, 41.6, 41.6),
                WHITE,
            );
            out.text(
                label,
                Rect::new(cx - w / 2. + 54.4, cy - label_h / 2., label_w, label_h),
                font,
                WHITE,
                false,
            );
        }
    }
    if scale != 1. {
        for draw in &mut out.draw[start..] {
            let rect = match draw {
                Draw::Text(_, rect, style, _, _) => {
                    style.size *= scale;
                    rect
                }
                Draw::TintedImage(_, rect, _, _, _) => rect,
                _ => continue,
            };
            *rect = Rect::new(
                rect.x * scale,
                rect.y * scale,
                rect.w * scale,
                rect.h * scale,
            );
        }
    }
}

fn home(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let language = state.preferences.language;
    let home = &state.preferences.home;
    let summary = state
        .preferences
        .show_weather
        .then(|| weather_summary_layout(state, assets));
    let greeting_height = home.greeting_size.pixels() * 1.05;
    let mut height = if home.show_date { 17. } else { 0. };
    if home.show_greeting {
        height += greeting_height;
    }
    if home.show_date && home.show_greeting {
        height += 20.8;
    }
    if let Some(summary) = &summary {
        if height > 0. {
            height += 18.4;
        }
        height += summary.height;
    }
    let mut y = 293.56 - height / 2. - state.scroll;
    let mut drawn = false;
    let order = if home.greeting_first {
        [true, false]
    } else {
        [false, true]
    };
    for greeting in order {
        if (greeting && !home.show_greeting) || (!greeting && !home.show_date) {
            continue;
        }
        if drawn {
            y += 20.8;
        }
        if greeting {
            let value = home.greeting(language, now.hour(), &state.preferences.greeting_name);
            let style = assets.fit_line(
                &value,
                text(home.greeting_size.pixels(), 500, -0.065).leading(1.05),
                1496.,
            );
            out.text(
                value,
                Rect::new(84., y, 1500., greeting_height.ceil()),
                style,
                WHITE,
                false,
            );
            y += greeting_height;
        } else {
            out.text(
                language.home_date(now, home.compact_date, home.show_weekday),
                Rect::new(84., y, 1200., 17.),
                text(14.4, 600, 0.12),
                c(0xf5f5f5, 0.72),
                false,
            );
            y += 17.;
        }
        drawn = true;
    }
    if let Some(summary) = summary {
        if drawn {
            y += 18.4;
        }
        weather_summary(out, state, assets, summary, y);
    }
    let mut top = 478.5;
    if home.inputs_first {
        let height = home_inputs(out, state, assets, top);
        if height > 0. {
            top += height + 44.9;
        }
    }
    if state.dock_enabled() {
        top = top.max(688.);
    }
    let height = home_favorites(out, state, assets, top);
    if !home.inputs_first {
        home_inputs(out, state, assets, top + height + 44.9);
    }
}

fn home_favorites(out: &mut Scene, state: &State, assets: &Assets, top: f32) -> f32 {
    let language = state.preferences.language;
    let cards = state.home_cards();
    let s = state.scroll;
    out.text(
        language.t("favorites"),
        Rect::new(84., top - s, 1000., 31.),
        text(23.2, 550, -0.035).leading(1.3),
        WHITE,
        false,
    );
    if state.moving.is_some() {
        out.text(
            language.t("moveApps"),
            Rect::new(1686., top + 6.5 - s, 150., 24.),
            text(14.4, 400, 0.),
            c(0xdedede, 1.),
            true,
        );
    }
    let apps = state.apps();
    let names = state.home_names_visible();
    let layout = state.home_layout(apps.len());
    let dock = state.dock_enabled() && !apps.is_empty();
    let (strip_left, strip_right) = state.home_strip();
    let mut grid_height = cards.height;
    if state.loading && state.inventory.apps.is_empty() {
        for index in 0..cards.columns {
            out.shape(
                Rect::new(
                    84. + index as f32 * cards.pitch,
                    top + 66.3 - s,
                    cards.width,
                    cards.height,
                ),
                19.2,
                c(0xffffff, 0.06),
            );
        }
    } else if apps.is_empty() {
        let error = state.error.is_some() && state.inventory.apps.is_empty();
        empty(
            out,
            state,
            assets,
            EmptyState {
                rect: Rect::new(84., top + 55.1 - s, 1752., 258.7),
                icon: if error { "alert" } else { "heart" },
                title: if error {
                    language.t("aLittleHelp")
                } else {
                    language.t("makeFavorites")
                },
                hint: if error {
                    state.error.as_deref().unwrap_or("")
                } else {
                    language.t("pinHint")
                },
                action: Some(if error { Target::Retry } else { Target::Browse }),
            },
        );
        grid_height = 247.5;
    }
    if !apps.is_empty() {
        grid_height = cards.height
            + if names {
                18.4 + apps
                    .iter()
                    .map(|app| caption_height(state, assets, app, cards.width))
                    .fold(49.28, f32::max)
            } else {
                0.
            };
    }
    let row_pitch = grid_height + 28.8;
    if !apps.is_empty() {
        grid_height += layout.rows.saturating_sub(1) as f32 * row_pitch;
    }
    if dock {
        let tray = Rect::new(
            strip_left - 32.,
            top + 46.3 - s,
            strip_right - strip_left + 64.,
            grid_height + 40.,
        );
        out.chrome(tray, 36., false, false, false);
    }
    // Like Apple TV's rows, the favourites run to the screen edge: the next tile shows cut off
    // at the right (and a scrolled-away one at the left), so it is clear the row continues.
    out.draw.push(Draw::Clip(Some(state.favorites_viewport())));
    // Soft edges: out to the right from the end of the sixth tile, and in from the left once
    // the row has scrolled.
    let left = if state.home_scroll > 0. {
        [
            if dock {
                strip_left - 24.
            } else {
                HOME_FADE_LEFT
            },
            strip_left,
        ]
    } else {
        [0., 0.]
    };
    out.draw.push(Draw::EdgeFadeX(Some([
        left[0],
        left[1],
        strip_right,
        if dock {
            strip_right + 24.
        } else {
            HOME_FADE_RIGHT
        },
    ])));
    for (index, app) in apps.iter().enumerate() {
        let (column, row) = layout.cell(index);
        tile(
            out,
            state,
            assets,
            app,
            Rect::new(
                strip_left + column as f32 * cards.pitch - state.home_scroll,
                top + 66.3 + row as f32 * row_pitch - s,
                cards.width,
                cards.height,
            ),
            names,
            false,
        );
    }
    out.draw.push(Draw::EdgeFadeX(None));
    out.draw.push(Draw::Clip(Some(state.content_viewport())));
    66.3 + grid_height + if dock { 20. } else { 0. }
}

fn home_inputs(out: &mut Scene, state: &State, assets: &Assets, top: f32) -> f32 {
    let language = state.preferences.language;
    let s = state.scroll;
    let inputs: Vec<_> = state
        .inventory
        .inputs
        .iter()
        .filter(|input| state.home_input_visible(input))
        .collect();
    if inputs.is_empty() {
        return 0.;
    }
    out.text(
        language.t("inputsHeading"),
        Rect::new(84., top - s, 1000., 31.),
        text(23.2, 550, -0.035).leading(1.3),
        WHITE,
        false,
    );
    for (index, input) in inputs.iter().enumerate() {
        let rect = Rect::new(
            84. + (index % 3) as f32 * 591.45,
            top + 55.1 - s + (index / 3) as f32 * 126.4,
            569.1,
            104.,
        );
        let focused = state.focus == Target::Input(input.id.clone());
        out.shadow(
            rect,
            18.4,
            if focused { 34. } else { 4. },
            if focused { 14. } else { 1. },
            if focused { 0.4 } else { 0.01 },
        );
        out.draw.push(Draw::RoundedGradient(
            rect,
            18.4,
            if focused {
                c(0x30363c, 0.70)
            } else {
                c(0x20252b, 0.60)
            },
            if focused {
                c(0x242a30, 0.65)
            } else {
                c(0x171c22, 0.55)
            },
        ));
        out.border(
            rect,
            18.4,
            1.,
            c(0xffffff, if focused { 0.24 } else { 0.063 }),
        );
        let art = Rect::new(rect.x + 21., rect.y + 24., 56., 56.);
        out.draw.push(Draw::RoundedGradient(
            art,
            12.8,
            c(0xd6d6d6, 0.24),
            c(0xa0a0a0, 0.10),
        ));
        let name = state.preferences.input_name(input);
        let icon = input_icon(input, name);
        let size = if icon == "apple-tv" { 41.6 } else { 35.2 };
        out.icon(
            assets,
            &format!("input-{icon}"),
            Rect::new(
                art.x + (56. - size) / 2.,
                art.y + (56. - size) / 2.,
                size,
                size,
            ),
            c(0xdedede, 1.),
        );
        out.text(
            state.preferences.input_title(input),
            Rect::new(rect.x + 93., rect.y + 24.3, 450., 28.8),
            text(24., 500, -0.03),
            WHITE,
            false,
        );
        out.text(
            input_status(input, name.is_some(), language),
            Rect::new(rect.x + 93., rect.y + 58.7, 450., 20.),
            text(13.12, 400, 0.).leading(1.5),
            c(0xe8e8e8, 0.48),
            false,
        );
        if focused {
            out.ring(rect, 18.4);
        }
        out.control(Target::Input(input.id.clone()), rect, true);
    }
    55.1 + inputs.len().div_ceil(3).saturating_sub(1) as f32 * 126.4 + 104.
}

struct WeatherSummary {
    height: f32,
    width: f32,
    icon: String,
    parts: Vec<(String, TextStyle, Color, Rect)>,
}
type WeatherPart = (String, TextStyle, Color, f32, f32);
fn weather_summary_layout(state: &State, assets: &Assets) -> WeatherSummary {
    let language = state.preferences.language;
    let base = text(23.2, 400, -0.025).leading(1.5);
    let mut parts = Vec::new();
    let mut icon = "location".to_owned();
    if let Some(place) = &state.preferences.weather_location {
        parts.push((place.name.clone(), base.weight(450), c(0xf5f5f5, 0.72)));
        parts.push(("·".into(), base, c(0xf5f5f5, 0.324)));
        if let Some(reading) = state.weather.reading(place) {
            let condition = weather::condition(reading);
            icon = format!("weather-{}", condition.icon);
            parts.push((
                weather::format_temperature(
                    reading.temperature,
                    state.preferences.temperature_unit,
                ),
                base.weight(550),
                c(0xf5f5f5, 0.91),
            ));
            parts.push((language.t(condition.key).into(), base, c(0xf5f5f5, 0.58)));
            if state.weather.failed {
                parts.push((
                    language.t("lastReading").into(),
                    text(15.08, 400, 0.).leading(1.5),
                    c(0xf2d5a5, 0.65),
                ));
            }
        } else {
            parts.push((
                if state.weather.loading {
                    language.t("loadingWeather")
                } else {
                    language.t("weatherUnavailable")
                }
                .into(),
                base,
                c(0xf5f5f5, 0.58),
            ));
        }
    } else {
        parts.push((
            language.t("setWeatherLocation").into(),
            base,
            c(0xf5f5f5, 0.72),
        ));
    }
    let mut rows: Vec<Vec<WeatherPart>> = vec![Vec::new()];
    let mut used = 0.;
    for (label, style, color) in parts {
        let measured_width = assets.width(&label, style);
        let width = measured_width.min(1714.4);
        // Use the same extra pixel as the rendered text rectangle. Counting
        // lines at the exact advance width can round into a phantom second
        // line even though the renderer displays the label on one line.
        let text_width = width + 1.;
        let lines = if measured_width <= text_width && !label.contains('\n') {
            1
        } else {
            assets.lines(&label, style, text_width)
        };
        let height = lines as f32 * style.size * style.line_height;
        if used > 0. && used + 11.2 + width > 1714.4 {
            rows.push(Vec::new());
            used = 0.;
        }
        rows.last_mut()
            .unwrap()
            .push((label, style, color, width, height));
        used += width + if used > 0. { 11.2 } else { 0. };
    }
    let mut positioned = Vec::new();
    let mut top = 1.6;
    let mut right: f32 = 0.;
    for row in rows {
        let height = row.iter().map(|part| part.4).fold(34.8, f32::max);
        let mut x = 121.6;
        for (label, style, color, width, item_height) in row {
            positioned.push((
                label,
                style,
                color,
                Rect::new(
                    x,
                    top + (height - item_height) / 2.,
                    width + 1.,
                    item_height,
                ),
            ));
            x += width + 11.2;
        }
        right = right.max(x - 11.2);
        top += height + 11.2;
    }
    WeatherSummary {
        height: top - 11.2 + 1.6,
        width: right - 84.,
        icon,
        parts: positioned,
    }
}
fn weather_summary(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    summary: WeatherSummary,
    y: f32,
) {
    let label_focus = summary.parts.first().map(|(_, style, _, rect)| {
        // The label's layout line box includes leading above and below the
        // text. Keep the focus ring snug to the label rather than the whole
        // weather row, which also contains the location icon.
        let leading = style.size * (style.line_height - 1.).max(0.);
        Rect::new(
            rect.x,
            y + rect.y + leading / 2.,
            rect.w,
            (rect.h - leading).max(style.size),
        )
    });
    out.icon(
        assets,
        &summary.icon,
        Rect::new(84., y + 5.8, 26.4, 26.4),
        c(0xdedede, 0.83),
    );
    for (label, style, color, rect) in summary.parts {
        out.text(
            label,
            Rect::new(rect.x, rect.y + y, rect.w, rect.h),
            style,
            color,
            false,
        );
    }
    if state.preferences.weather_location.is_none() {
        let rect = Rect::new(84., y, summary.width, summary.height);
        if state.focus == Target::WeatherConfigure
            && !state.pointer
            && let Some(label) = label_focus
        {
            out.border(
                label.inset(-5.),
                8.6,
                2.,
                out.accent_color(c(0xf0f0f0, 0.95)),
            );
        }
        out.control(Target::WeatherConfigure, rect, true);
    }
}

fn library(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let cards = state.apps_cards();
    let s = state.scroll;
    let mut x = 84.;
    for (index, label) in CATEGORIES.into_iter().enumerate() {
        let label = language.category(label);
        let width = assets.width(label, text(16., 400, 0.)) + 35.6;
        let rect = Rect::new(x, 171.4 - s, width, 42.8);
        filter(
            out,
            state,
            Target::Filter(index + 1),
            rect,
            label,
            state.filter == index + 1,
        );
        x += width + 7.2;
    }
    if state.hidden_count() > 0 {
        let label = language.format("hidden", &[("count", &state.hidden_count().to_string())]);
        let rect = Rect::new(
            x,
            171.4 - s,
            assets.width(&label, text(16., 400, 0.)) + 35.6,
            42.8,
        );
        filter(
            out,
            state,
            Target::Filter(5),
            rect,
            &label,
            state.filter == 5,
        );
        x += rect.w + 7.2;
    }
    if state.filter != 0 {
        let rect = Rect::new(x, 176.8 - s, 32., 32.);
        out.shape(rect, 16., c(0x23282c, 0.5));
        out.icon(assets, "close", rect.inset(8.), c(0xeeeeee, 0.73));
        if state.focus == Target::ClearFilter {
            out.border(rect.inset(-2.), 18., 2., out.accent_color(WHITE));
        }
        out.control(Target::ClearFilter, rect, true);
    }
    let search = Rect::new(1532., 164.8 - s, 304., 56.);
    let focused = matches!(state.focus, Target::Search | Target::ClearSearch);
    out.shape(
        search,
        48.,
        if focused {
            c(0x303438, 1.)
        } else {
            c(0x171c20, 0.72)
        },
    );
    out.border(
        search,
        48.,
        1.,
        c(0xffffff, if focused { 0.70 } else { 0.07 }),
    );
    if focused {
        out.border(search.inset(-3.), 51., 3., c(0xffffff, 0.063));
    }
    out.icon(
        assets,
        "search",
        Rect::new(search.x + 19.4, search.y + 18.4, 19.2, 19.2),
        c(0xeeeeee, 0.73),
    );
    out.text(
        if state.query.is_empty() {
            language.t("findApp")
        } else {
            &state.query
        },
        Rect::new(
            search.x + 49.8,
            search.y + 17.8,
            if state.query.is_empty() { 234. } else { 202. },
            22.,
        ),
        text(16.8, 400, -0.02),
        c(0xeeeeee, if state.query.is_empty() { 0.73 } else { 1. }),
        false,
    );
    out.control(Target::Search, search, true);
    if !state.query.is_empty() {
        let clear = Rect::new(search.x + search.w - 45.8, search.y + 14.8, 26.4, 26.4);
        out.shape(clear, 13.2, c(0xffffff, 0.055));
        out.icon(assets, "close", clear.inset(5.2), WHITE);
        if state.focus == Target::ClearSearch {
            out.border(clear.inset(-2.), 15.2, 2., out.accent_color(WHITE));
        }
        out.control(Target::ClearSearch, clear, true);
    }
    let favorites = state.preferences.favorites(&state.inventory.apps);
    let apps = state.apps();
    if apps.is_empty() {
        let title = if state.loading {
            language.t("findingApps")
        } else if !state.query.is_empty() {
            language.t("noMatches")
        } else if state.filter == 5 {
            language.t("noHidden")
        } else {
            language.t("roomForNew")
        };
        let hint = state.error.clone().unwrap_or_else(|| {
            if !state.query.is_empty() {
                language.format("noMatchesHint", &[("query", &state.query)])
            } else if state.filter == 5 {
                language.t("hiddenHint").into()
            } else {
                language.t("noCategoryApps").into()
            }
        });
        empty(
            out,
            state,
            assets,
            EmptyState {
                rect: Rect::new(84., 259.2 - s, 1752., 304.),
                icon: "grid",
                title,
                hint: &hint,
                action: state.error.as_ref().map(|_| Target::Retry),
            },
        );
    }
    let mut y = 270.4;
    let names = state.preferences.show_app_names;
    for row in apps.chunks(cards.columns) {
        let row_height = cards.height
            + if names {
                18.4 + row
                    .iter()
                    .map(|app| caption_height(state, assets, app, cards.width))
                    .fold(49.28, f32::max)
            } else {
                0.
            };
        for (column, app) in row.iter().enumerate() {
            tile(
                out,
                state,
                assets,
                app,
                Rect::new(
                    84. + column as f32 * cards.pitch,
                    y - s,
                    cards.width,
                    cards.height,
                ),
                names,
                favorites.contains(&app.id),
            );
        }
        y += row_height + 38.4;
    }
}

fn filter(out: &mut Scene, state: &State, target: Target, rect: Rect, label: &str, selected: bool) {
    if selected {
        out.shape(rect, 32., out.accent_color(c(0xffffff, 0.11)));
        out.border(rect, 32., 1., out.accent_color(c(0xffffff, 0.07)));
    }
    if state.focus == target && !state.pointer {
        out.border(
            rect.inset(-7.),
            39.,
            2.,
            out.accent_color(c(0xf0f0f0, 0.95)),
        );
    }
    out.text(
        label,
        Rect::new(rect.x, rect.y + 11.4, rect.w, 20.),
        text(16., 400, 0.),
        c(
            if selected { 0xf5f5f5 } else { 0xf0f0f0 },
            if selected { 1. } else { 0.66 },
        ),
        true,
    );
    out.control(target, rect, true);
}

struct Row<'a> {
    target: Target,
    rect: Rect,
    title: &'a str,
    icon: Option<&'a str>,
    value: Option<&'a str>,
    description: Option<&'a str>,
    selected: bool,
    chevron: bool,
}
fn row(out: &mut Scene, state: &State, assets: &Assets, row: Row<'_>) {
    let focused = state.focus == row.target;
    let enabled = state.target_enabled(&row.target);
    let bright = focused && enabled;
    let radius = if row.description.is_some() { 20. } else { 48. };
    out.material(row.rect, radius, bright, false);
    if focused {
        out.border(row.rect, radius, 2., out.accent_color(c(0xffffff, 0.90)));
    }
    out.pressed_feedback(state, &row.target, row.rect, radius);
    let color = if bright {
        c(0x2c2c2c, 1.)
    } else {
        c(0xeeeeee, 0.73)
    };
    let color = if enabled { color } else { c(0xffffff, 0.45) };
    let mut x = row.rect.x + 24.2;
    if let Some(icon) = row.icon {
        out.icon(
            assets,
            icon,
            Rect::new(x, row.rect.y + (row.rect.h - 23.2) / 2., 23.2, 23.2),
            if bright {
                c(0x444444, 1.)
            } else if row.selected {
                out.accent_color(c(0xe4e4e4, 0.61))
            } else {
                c(0xe4e4e4, 0.61)
            },
        );
        x += 39.2;
    }
    let title_y = row.rect.y
        + if row.description.is_some() {
            17.
        } else {
            (row.rect.h - 29.12) / 2.
        };
    let title_width = row.rect.w - (x - row.rect.x) - if row.value.is_some() { 210. } else { 64. };
    let title_height = if row.description.is_some() {
        assets.lines(row.title, text(20.8, 550, -0.02), title_width) as f32 * 29.12
    } else {
        29.12
    };
    out.text(
        row.title,
        Rect::new(x, title_y, title_width, title_height),
        if row.description.is_some() {
            text(20.8, 550, -0.02).leading(1.4)
        } else {
            assets.fit_line(row.title, text(20.8, 550, -0.02), title_width)
        },
        color,
        false,
    );
    if let Some(description) = row.description {
        out.text(
            description,
            Rect::new(
                x,
                title_y + title_height + 4.8,
                row.rect.w - 100.,
                (row.rect.h - 34. - title_height - 4.8).max(22.8),
            ),
            text(15.2, 400, 0.).leading(1.5),
            if bright {
                c(0x555555, 1.)
            } else {
                c(0xf5f5f5, 0.56)
            },
            false,
        );
    }
    if let Some(value) = row.value {
        out.text(
            value,
            Rect::new(
                row.rect.x + row.rect.w - 194.2 - if row.chevron { 34.4 } else { 0. },
                row.rect.y + (row.rect.h - 23.) / 2.,
                170.,
                24.,
            ),
            text(18.4, 450, 0.).right(),
            if bright {
                c(0x444444, 1.)
            } else {
                c(0xeeeeee, 0.73)
            },
            false,
        );
    }
    if row.chevron || row.selected {
        out.icon(
            assets,
            if row.selected { "check" } else { "right" },
            Rect::new(
                row.rect.x + row.rect.w - 42.6,
                row.rect.y + (row.rect.h - 18.4) / 2.,
                18.4,
                18.4,
            ),
            if bright {
                c(0x444444, 1.)
            } else if row.selected {
                out.accent_color(c(0xe4e4e4, 0.61))
            } else {
                c(0xe4e4e4, 0.61)
            },
        );
    }
    out.control(row.target, row.rect, state.overlay.is_none());
}

fn weather_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    row(
        out,
        state,
        assets,
        Row {
            target: Target::WeatherToggle,
            rect: Rect::new(640., 164.8 - s, 1120., 72.),
            title: language.t("showWidget"),
            icon: Some("eye"),
            value: Some(if state.preferences.show_weather {
                language.t("stateOn")
            } else {
                language.t("stateOff")
            }),
            description: None,
            selected: false,
            chevron: false,
        },
    );
    let city = state
        .preferences
        .weather_location
        .as_ref()
        .map(|place| place.name.as_str());
    let style = text(18.4, 450, 0.).leading(1.4);
    let city_lines = city.map_or(1, |city| assets.lines(city, style, 450.));
    let height = (34. + city_lines as f32 * 25.76).max(72.);
    let rect = Rect::new(640., 264.8 - s, 1120., height);
    row(
        out,
        state,
        assets,
        Row {
            target: Target::WeatherLocation,
            rect,
            title: language.t("location"),
            icon: Some("location"),
            value: None,
            description: None,
            selected: false,
            chevron: true,
        },
    );
    if let Some(city) = city {
        out.text(
            city,
            Rect::new(
                rect.x + rect.w - 508.6,
                rect.y + (height - city_lines as f32 * 25.76) / 2.,
                450.,
                city_lines as f32 * 25.76,
            ),
            style.right(),
            if state.focus == Target::WeatherLocation {
                c(0x444444, 1.)
            } else {
                c(0xeeeeee, 0.73)
            },
            false,
        );
    }
    let y = rect.y + height + 12.;
    row(
        out,
        state,
        assets,
        Row {
            target: Target::WeatherTemperature,
            rect: Rect::new(640., y, 1120., 72.),
            title: language.t("temperature"),
            icon: Some("thermometer"),
            value: Some(state.preferences.temperature_unit.symbol()),
            description: None,
            selected: false,
            chevron: true,
        },
    );
}

fn location_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let mut y = 164.8 - state.scroll;
    if let Some(place) = &state.preferences.weather_location {
        let rect = Rect::new(640., y, 1120., 76.2);
        out.shape(rect, 20., c(0x171c20, 0.72));
        out.border(rect, 20., 1., c(0xffffff, 0.04));
        out.icon(
            assets,
            "location",
            Rect::new(664.2, y + 29.1, 18., 18.),
            c(0xcccccc, 1.),
        );
        out.text(
            &place.name,
            Rect::new(698.2, y + 17., 920., 25.),
            text(18.4, 500, 0.),
            WHITE,
            false,
        );
        out.text(
            place.detail(),
            Rect::new(698.2, y + 42.3, 920., 22.),
            text(14.4, 400, 0.),
            c(0xf5f5f5, 0.56),
            false,
        );
        let clear = Rect::new(1695.8, y + 18.1, 40., 40.);
        out.shape(clear, 20., c(0x23282c, 0.5));
        out.border(clear, 20., 1., c(0xffffff, 0.07));
        out.icon(assets, "close", clear.inset(12.), WHITE);
        if state.focus == Target::WeatherClear {
            out.border(
                clear.inset(-2.),
                22.,
                2.,
                out.accent_color(c(0xf0f0f0, 0.95)),
            );
        }
        out.control(Target::WeatherClear, clear, true);
        y += 96.2;
    }
    let query = Rect::new(640., y, 1044., 64.);
    let focused = state.focus == Target::WeatherQuery;
    out.shape(
        query,
        48.,
        if focused {
            c(0x303438, 1.)
        } else {
            c(0x171c20, 0.72)
        },
    );
    out.border(
        query,
        48.,
        1.,
        c(0xffffff, if focused { 0.70 } else { 0.07 }),
    );
    if focused {
        out.border(query.inset(-3.), 51., 3., c(0xffffff, 0.063));
    }
    out.icon(
        assets,
        "search",
        Rect::new(664.2, y + 21.2, 21.6, 21.6),
        c(0xeeeeee, 0.73),
    );
    out.text(
        if state.weather.query.is_empty() {
            language.t("cityPostcode")
        } else {
            &state.weather.query
        },
        Rect::new(699.4, y + 20.9, 950., 24.),
        text(18.4, 400, -0.02),
        c(0xeeeeee, 0.73),
        false,
    );
    out.control(Target::WeatherQuery, query, true);
    let search = Rect::new(1696., y, 64., 64.);
    out.shape(search, 32., c(0x171c20, 0.72));
    out.border(search, 32., 1., c(0xffffff, 0.07));
    out.icon(
        assets,
        if state.weather.searching {
            "refresh"
        } else {
            "search"
        },
        search.inset(20.8),
        WHITE,
    );
    if state.focus == Target::WeatherSearch {
        out.border(
            search.inset(-7.),
            39.,
            2.,
            out.accent_color(c(0xf0f0f0, 0.95)),
        );
    }
    out.control(Target::WeatherSearch, search, true);
    y += 84.;
    if state.weather.searching || state.weather.message.is_some() {
        out.text(
            if state.weather.searching {
                language.t("searching")
            } else {
                language.t(state.weather.message.as_deref().unwrap_or(""))
            },
            Rect::new(664.2, y, 1071.6, 58.),
            text(16., 400, 0.).leading(1.5),
            c(0xe0e0e0, 0.68),
            false,
        );
    }
    for (index, place) in state.weather.results.iter().enumerate() {
        let detail = place.detail();
        let height = (34.
            + assets.lines(&place.name, text(20.8, 550, -0.02), 992.6) as f32 * 29.12
            + 4.8
            + assets.lines(&detail, text(15.2, 400, 0.), 1020.) as f32 * 22.8)
            .max(90.7);
        let rect = Rect::new(640., y, 1120., height);
        row(
            out,
            state,
            assets,
            Row {
                target: Target::WeatherResult(index),
                rect,
                title: &place.name,
                icon: Some("location"),
                value: None,
                description: Some(&detail),
                selected: false,
                chevron: false,
            },
        );
        if state
            .preferences
            .weather_location
            .as_ref()
            .is_some_and(|saved| saved.key() == place.key())
        {
            out.icon(
                assets,
                "check",
                Rect::new(1717.4, y + (height - 18.4) / 2., 18.4, 18.4),
                if state.focus == Target::WeatherResult(index) {
                    c(0x444444, 1.)
                } else {
                    c(0xe4e4e4, 0.61)
                },
            );
        }
        y += height + 12.;
    }
}

fn paint_wallpaper(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    id: &str,
    suffix: &str,
    alpha: f32,
) {
    let rect = Rect::new(0., 0., 1920., 1080.);
    let tint = [1., 1., 1., alpha];
    if crate::features::wallpapers::valid_id(id) {
        let key = format!("{id}{suffix}");
        let image = state
            .wallpapers
            .decoded
            .get(&key)
            .and_then(Option::as_ref)
            .or_else(|| {
                state
                    .wallpapers
                    .photos
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| p.preview.as_ref())
            });
        if let Some(image) = image {
            out.draw.push(Draw::AppImage(image.clone(), rect, tint));
            return;
        }
    }
    let fallback = if crate::features::wallpapers::valid_id(id) {
        &state.preferences.background
    } else {
        id
    };
    let path = assets.path(format!("wallpapers/{fallback}{suffix}.jpg"));
    if alpha >= 1. {
        out.image(path, rect, 0., false);
    } else {
        out.draw
            .push(Draw::TintedImage(path, rect, 0., false, tint));
    }
}

fn settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let s = state.scroll;
    for (index, key) in ["interface", "background", "weather", "inputs", "system"]
        .into_iter()
        .enumerate()
    {
        let label = language.t(key);
        let rect = Rect::new(160., 164.8 + index as f32 * 84. - s, 403.2, 72.);
        let focused = state.focus == Target::Sidebar(index);
        out.pill(rect, focused, state.section == index);
        let color = if focused {
            c(0x2c2c2c, 1.)
        } else if state.section == index {
            out.accent_color(WHITE)
        } else {
            c(0xeeeeee, 0.73)
        };
        if state.section == index && state.subpage.is_some() {
            let clock_path = match state.subpage {
                Some(Subpage::ClockFormat) => Some(format!(
                    "{} › {}",
                    language.t("clock"),
                    language.t("clockFormat")
                )),
                Some(Subpage::TimeFormat) => Some(format!(
                    "{} › {}",
                    language.t("clock"),
                    language.t("timeFormat")
                )),
                _ => None,
            };
            let sub = match &state.subpage {
                Some(Subpage::AppNames) => language.t("appNames"),
                Some(Subpage::Greeting) => language.t("greeting"),
                Some(Subpage::Home) => language.t("homeLabel"),
                Some(Subpage::Appearance) => language.t("appearance"),
                Some(Subpage::Display) => language.t("displayResolution"),
                Some(Subpage::Rotation) => language.t("wallpaperRotation"),
                Some(Subpage::Wallpapers) => language.t("myImages"),
                Some(Subpage::Shortcuts) => language.t("remoteShortcuts"),
                Some(Subpage::StartupFocus) => language.t("homeStartupFocus"),
                Some(Subpage::Clock) => language.t("clock"),
                Some(Subpage::ClockFormat | Subpage::TimeFormat) => clock_path.as_deref().unwrap(),
                Some(Subpage::Input(id)) => state
                    .inventory
                    .inputs
                    .iter()
                    .find(|input| &input.id == id)
                    .map_or(language.t("inputs"), |input| {
                        state.preferences.input_title(input)
                    }),
                Some(Subpage::WeatherLocation) => language.t("location"),
                Some(Subpage::Temperature) => language.t("temperature"),
                Some(Subpage::Language) => language.t("language"),
                Some(Subpage::TextRendering) => language.t("textRendering"),
                Some(Subpage::SystemInfo) => language.t("tvInformation"),
                Some(Subpage::Privacy) => language.t("privacy"),
                None => "",
            };
            let parent_width = assets.width(label, text(20.8, 550, -0.02)) + 2.;
            out.text(
                label,
                Rect::new(rect.x + 24.2, rect.y + 23.5, parent_width, 25.),
                text(20.8, 550, -0.02),
                [color[0], color[1], color[2], color[3] * 0.7],
                false,
            );
            out.icon(
                assets,
                "right",
                Rect::new(rect.x + 24.2 + parent_width + 5., rect.y + 27., 17., 17.),
                [color[0], color[1], color[2], color[3] * 0.6],
            );
            let sub_width = rect.w - 24.2 - parent_width - 28. - 24.2;
            let mut sub_style = text(20.8, 550, -0.02);
            if clock_path.is_some()
                || matches!(
                    state.subpage,
                    Some(Subpage::Home | Subpage::StartupFocus | Subpage::Shortcuts)
                )
            {
                sub_style.size *= (sub_width / assets.width(sub, sub_style)).min(1.);
            }
            out.text(
                sub,
                Rect::new(
                    rect.x + 24.2 + parent_width + 28.,
                    rect.y + 23.5,
                    sub_width,
                    25.,
                ),
                sub_style,
                color,
                false,
            );
        } else {
            out.text(
                label,
                Rect::new(rect.x + 24.2, rect.y + 23.5, 330., 25.),
                text(20.8, 550, -0.02),
                color,
                false,
            );
            if state.section == index {
                out.icon(
                    assets,
                    "right",
                    Rect::new(rect.x + rect.w - 42.6, rect.y + 26.8, 18.4, 18.4),
                    if focused {
                        c(0x2c2c2c, 0.6)
                    } else {
                        c(0xeeeeee, 0.5)
                    },
                );
            }
        }
        out.control(Target::Sidebar(index), rect, true);
    }
    if let Some(subpage) = &state.subpage {
        let start = out.draw.len();
        if matches!(
            subpage,
            Subpage::WeatherLocation
                | Subpage::Home
                | Subpage::Appearance
                | Subpage::Wallpapers
                | Subpage::Rotation
                | Subpage::Display
                | Subpage::Shortcuts
                | Subpage::StartupFocus
                | Subpage::Greeting
                | Subpage::AppNames
                | Subpage::Clock
                | Subpage::SystemInfo
                | Subpage::Privacy
        ) {
            match subpage {
                Subpage::WeatherLocation => location_settings(out, state, assets),
                Subpage::Greeting => greeting_settings(out, state, assets),
                Subpage::Home => home_settings(out, state, assets),
                Subpage::Appearance => appearance_settings(out, state, assets),
                Subpage::Display => display_settings(out, state, assets),
                Subpage::Rotation => rotation_settings(out, state, assets),
                Subpage::Wallpapers => photo_settings(out, state, assets),
                Subpage::Shortcuts => shortcut_settings(out, state, assets),
                Subpage::StartupFocus => startup_focus_settings(out, state, assets),
                Subpage::AppNames => app_name_settings(out, state, assets),
                Subpage::Clock => clock_settings(out, state, assets),
                Subpage::SystemInfo => system_information(out, state, assets),
                Subpage::Privacy => privacy_information(out, state, assets),
                _ => unreachable!(),
            }
            if let Some(opened) = state.panel_started {
                out.fade(start, (opened.elapsed().as_secs_f32() / 0.15).min(1.), 1.);
            }
            return;
        }
        let options: Vec<(&str, Option<&str>, bool)> = match subpage {
            Subpage::ClockFormat => vec![
                (language.t("hour12"), None, !state.preferences.hour_24),
                (language.t("hour24"), None, state.preferences.hour_24),
            ],
            Subpage::TimeFormat => vec![
                ("HH:MM", None, !state.preferences.show_seconds),
                ("HH:MM:SS", None, state.preferences.show_seconds),
            ],
            Subpage::Input(id) => {
                let current = state
                    .preferences
                    .input_modes
                    .get(id)
                    .copied()
                    .unwrap_or_default();
                vec![
                    (
                        language.t("stateOn"),
                        Some(language.t("inputAlways")),
                        current == InputMode::On,
                    ),
                    (
                        language.t("stateOff"),
                        Some(language.t("inputNever")),
                        current == InputMode::Off,
                    ),
                    (
                        language.t("stateAuto"),
                        Some(language.t("inputWhenConnected")),
                        current == InputMode::Auto,
                    ),
                ]
            }
            Subpage::Temperature => vec![
                (
                    language.t("celsius"),
                    None,
                    state.preferences.temperature_unit == Unit::Celsius,
                ),
                (
                    language.t("fahrenheit"),
                    None,
                    state.preferences.temperature_unit == Unit::Fahrenheit,
                ),
            ],
            Subpage::WeatherLocation
            | Subpage::Home
            | Subpage::Appearance
            | Subpage::Wallpapers
            | Subpage::Rotation
            | Subpage::Display
            | Subpage::Shortcuts
            | Subpage::StartupFocus
            | Subpage::Greeting
            | Subpage::AppNames
            | Subpage::Clock
            | Subpage::SystemInfo => {
                unreachable!()
            }
            Subpage::Privacy => {
                unreachable!()
            }
            Subpage::Language => Language::ALL
                .into_iter()
                .map(|option| (option.name(), None, option == language))
                .collect(),
            Subpage::TextRendering => crate::text_rendering::TextRendering::ALL
                .into_iter()
                .map(|mode| {
                    (
                        language.t(mode.label_key()),
                        Some(language.t("textRenderingRestart")),
                        mode == state.preferences.text_rendering,
                    )
                })
                .collect(),
        };
        for (index, (title, description, selected)) in options.into_iter().enumerate() {
            let h = if description.is_some() { 90.7 } else { 72. };
            row(
                out,
                state,
                assets,
                Row {
                    target: Target::Choice(index),
                    rect: Rect::new(640., 164.8 + index as f32 * (h + 12.) - s, 1120., h),
                    title,
                    icon: None,
                    value: None,
                    description,
                    selected,
                    chevron: false,
                },
            );
        }
        if let Subpage::Input(id) = subpage {
            input_name_rows(out, state, assets, id, 164.8 + 3. * 102.7 + 16. - s);
        }
        if let Some(opened) = state.panel_started {
            out.fade(start, (opened.elapsed().as_secs_f32() / 0.15).min(1.), 1.);
        }
        return;
    }
    match state.section {
        0 => {
            for (position, (index, key, icon, value, chevron)) in [
                (0, "appNames", "type", None, true),
                (1, "clock", "clock", None, true),
                (
                    2,
                    "reducedMotion",
                    "motion",
                    Some(if state.preferences.reduced_motion {
                        language.t("stateOn")
                    } else {
                        language.t("stateOff")
                    }),
                    false,
                ),
                (4, "language", "language", Some(language.name()), true),
                (11, "homePersonalization", "settings", None, true),
                (12, "appearance", "grid", None, true),
                (13, "remoteShortcuts", "settings", None, true),
                (
                    19,
                    "saverTimeout",
                    "clock",
                    Some(language.t(state.preferences.screensaver.timeout.key())),
                    false,
                ),
                (
                    20,
                    "saverStyle",
                    "grid",
                    Some(language.t(if state.preferences.screensaver.photos {
                        "saverPhotos"
                    } else {
                        "clock"
                    })),
                    false,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                if index == 19 {
                    if let Some((step, labels)) = state.slider_values(&Target::Setting(19)) {
                        slider::draw(
                            out,
                            state,
                            Target::Setting(19),
                            Rect::new(
                                640.,
                                164.8 + position as f32 * 84. - s,
                                1120.,
                                crate::features::sliders::HEIGHT,
                            ),
                            language.t(key),
                            &labels,
                            step,
                        );
                    }
                    continue;
                }
                row(
                    out,
                    state,
                    assets,
                    Row {
                        target: Target::Setting(index),
                        rect: Rect::new(
                            640.,
                            164.8
                                + position as f32 * 84.
                                + if index == 20 {
                                    crate::features::sliders::HEIGHT - 72.
                                } else {
                                    0.
                                }
                                - s,
                            1120.,
                            72.,
                        ),
                        title: language.t(key),
                        icon: Some(icon),
                        value,
                        description: None,
                        selected: false,
                        chevron,
                    },
                );
            }
        }
        1 => {
            for (index, (id, _title)) in BACKGROUNDS.iter().enumerate() {
                let rect = Rect::new(
                    640. + (index % 3) as f32 * 380.,
                    164.8 + (index / 3) as f32 * 264.3 - s,
                    360.,
                    202.5,
                );
                out.shadow(rect, 16., 15., 3., 0.13);
                out.image(
                    assets.path(format!("thumbnails/{id}.jpg")),
                    rect,
                    16.,
                    false,
                );
                out.border(rect, 16., 1., c(0xffffff, 0.086));
                if state.current_background() == *id {
                    out.border(rect.inset(-2.), 18., 2., out.accent_color(c(0xdedede, 1.)));
                    out.border(
                        rect.inset(-7.),
                        23.,
                        3.,
                        out.accent_color(c(0xdedede, 0.10)),
                    );
                    let check = Rect::new(rect.x + rect.w - 28., rect.y + rect.h - 28., 21.6, 21.6);
                    out.shape(check, 10.8, out.accent_color(c(0xf3f3f3, 1.)));
                    out.icon(assets, "check", check.inset(3.6), c(0x333333, 1.));
                }
                if state.focus == Target::Background(index) {
                    out.ring(rect, 16.);
                }
                out.text(
                    language.wallpaper(id),
                    Rect::new(rect.x + 4., rect.y + rect.h + 12., 352., 26.),
                    text(18.4, 500, -0.02).leading(1.4),
                    c(0xf0f0f0, 0.87),
                    false,
                );
                out.control(
                    Target::Background(index),
                    Rect::new(rect.x, rect.y, rect.w, rect.h + 37.8),
                    true,
                );
            }
            row(
                out,
                state,
                assets,
                Row {
                    target: Target::Setting(17),
                    rect: Rect::new(640., 950. - s, 1120., 72.),
                    title: language.t("myImages"),
                    icon: Some("grid"),
                    value: None,
                    description: None,
                    selected: false,
                    chevron: true,
                },
            );
            row(
                out,
                state,
                assets,
                Row {
                    target: Target::Setting(18),
                    rect: Rect::new(640., 1034. - s, 1120., 72.),
                    title: language.t("wallpaperRotation"),
                    icon: Some("refresh"),
                    value: None,
                    description: None,
                    selected: false,
                    chevron: true,
                },
            );
        }
        WEATHER_SECTION => weather_settings(out, state, assets),
        SYSTEM_SECTION => {
            for (position, (index, title, icon, value, chevron)) in [
                (
                    3,
                    language.t(if state.loading {
                        "refreshNotice"
                    } else {
                        "refreshApps"
                    }),
                    "refresh",
                    None,
                    false,
                ),
                (
                    7,
                    language.t("textRendering"),
                    "type",
                    Some(language.t(state.preferences.text_rendering.label_key())),
                    true,
                ),
                (8, language.t("tvInformation"), "tv", None, true),
                (21, language.t("displayResolution"), "tv", None, true),
                (9, language.t("privacy"), "settings", None, true),
                (
                    14,
                    language.t("blockerRefresh"),
                    "refresh",
                    Some(language.t(if state.controls.loading {
                        "infoReading"
                    } else {
                        match state.controls.report.blocker.as_str() {
                            "on" => "stateOn",
                            "off" => "stateOff",
                            "conflict" => "blockerConflict",
                            _ => "infoUnknown",
                        }
                    })),
                    false,
                ),
                (15, language.t("blockerEnable"), "settings", None, true),
                (16, language.t("blockerDisable"), "refresh", None, true),
                (5, language.t("quitApp"), "power", None, false),
            ]
            .into_iter()
            .enumerate()
            {
                row(
                    out,
                    state,
                    assets,
                    Row {
                        target: Target::Setting(index),
                        rect: Rect::new(640., 164.8 + position as f32 * 84. - s, 1120., 72.),
                        title,
                        icon: Some(icon),
                        value,
                        description: None,
                        selected: false,
                        chevron,
                    },
                );
            }
        }
        _ => {
            for (index, input) in state.inventory.inputs.iter().enumerate() {
                let mode = state
                    .preferences
                    .input_modes
                    .get(&input.id)
                    .copied()
                    .unwrap_or_default();
                row(
                    out,
                    state,
                    assets,
                    Row {
                        target: Target::InputMode(input.id.clone()),
                        rect: Rect::new(640., 164.8 + index as f32 * 102.7 - s, 1120., 90.7),
                        title: state.preferences.input_title(input),
                        icon: Some(&format!(
                            "input-{}",
                            input_icon(input, state.preferences.input_name(input))
                        )),
                        value: Some(language.t(match mode {
                            InputMode::On => "stateOn",
                            InputMode::Off => "stateOff",
                            InputMode::Auto => "stateAuto",
                        })),
                        description: Some(&input_status(
                            input,
                            state.preferences.input_name(input).is_some(),
                            language,
                        )),
                        selected: false,
                        chevron: true,
                    },
                );
            }
            let mut y = 164.8 + state.inventory.inputs.len() as f32 * 102.7 + 16. - s;
            if state.inventory.inputs.is_empty() {
                out.text(
                    language.t("noInputs"),
                    Rect::new(664., 165. - s, 1000., 26.),
                    text(16.8, 400, 0.).leading(1.5),
                    c(0xf5f5f5, 0.72),
                    false,
                );
                y += 36.;
            }
            row(
                out,
                state,
                assets,
                Row {
                    target: Target::Setting(3),
                    rect: Rect::new(640., y, 1120., 72.),
                    title: if state.loading {
                        language.t("refreshNotice")
                    } else {
                        language.t("refreshInputs")
                    },
                    icon: Some("refresh"),
                    value: None,
                    description: None,
                    selected: false,
                    chevron: false,
                },
            );
        }
    }
}

fn privacy_information(out: &mut Scene, state: &State, assets: &Assets) {
    use crate::features::privacy::{FIELDS, Observation};
    let language = state.preferences.language;
    row(
        out,
        state,
        assets,
        Row {
            target: Target::PrivacyRefresh,
            rect: Rect::new(640., 164.8 - state.scroll, 1120., 102.7),
            title: language.t("infoRefresh"),
            icon: Some("refresh"),
            value: state.privacy.loading.then(|| language.t("infoReading")),
            description: Some(language.t(if state.privacy.failed {
                "infoReadFailed"
            } else if !state.webos {
                "infoPreview"
            } else {
                "controlPrivacyHint"
            })),
            selected: false,
            chevron: false,
        },
    );
    for (index, (_, title, consent)) in FIELDS.iter().enumerate() {
        let value = if !state.webos {
            "infoNotApplicable"
        } else {
            state
                .privacy
                .report
                .as_ref()
                .map_or(Observation::Unknown, |report| report.values[index])
                .label(*consent)
        };
        row(
            out,
            state,
            assets,
            Row {
                target: Target::PrivacyField(index),
                rect: Rect::new(640., 282. - state.scroll + index as f32 * 94., 1120., 82.),
                title: language.t(title),
                icon: None,
                value: None,
                description: Some(language.t(value)),
                selected: false,
                chevron: false,
            },
        );
    }
}

fn system_information(out: &mut Scene, state: &State, assets: &Assets) {
    use crate::features::system::Access;
    let language = state.preferences.language;
    let report = state.system.report.as_ref();
    let unavailable = language.t(if !state.webos {
        "infoNotApplicable"
    } else {
        report.map_or("infoUnknown", |report| match report.system_info {
            Access::Available => "infoUnavailable",
            access => access.key(),
        })
    });
    row(
        out,
        state,
        assets,
        Row {
            target: Target::SystemRefresh,
            rect: Rect::new(640., 164.8 - state.scroll, 1120., 102.7),
            title: language.t("infoRefresh"),
            icon: Some("refresh"),
            value: state.system.loading.then(|| language.t("infoReading")),
            description: Some(language.t(if state.system.failed {
                "infoReadFailed"
            } else if !state.webos {
                "infoPreview"
            } else {
                "infoReadOnly"
            })),
            selected: false,
            chevron: false,
        },
    );
    let rows = [
        (
            "infoModel",
            report
                .and_then(|r| r.model.as_deref())
                .unwrap_or(unavailable),
        ),
        (
            "infoWebos",
            report.and_then(|r| r.sdk.as_deref()).unwrap_or(unavailable),
        ),
        (
            "infoFirmware",
            report
                .and_then(|r| r.firmware.as_deref())
                .unwrap_or(unavailable),
        ),
        (
            "infoRoot",
            if !state.webos {
                unavailable
            } else {
                language.t(report.map_or("infoUnknown", |r| r.root.key()))
            },
        ),
        (
            "infoHome",
            if !state.webos {
                unavailable
            } else {
                language.t(report.map_or("infoUnknown", |r| r.home.key()))
            },
        ),
        (
            "infoInstallation",
            if !state.webos {
                unavailable
            } else {
                language.t(report.map_or("infoUnknown", |r| r.installation.key()))
            },
        ),
        (
            "infoHomeBoot",
            if !state.webos {
                unavailable
            } else {
                language.t(report.map_or("infoUnknown", |r| r.home_boot.key()))
            },
        ),
    ];
    for (index, (title, value)) in rows.into_iter().enumerate() {
        row(
            out,
            state,
            assets,
            Row {
                target: Target::SystemField(index),
                rect: Rect::new(640., 282. - state.scroll + index as f32 * 94., 1120., 82.),
                title: language.t(title),
                icon: None,
                value: None,
                description: Some(value),
                selected: false,
                chevron: false,
            },
        );
    }
}

fn display_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    out.text(
        &state.display_report,
        Rect::new(664., 164.8 - state.scroll, 1072., 110.),
        text(18., 400, 0.).leading(1.5),
        WHITE,
        false,
    );
    out.text(
        language.t(if state.display_trial {
            "displayConfirmHint"
        } else {
            "displayExperimentalHint"
        }),
        Rect::new(664., 282. - state.scroll, 1072., 120.),
        text(19., 400, 0.).leading(1.5),
        WHITE,
        false,
    );
    let rows = if state.display_trial {
        vec![
            (Target::DisplayHD, "displayHD"),
            (Target::DisplayKeep, "displayKeep"),
        ]
    } else {
        vec![
            (Target::DisplayHD, "displayHD"),
            (Target::DisplayUHD, "displayUHD"),
        ]
    };
    for (index, (target, key)) in rows.into_iter().enumerate() {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(640., 430. + index as f32 * 84. - state.scroll, 1120., 72.),
                title: language.t(key),
                icon: None,
                value: None,
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
}

fn rotation_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    for (index, target) in state.rotation_controls().into_iter().enumerate() {
        let (title, value, selected) = match &target {
            Target::RotationMode => (
                language.t("wallpaperRotation"),
                Some(language.t(state.preferences.wallpapers.rotation.mode.key())),
                false,
            ),
            Target::RotationShuffle => (
                language.t("rotationShuffle"),
                Some(
                    language.t(if state.preferences.wallpapers.rotation.shuffle {
                        "stateOn"
                    } else {
                        "stateOff"
                    }),
                ),
                false,
            ),
            Target::RotationNext => (language.t("rotationNext"), None, false),
            Target::RotationInclude(id) => (
                state
                    .wallpapers
                    .photos
                    .iter()
                    .find(|p| &p.id == id)
                    .map_or_else(|| language.wallpaper(id), |p| p.title.as_str()),
                None,
                state.preferences.wallpapers.rotation.selected.contains(id),
            ),
            _ => unreachable!(),
        };
        row(
            out,
            state,
            assets,
            Row {
                target: target.clone(),
                rect: Rect::new(640., 164.8 + index as f32 * 84. - state.scroll, 1120., 72.),
                title,
                icon: None,
                value,
                description: None,
                selected,
                chevron: false,
            },
        );
    }
}

fn photo_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let entries = [
        (
            Target::PhotoFolder,
            "photoFolder",
            Some(if state.preferences.wallpapers.folder.is_empty() {
                language.t("stateOff")
            } else {
                language.t("photoConfigured")
            }),
        ),
        (
            Target::PhotoUrl,
            "photoUrl",
            Some(if state.preferences.wallpapers.url.is_empty() {
                language.t("stateOff")
            } else {
                language.t("photoConfigured")
            }),
        ),
        (
            Target::PhotoFit,
            "photoFit",
            Some(language.t(if state.preferences.wallpapers.contain {
                "photoContain"
            } else {
                "photoCover"
            })),
        ),
        (
            Target::PhotoImport,
            "photoImport",
            state.wallpapers.loading.then(|| language.t("infoReading")),
        ),
        (Target::PhotoStock, "photoStock", None),
        (Target::PhotoClearSources, "photoClearSources", None),
    ];
    let mut y = 164.8 - state.scroll;
    for (target, key, value) in entries {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(640., y, 1120., 72.),
                title: language.t(key),
                icon: None,
                value,
                description: None,
                selected: false,
                chevron: false,
            },
        );
        y += 84.;
    }
    for (index, photo) in state.wallpapers.photos.iter().enumerate() {
        row(
            out,
            state,
            assets,
            Row {
                target: Target::Photo(index),
                rect: Rect::new(640., y, 1120., 90.7),
                title: &photo.title,
                icon: None,
                value: None,
                description: None,
                selected: state.preferences.wallpapers.selected.as_deref() == Some(&photo.id),
                chevron: false,
            },
        );
        if let Some(image) = &photo.preview {
            out.draw.push(Draw::AppImage(
                image.clone(),
                Rect::new(1480., y + 8., 132., 74.25),
                WHITE,
            ));
        }
        y += 102.7;
    }
}

fn control_dialog(out: &mut Scene, state: &State, command: crate::features::controls::Command) {
    let language = state.preferences.language;
    let panel = Rect::new(550., 280., 820., 520.);
    out.modal = Some(panel);
    out.shape(panel, 24., c(0x24282d, 1.));
    let mut title = language.t(command.key()).to_owned();
    if let crate::features::controls::Command::PrivacySet { index, value, .. } = command {
        title = format!(
            "{}: {}",
            language.t(crate::features::privacy::FIELDS[index].1),
            language.t(if value { "stateOn" } else { "stateOff" })
        );
    }
    if let crate::features::controls::Command::SleepTimer { value, .. } = command {
        title = format!(
            "{}: {}",
            language.t("ccSleepTimer"),
            if value == 0 {
                language.t("stateOff").to_owned()
            } else {
                format!(
                    "{} {}",
                    crate::features::control_center::SLEEP_MINUTES[value.min(5)],
                    language.t("minutesShort")
                )
            }
        );
    }
    out.text(
        title,
        Rect::new(590., 320., 740., 90.),
        text(26., 550, 0.).leading(1.4),
        WHITE,
        false,
    );
    out.text(
        language.t(match command {
            crate::features::controls::Command::Standby => "ccStandbyHint",
            crate::features::controls::Command::SleepTimer { .. } => "ccTimerHint",
            _ => "controlConfirmHint",
        }),
        Rect::new(590., 425., 740., 160.),
        text(19., 400, 0.).leading(1.5),
        WHITE,
        false,
    );
    let mut buttons = vec![
        (Target::CloseDialog, "cancel"),
        (Target::ControlApply, "controlApply"),
    ];
    if let crate::features::controls::Command::PrivacySet { index, .. } = command
        && state.controls.report.restore[index]
    {
        buttons.push((Target::ControlRestore, "controlRestore"));
    }
    for (index, (target, key)) in buttons.into_iter().enumerate() {
        let rect = Rect::new(590., 610. + index as f32 * 52., 740., 44.);
        out.pill(rect, state.focus == target, false);
        out.text(
            language.t(key),
            Rect::new(rect.x, rect.y + 11., rect.w, 22.),
            text(18., 500, 0.),
            if state.focus == target {
                c(0x2c2c2c, 1.)
            } else {
                WHITE
            },
            true,
        );
        out.control(target, rect, false);
    }
}

fn shortcut_settings(out: &mut Scene, state: &State, assets: &Assets) {
    use crate::features::shortcuts::Button;
    let language = state.preferences.language;
    row(
        out,
        state,
        assets,
        Row {
            target: Target::ShortcutsEnabled,
            rect: Rect::new(640., 164.8 - state.scroll, 1120., 90.7),
            title: language.t("shortcutsEnabled"),
            icon: None,
            value: Some(language.t(if state.preferences.shortcuts.enabled {
                "stateOn"
            } else {
                "stateOff"
            })),
            description: Some(language.t(if state.webos {
                "shortcutScope"
            } else {
                "shortcutDesktopHint"
            })),
            selected: false,
            chevron: false,
        },
    );
    for button in Button::ALL {
        let title = if state.webos {
            language.t(button.key()).to_owned()
        } else {
            format!("{} / F{}", language.t(button.key()), button.index() + 1)
        };
        let status = if state.shortcut_session.observed(button) {
            if state.webos {
                "shortcutReceived"
            } else {
                "shortcutPreviewReceived"
            }
        } else {
            "shortcutNotReceived"
        };
        row(
            out,
            state,
            assets,
            Row {
                target: Target::ShortcutBinding(button),
                rect: Rect::new(
                    640.,
                    267.5 + button.index() as f32 * 102.7 - state.scroll,
                    1120.,
                    90.7,
                ),
                title: &title,
                icon: None,
                value: Some(language.t(state.preferences.shortcuts.get(button).key())),
                description: Some(language.t(status)),
                selected: false,
                chevron: false,
            },
        );
    }
    row(
        out,
        state,
        assets,
        Row {
            target: Target::ShortcutsReset,
            rect: Rect::new(640., 678.3 - state.scroll, 1120., 72.),
            title: language.t("shortcutsReset"),
            icon: Some("refresh"),
            value: None,
            description: None,
            selected: false,
            chevron: false,
        },
    );
}

fn appearance_settings(out: &mut Scene, state: &State, assets: &Assets) {
    use crate::features::appearance::Setting;
    let language = state.preferences.language;
    let mut y = 164.8 - state.scroll;
    for option in Setting::ALL {
        let value = match option {
            Setting::Density => language.t(state.preferences.appearance.density.key()),
            Setting::Accent => language.t(state.preferences.appearance.accent.key()),
            Setting::Dimming => language.t(state.preferences.appearance.dimming.key()),
            Setting::Blur => language.t(state.preferences.appearance.blur.key()),
            Setting::LiquidGlass => language.t(if state.preferences.appearance.liquid_glass {
                "stateOn"
            } else {
                "stateOff"
            }),
        };
        let target = Target::AppearanceOption(option);
        if let Some((step, labels)) = state.slider_values(&target) {
            slider::draw(
                out,
                state,
                target,
                Rect::new(640., y, 1120., crate::features::sliders::HEIGHT),
                language.t(option.key()),
                &labels,
                step,
            );
            y += crate::features::sliders::HEIGHT + 12.;
            continue;
        }
        let description = (option == Setting::LiquidGlass).then_some(language.t("liquidGlassHint"));
        let height = if description.is_some() { 112. } else { 72. };
        row(
            out,
            state,
            assets,
            Row {
                target: Target::AppearanceOption(option),
                rect: Rect::new(640., y, 1120., height),
                title: language.t(option.key()),
                icon: None,
                value: Some(value),
                description,
                selected: false,
                chevron: false,
            },
        );
        y += height + 12.;
    }
}

fn home_settings(out: &mut Scene, state: &State, assets: &Assets) {
    use crate::features::home::{GreetingMode, Setting};
    let language = state.preferences.language;
    let home = &state.preferences.home;
    let mut y = 164.8 - state.scroll;
    for option in Setting::ALL {
        let group = match option {
            Setting::TwoRows => Some("favorites"),
            Setting::ShowGreeting => Some("greeting"),
            Setting::ShowDate => Some("homeDateGroup"),
            Setting::ShowInputs => Some("inputs"),
            Setting::StartupFocus => Some("homeStartupFocus"),
            _ => None,
        };
        if let Some(key) = group {
            if option != Setting::TwoRows {
                y += 20.;
            }
            out.text(
                language.t(key),
                Rect::new(664., y, 1040., 26.),
                text(17., 550, 0.),
                c(0xffffff, 0.88),
                false,
            );
            y += 38.;
        }
        let enabled = |flag| language.t(if flag { "stateOn" } else { "stateOff" });
        let value = match option {
            Setting::TwoRows => enabled(state.preferences.two_row_home_favorites),
            Setting::FavoriteNames => enabled(state.home_names_visible()),
            Setting::FavoritesDock => enabled(state.dock_enabled()),
            Setting::ShowGreeting => enabled(home.show_greeting),
            Setting::GreetingMode => language.t(if home.greeting_mode == GreetingMode::Timed {
                "homeTimed"
            } else {
                "homeStatic"
            }),
            Setting::GreetingSize => language.t(home.greeting_size.key()),
            Setting::ShowDate => enabled(home.show_date),
            Setting::DateFormat => language.t(if home.compact_date {
                "homeCompact"
            } else {
                "homeExtended"
            }),
            Setting::ShowWeekday => enabled(home.show_weekday),
            Setting::ShowInputs => enabled(home.show_inputs),
            Setting::GreetingFirst => enabled(home.greeting_first),
            Setting::InputsFirst => enabled(home.inputs_first),
            Setting::GreetingText | Setting::StartupFocus => "",
        };
        let target = Target::HomeOption(option);
        if let Some((step, labels)) = state.slider_values(&target) {
            slider::draw(
                out,
                state,
                target,
                Rect::new(640., y, 1120., crate::features::sliders::HEIGHT),
                language.t(option.key()),
                &labels,
                step,
            );
            y += crate::features::sliders::HEIGHT + 12.;
            continue;
        }
        let hint = match option {
            Setting::FavoritesDock => Some(if state.preferences.two_row_home_favorites {
                "homeDockLocked"
            } else {
                "homeFavoritesDockHint"
            }),
            Setting::FavoriteNames if state.dock_enabled() => Some("homeNamesDockHint"),
            _ => None,
        };
        let height = if hint.is_some() { 112. } else { 72. };
        row(
            out,
            state,
            assets,
            Row {
                target: Target::HomeOption(option),
                rect: Rect::new(640., y, 1120., height),
                title: language.t(option.key()),
                icon: None,
                value: (!value.is_empty()).then_some(value),
                description: hint.map(|key| language.t(key)),
                selected: false,
                chevron: matches!(option, Setting::GreetingText | Setting::StartupFocus),
            },
        );
        y += height + 12.;
    }
}

fn startup_focus_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let choices = state.startup_choices();
    let selected = if choices
        .iter()
        .any(|(choice, _)| choice == &state.preferences.home.startup_focus)
    {
        state.preferences.home.startup_focus.clone()
    } else {
        crate::features::home::StartupFocus::Auto
    };
    let mut y = 164.8 - state.scroll;
    for (choice, title) in choices {
        let description = match &choice {
            crate::features::home::StartupFocus::Auto => {
                state.preferences.language.t("homeStartupHint")
            }
            crate::features::home::StartupFocus::App(id)
            | crate::features::home::StartupFocus::Input(id) => id,
        };
        let title_lines = assets.lines(&title, text(20.8, 550, -0.02), 1031.8);
        let description_lines = assets.lines(description, text(15.2, 400, 0.), 1020.);
        let height = 90.7
            + title_lines.saturating_sub(1) as f32 * 29.12
            + description_lines.saturating_sub(1) as f32 * 22.8;
        row(
            out,
            state,
            assets,
            Row {
                target: Target::StartupFocus(choice.clone()),
                rect: Rect::new(640., y, 1120., height),
                title: &title,
                icon: None,
                value: None,
                description: Some(description),
                selected: selected == choice,
                chevron: false,
            },
        );
        y += height + 12.;
    }
}

fn greeting_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let height =
        if state.preferences.home.greeting_mode == crate::features::home::GreetingMode::Static {
            113.5
        } else {
            90.7
        };
    let description = if state.text_input_active() && state.keyboard_field == TextField::Name {
        format!("{}|", state.greeting_name_draft)
    } else if state.greeting_text().is_empty() {
        language
            .t(
                if state.preferences.home.greeting_mode
                    == crate::features::home::GreetingMode::Static
                {
                    "homeStaticHint"
                } else {
                    "stateOff"
                },
            )
            .into()
    } else {
        state.greeting_text().to_owned()
    };
    row(
        out,
        state,
        assets,
        Row {
            target: Target::GreetingName,
            rect: Rect::new(640., 164.8 - state.scroll, 1120., height),
            title: language.t(
                if state.preferences.home.greeting_mode
                    == crate::features::home::GreetingMode::Static
                {
                    "homeStaticGreeting"
                } else {
                    "name"
                },
            ),
            icon: Some("type"),
            value: None,
            description: Some(&description),
            selected: false,
            chevron: true,
        },
    );
    // Always present so completing name entry cannot move a clicked control.
    row(
        out,
        state,
        assets,
        Row {
            target: Target::GreetingNameClear,
            rect: Rect::new(640., 164.8 + height + 12. - state.scroll, 1120., 72.),
            title: language.t(
                if state.preferences.home.greeting_mode
                    == crate::features::home::GreetingMode::Static
                {
                    "homeClearGreeting"
                } else {
                    "clearName"
                },
            ),
            icon: Some("refresh"),
            value: None,
            description: None,
            selected: false,
            chevron: false,
        },
    );
}

fn clock_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    for (index, (target, key, value)) in [
        (
            Target::ClockFormat,
            "clockFormat",
            language.t(if state.preferences.hour_24 {
                "hour24"
            } else {
                "hour12"
            }),
        ),
        (
            Target::ClockTimeFormat,
            "timeFormat",
            if state.preferences.show_seconds {
                "HH:MM:SS"
            } else {
                "HH:MM"
            },
        ),
        (
            Target::ClockAvatar,
            "clockAvatar",
            language.t(if state.preferences.header.combined {
                "stateOn"
            } else {
                "stateOff"
            }),
        ),
        (
            Target::ClockSide,
            "clockSide",
            language.t(
                if state.preferences.header.side == crate::features::header::Side::Left {
                    "sideLeft"
                } else {
                    "sideRight"
                },
            ),
        ),
        (
            Target::AvatarStyle,
            "avatarStyle",
            language.t(
                if state.preferences.header.avatar == crate::features::header::Avatar::Initials {
                    "avatarInitials"
                } else {
                    "avatarSymbol"
                },
            ),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(640., 164.8 + index as f32 * 84. - state.scroll, 1120., 72.),
                title: language.t(key),
                icon: None,
                value: Some(value),
                description: None,
                selected: false,
                chevron: true,
            },
        );
    }
}

fn app_name_settings(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let mut y = 164.8 - state.scroll;
    for (target, title, enabled) in [
        (
            Target::AppNamesHome,
            "showOnHome",
            state.home_names_visible(),
        ),
        (
            Target::AppNamesApps,
            "showOnApps",
            state.preferences.show_app_names,
        ),
    ]
    .into_iter()
    {
        let locked = target == Target::AppNamesHome && state.dock_enabled();
        let height = if locked { 112. } else { 72. };
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(640., y, 1120., height),
                title: language.t(title),
                icon: None,
                value: Some(language.t(if enabled { "stateOn" } else { "stateOff" })),
                description: locked.then_some(language.t("homeNamesDockHint")),
                selected: false,
                chevron: false,
            },
        );
        y += height + 12.;
    }
}

fn input_name_rows(out: &mut Scene, state: &State, assets: &Assets, id: &str, y: f32) {
    let language = state.preferences.language;
    let custom = state.preferences.input_names.get(id);
    let editing = state.text_input_active() && state.keyboard_field == TextField::InputName;
    let description = if editing {
        format!("{}|", state.input_name)
    } else {
        custom
            .cloned()
            .unwrap_or_else(|| language.t("inputNameHint").into())
    };
    row(
        out,
        state,
        assets,
        Row {
            target: Target::InputRename,
            rect: Rect::new(640., y, 1120., 90.7),
            title: language.t("renameInput"),
            icon: Some("type"),
            value: None,
            description: Some(&description),
            selected: false,
            chevron: false,
        },
    );
    if custom.is_some() {
        row(
            out,
            state,
            assets,
            Row {
                target: Target::InputNameReset,
                rect: Rect::new(640., y + 102.7, 1120., 72.),
                title: language.t("clearName"),
                icon: Some("refresh"),
                value: None,
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
}

/// A custom name such as "PS5" can choose the icon; otherwise the TV's label.
fn input_icon(input: &Input, custom: Option<&str>) -> &'static str {
    custom
        .map(device_icon)
        .filter(|icon| *icon != "hdmi")
        .unwrap_or_else(|| device_icon(&input.title))
}

fn device_icon(name: &str) -> &'static str {
    let name = name.to_lowercase().replace(' ', "");
    if name.contains("playstation") || ["ps1", "ps2", "ps3", "ps4", "ps5"].contains(&name.as_str())
    {
        "playstation"
    } else if name.contains("appletv") {
        "apple-tv"
    } else if name.contains("usb") {
        "usb"
    } else if ["xbox", "switch", "console"]
        .iter()
        .any(|part| name.contains(part))
    {
        "console"
    } else if ["pc", "computer", "laptop"]
        .iter()
        .any(|part| name.contains(part))
    {
        "computer"
    } else if ["stream", "fire", "roku"]
        .iter()
        .any(|part| name.contains(part))
    {
        "streamer"
    } else {
        "hdmi"
    }
}
/// Detail under an input title: the TV's own label (when connected and not just
/// the port), the port when a custom name replaces it, then the status.
pub fn input_status(input: &Input, renamed: bool, language: Language) -> String {
    let mut parts = vec![];
    if input.connected == Some(true) && input.title != input.port {
        parts.push(input.title.as_str());
    }
    if renamed {
        parts.push(input.port.as_str());
    }
    parts.push(match input.connected {
        Some(false) => language.t("noDevice"),
        None => language.t("externalInput"),
        Some(true) => language.t("connected"),
    });
    parts.join(" · ")
}

fn keycap(out: &mut Scene, label: &str, x: f32, y: f32, w: f32, small: bool) {
    let h = if small { 16. } else { 23.2 };
    out.border(
        Rect::new(x, y, w, h),
        4.8,
        1.,
        c(0xffffff, if small { 0.125 } else { 0.20 }),
    );
    out.text(
        label,
        Rect::new(x, y + if small { 2. } else { 4.2 }, w, h),
        text(if small { 8. } else { 11.52 }, 500, 0.),
        c(0xf3f3f3, 0.64),
        true,
    );
}

fn footer(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let style = text(14.4, 400, 0.005);
    if state.page == Page::Home && state.moving.is_none() {
        return;
    }
    let color = c(0xeeeeee, 0.37);
    if state.moving.is_some() {
        let two_rows = state.preferences.two_row_home_favorites;
        let key_width = if two_rows { 76. } else { 42. };
        let label_x = 84. + key_width + 8.;
        keycap(
            out,
            if two_rows {
                "← ↑ ↓ →"
            } else {
                "← →"
            },
            84.,
            1030.4,
            key_width,
            false,
        );
        out.text(
            language.t("move"),
            Rect::new(label_x, 1033., 100., 20.),
            text(14.4, 400, 0.005),
            color,
            false,
        );
        let second_x = label_x + assets.width(language.t("move"), style) + 25.6;
        keycap(out, "OK/Back", second_x, 1030.4, 62., false);
        out.text(
            language.t("done"),
            Rect::new(second_x + 70., 1033., 150., 20.),
            text(14.4, 400, 0.005),
            color,
            false,
        );
    } else {
        let label = language.t(if state.page == Page::Settings {
            "select"
        } else {
            "open"
        });
        let second_x = 119. + assets.width(label, style) + 25.6;
        keycap(out, "OK", 84., 1030.4, 27., false);
        out.text(
            label,
            Rect::new(119., 1033., 100., 20.),
            text(14.4, 400, 0.005),
            color,
            false,
        );
        if state.page == Page::Settings {
            keycap(out, "Back", second_x, 1030.4, 39., false);
            out.text(
                language.t("homeLabel"),
                Rect::new(second_x + 47., 1033., 180., 20.),
                text(14.4, 400, 0.005),
                color,
                false,
            );
            out.text(
                format!("v{}", assets.version.trim()),
                Rect::new(1775., 1037., 61., 16.),
                text(12.48, 400, 0.),
                c(0xf5f5f5, 0.56),
                false,
            );
        } else {
            out.icon(
                assets,
                "more",
                Rect::new(second_x, 1032., 19.2, 19.2),
                c(0xeeeeee, 0.26),
            );
            out.text(
                language.t("options"),
                Rect::new(second_x + 27.2, 1033., 400., 20.),
                text(14.4, 400, 0.005),
                color,
                false,
            );
        }
    }
}

struct EmptyState<'a> {
    rect: Rect,
    icon: &'a str,
    title: &'a str,
    hint: &'a str,
    action: Option<Target>,
}
fn empty(out: &mut Scene, state: &State, assets: &Assets, content: EmptyState<'_>) {
    let EmptyState {
        rect,
        icon,
        title,
        hint,
        action,
    } = content;
    out.shape(rect, 17.6, c(0xffffff, 0.02));
    out.border(rect, 17.6, 1., c(0xffffff, 0.05));
    let center = rect.x + rect.w / 2.;
    let top = rect.y
        + if action.is_some() {
            33.
        } else {
            (rect.h - 123.2) / 2.
        };
    out.icon(
        assets,
        icon,
        Rect::new(center - 17.6, top, 35.2, 35.2),
        c(0xd8d8d8, 0.60),
    );
    out.text(
        title,
        Rect::new(rect.x + 32., top + 52., rect.w - 64., 31.),
        text(24., 550, -0.035),
        WHITE,
        true,
    );
    out.text(
        hint,
        Rect::new(center - 352., top + 97.6, 704., 64.),
        text(16., 400, 0.).leading(1.6),
        c(0xf5f5f5, 0.72),
        true,
    );
    if let Some(target) = action {
        let label = state.preferences.language.t(if target == Target::Retry {
            "retry"
        } else {
            "exploreApps"
        });
        let width = assets.width(label, text(15.2, 600, 0.)) + 72.;
        let button = Rect::new(center - width / 2., top + 144.8, width, 46.8);
        out.shape(button, 11.2, c(0xededed, 1.));
        if state.focus == target {
            out.border(button.inset(-7.), 18.2, 2., out.accent_color(WHITE));
        }
        out.text(
            label,
            Rect::new(button.x + 20.8, button.y + 14.4, button.w - 72., 18.4),
            text(15.2, 600, 0.),
            c(0x242424, 1.),
            false,
        );
        out.icon(
            assets,
            "arrow",
            Rect::new(button.x + button.w - 38.4, button.y + 14.6, 17.6, 17.6),
            c(0x242424, 1.),
        );
        out.control(target, button, true);
    }
}

fn options(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    id: &str,
    removal: Option<&crate::features::apps::management::RemovalTarget>,
) {
    let language = state.preferences.language;
    let app = state.inventory.apps.iter().find(|app| app.id == id);
    let title = app.map_or(id, |app| app.title.as_str());
    let extra = assets
        .lines(title, text(37.6, 500, -0.055).leading(1.1), 432.)
        .saturating_sub(1) as f32
        * 41.4;
    let added = if removal.is_some() { 76.8 } else { 0. };
    let panel = Rect::new(
        680.,
        362.9 - extra / 2. - added / 2.,
        560.,
        354.2 + extra + added,
    );
    out.modal_panel(panel, 32.);
    out.text(
        title,
        Rect::new(717.8, panel.y + 36.2, 432., 42. + extra),
        text(37.6, 500, -0.055).leading(1.1),
        WHITE,
        false,
    );
    out.text(
        language.category(app.map_or("Utilities", |app| app.category.as_str())),
        Rect::new(717.8, panel.y + 87.9 + extra, 440., 23.),
        text(15.2, 400, 0.).leading(1.5),
        c(0xf5f5f5, 0.56),
        false,
    );
    let close = Rect::new(1168.6, panel.y + 39.4, 33.6, 33.6);
    let close_color = out.chrome(
        close,
        16.8,
        state.focus == Target::CloseDialog,
        false,
        state.pressed.as_ref() == Some(&Target::CloseDialog),
    );
    out.icon(assets, "close", close.inset(7.6), close_color);
    out.control(Target::CloseDialog, close, false);
    let favorite = state
        .preferences
        .favorites(&state.inventory.apps)
        .iter()
        .any(|item| item == id);
    let hidden = state.preferences.hidden.iter().any(|item| item == id);
    let mut labels = vec![
        if favorite {
            language.t("removeFavorite")
        } else {
            language.t("addFavorite")
        },
        language.t(if hidden {
            "showInLumaglass"
        } else {
            "hideInLumaglass"
        }),
    ];
    if removal.is_some() {
        labels.push(
            language.t(
                if state.management.active()
                    && state
                        .management
                        .target
                        .as_ref()
                        .is_some_and(|target| target.id == id)
                {
                    "uninstallStatus"
                } else {
                    "uninstallApp"
                },
            ),
        );
    }
    for (index, label) in labels.into_iter().enumerate() {
        let rect = Rect::new(
            717.8,
            panel.y + 137.9 + extra + index as f32 * 76.8,
            484.4,
            67.2,
        );
        row(
            out,
            state,
            assets,
            Row {
                target: Target::Option(index),
                rect,
                title: label,
                icon: Some(if index == 0 {
                    if favorite { "heart-filled" } else { "heart" }
                } else if index == 2 {
                    "close"
                } else if hidden {
                    "eye"
                } else {
                    "eye-off"
                }),
                value: None,
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
    let line_y = panel.y + panel.h - 53.2;
    out.shape(Rect::new(717.8, line_y, 484.4, 1.), 0., c(0xffffff, 0.04));
    let shortcut = text(10.4, 400, 0.);
    let select_width = assets.width(language.t("selectClose"), shortcut);
    let close_width = assets.width(language.t("close"), shortcut);
    let start = 960. - (23. + 6.4 + select_width + 15. + 29. + 6.4 + close_width) / 2.;
    let back = start + 29.4 + select_width + 15.;
    keycap(out, "OK", start, line_y + 15., 23., true);
    out.text(
        language.t("selectClose"),
        Rect::new(start + 29.4, line_y + 18., select_width + 1., 13.),
        text(10.4, 400, 0.),
        c(0xe0e0e0, 0.35),
        false,
    );
    keycap(out, "Back", back, line_y + 15., 29., true);
    out.text(
        language.t("close"),
        Rect::new(back + 35.4, line_y + 18., close_width + 1., 13.),
        text(10.4, 400, 0.),
        c(0xe0e0e0, 0.35),
        false,
    );
}

fn removal_dialog(out: &mut Scene, state: &State, id: &str, title: &str, confirmation: bool) {
    use crate::features::apps::management::Phase;
    let language = state.preferences.language;
    let panel = Rect::new(600., 235., 720., 610.);
    out.modal = Some(panel);
    out.shadow(panel, 27.2, 120., 25., 0.47);
    out.draw.push(Draw::RoundedGradient(
        panel,
        27.2,
        c(0x34383b, 0.96),
        c(0x24282d, 0.988),
    ));
    out.border(panel, 27.2, 1., c(0xffffff, 0.12));
    out.text(
        title,
        Rect::new(638., 270., 644., 150.),
        text(28., 550, -0.02).leading(1.3),
        WHITE,
        false,
    );
    out.text(
        id,
        Rect::new(638., 430., 644., 56.),
        text(16.8, 400, 0.).leading(1.5),
        c(0xf5f5f5, 0.6),
        false,
    );
    out.text(
        language.t(if confirmation {
            "uninstallConfirmText"
        } else if !state.webos && state.management.phase == Phase::Removed {
            "uninstallPreviewDone"
        } else {
            state.management.status_key()
        }),
        Rect::new(638., 507., 644., 104.),
        text(20.8, 500, 0.).leading(1.4),
        WHITE,
        false,
    );
    out.text(
        language.t(if !state.webos {
            "uninstallPreview"
        } else if confirmation || !state.management.active() {
            "uninstallConfirmNote"
        } else {
            "uninstallCloseNote"
        }),
        Rect::new(638., 625., 644., 84.),
        text(16.8, 400, 0.).leading(1.5),
        c(0xf5f5f5, 0.6),
        false,
    );
    let check_enabled = !state.management.busy
        && matches!(state.management.phase, Phase::Verifying | Phase::NeedsCheck);
    let buttons = if confirmation {
        [
            (Target::RemovalCancel, "cancel", true),
            (Target::RemovalConfirm, "uninstallApp", true),
        ]
    } else {
        [
            (Target::CloseDialog, "close", true),
            (Target::RemovalCheck, "uninstallCheck", check_enabled),
        ]
    };
    for (index, (target, label, enabled)) in buttons.into_iter().enumerate() {
        let rect = Rect::new(638. + index as f32 * 338., 735., 306., 60.);
        let focused = state.focus == target;
        out.shape(rect, 12., c(0xffffff, if focused { 0.12 } else { 0.025 }));
        out.border(
            rect,
            12.,
            if focused { 2. } else { 1. },
            if focused {
                out.accent_color(c(0xffffff, 0.9))
            } else {
                c(0xffffff, 0.1)
            },
        );
        out.text(
            language.t(label),
            Rect::new(rect.x + 14., rect.y + 19., rect.w - 28., 26.),
            text(18.4, 500, 0.),
            c(0xffffff, if enabled { 1. } else { 0.35 }),
            true,
        );
        out.control(target, rect, false);
    }
}

fn keyboard(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(350., 290., 1220., 530.);
    out.modal = Some(panel);
    out.shadow(panel, 27.2, 90., 20., 0.45);
    out.draw.push(Draw::RoundedGradient(
        panel,
        27.2,
        c(0x34383b, 0.98),
        c(0x24282d, 1.),
    ));
    out.border(panel, 27.2, 1., c(0xffffff, 0.12));
    out.text(
        match state.keyboard_field {
            TextField::ProfileName => language.t("profileName"),
            TextField::Location => language.t("cityPostcode"),
            TextField::Name => language.t(
                if state.preferences.home.greeting_mode
                    == crate::features::home::GreetingMode::Static
                {
                    "homeStaticGreeting"
                } else {
                    "greetingName"
                },
            ),
            TextField::InputName => language.t("inputName"),
            TextField::PhotoFolder => language.t("photoFolder"),
            TextField::PhotoUrl => language.t("photoUrl"),
            TextField::Apps => language.t("findApp"),
        },
        Rect::new(389., 324., 1000., 44.),
        text(37.6, 500, -0.055).leading(1.1),
        WHITE,
        false,
    );
    let close = Rect::new(1497., 328., 33.6, 33.6);
    out.icon(assets, "close", close.inset(7.6), WHITE);
    out.control(Target::CloseDialog, close, false);
    if state.focus == Target::CloseDialog {
        out.border(close, 16.8, 2., out.accent_color(WHITE));
    }
    out.shape(Rect::new(389., 390., 1142., 56.), 28., c(0x171c20, 0.72));
    out.icon(
        assets,
        "search",
        Rect::new(409., 408., 20., 20.),
        c(0xeeeeee, 0.73),
    );
    out.text(
        format!("{}|", state.editing_text()),
        Rect::new(446., 406., 1058., 25.),
        text(20.8, 400, -0.02),
        WHITE,
        false,
    );
    for (index, key) in KEYS.iter().enumerate() {
        let label = match *key {
            "Space" => language.t("space"),
            "Delete" => language.t("delete"),
            "Clear" => language.t("clear"),
            "Done" => language.t("done"),
            _ => key,
        };
        let rect = Rect::new(
            389. + (index % 10) as f32 * 115.6,
            474. + (index / 10) as f32 * 74.,
            101.6,
            60.,
        );
        let focused = state.focus == Target::Key(index);
        out.pill(rect, focused, false);
        let mut style = text(18.4, 550, -0.02);
        let width = assets.width(label, style);
        if width > rect.w - 12. {
            style.size *= (rect.w - 12.) / width;
        }
        out.text(
            label,
            Rect::new(rect.x, rect.y + 18., rect.w, 25.),
            style,
            if focused { c(0x2c2c2c, 1.) } else { WHITE },
            true,
        );
        out.control(Target::Key(index), rect, false);
    }
}

fn toast(out: &mut Scene, state: &State, assets: &Assets, notice: &str) {
    let start = out.draw.len();
    let style = text(16., 400, 0.).leading(1.5);
    let width = (assets.width(notice, style) + 118.).clamp(240., 1856.);
    let height = 34. + assets.lines(notice, style, width - 100.) as f32 * 24.;
    let rect = Rect::new(960. - width / 2., 1080. - 51.2 - height, width, height);
    out.shadow(rect, 16., 40., 8., 0.33);
    out.shape(rect, 16., c(0xededed, 0.96));
    out.border(rect, 16., 1., c(0xffffff, 0.5));
    out.icon(
        assets,
        "sparkles",
        Rect::new(rect.x + 20., rect.y + (height - 20.8) / 2., 20.8, 20.8),
        c(0x727272, 1.),
    );
    out.text(
        notice,
        Rect::new(rect.x + 53.6, rect.y + 17., width - 100., height - 34.),
        text(16., 400, 0.).leading(1.5),
        c(0x303030, 1.),
        false,
    );
    let close = Rect::new(rect.x + width - 33., rect.y + (height - 20.) / 2., 20., 20.);
    out.icon(assets, "close", close.inset(1.2), c(0x6b6b6b, 1.));
    if state.focus == Target::DismissNotice {
        out.border(close.inset(-2.), 5., 2., c(0x303030, 1.));
    }
    if state.overlay.is_none() {
        out.control(Target::DismissNotice, close, false);
    }
    if !state.preferences.reduced_motion
        && let Some((_, opened)) = &state.notice
    {
        out.fade(start, (opened.elapsed().as_secs_f32() / 0.12).min(1.), 1.);
    }
}

/// A corner-less, borderless fill of the whole logical screen.
fn full_screen_fill(draw: &Draw) -> bool {
    let whole = |r: &Rect| r.x <= 0. && r.y <= 0. && r.x + r.w >= 1920. && r.y + r.h >= 1080.;
    match draw {
        Draw::Shape(rect, radius, border, _) => whole(rect) && *radius <= 0. && *border <= 0.,
        Draw::RoundedGradient(rect, radius, ..) => whole(rect) && *radius <= 0.,
        _ => false,
    }
}
