//! Short, demand-driven focus lift; no animation work after settling.
use crate::state::{Rect, Target};
use std::time::{Duration, Instant};
const DURATION: Duration = Duration::from_millis(140);
#[derive(Default)]
pub struct Motion {
    current: Option<Target>,
    previous: Option<Target>,
    started: Option<Instant>,
}
impl Motion {
    pub fn update(&mut self, target: &Target, reduced: bool, now: Instant) -> bool {
        if self.current.as_ref() != Some(target) {
            self.previous = self.current.replace(target.clone());
            self.started = (!reduced && self.previous.is_some()).then_some(now);
            return true;
        }
        if reduced {
            self.started = None;
            self.previous = None;
            return false;
        }
        if self
            .started
            .is_some_and(|s| now.saturating_duration_since(s) >= DURATION)
        {
            self.started = None;
            self.previous = None;
            return true;
        }
        self.active()
    }
    pub fn active(&self) -> bool {
        self.started.is_some()
    }
    pub fn amount(&self, target: &Target) -> f32 {
        let t = self.started.map_or(1., |s| {
            (s.elapsed().as_secs_f32() / DURATION.as_secs_f32()).clamp(0., 1.)
        });
        let ease = 1. - (1. - t).powi(3);
        if self.current.as_ref() == Some(target) {
            ease
        } else if self.previous.as_ref() == Some(target) {
            1. - ease
        } else {
            0.
        }
    }
}
pub fn lift(rect: Rect, amount: f32, pressed: bool, card: bool) -> Rect {
    let scale = if pressed {
        0.985
    } else {
        1. + amount * if card { 0.045 } else { 0.018 }
    };
    Rect::new(
        rect.x - rect.w * (scale - 1.) / 2.,
        rect.y - rect.h * (scale - 1.) / 2.,
        rect.w * scale,
        rect.h * scale,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motion_settles_and_reduced_motion_schedules_no_frames() {
        let mut m = Motion::default();
        let now = Instant::now();
        assert!(m.update(&Target::Nav(0), false, now));
        assert!(!m.active());
        assert!(m.update(&Target::Nav(1), false, now));
        assert!(m.active());
        assert!(m.update(&Target::Nav(1), false, now + DURATION));
        assert!(!m.active());
        assert!(!m.update(&Target::Nav(1), false, now + DURATION));
        m.update(&Target::Nav(2), true, now + DURATION);
        assert!(!m.active());
    }
    #[test]
    fn press_compresses_around_a_stable_hit_rectangle() {
        let r = Rect::new(100., 200., 800., 72.);
        let p = lift(r, 1., true, false);
        assert!(p.w < r.w);
        assert!(
            (p.center().0 - r.center().0).abs() < 0.01
                && (p.center().1 - r.center().1).abs() < 0.01
        );
        let f = lift(r, 1., false, false);
        assert!(f.w > r.w);
        assert!(
            (f.center().0 - r.center().0).abs() < 0.01
                && (f.center().1 - r.center().1).abs() < 0.01
        );
    }
}
