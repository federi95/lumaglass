//! App-local idle presentation; leaves LG power and panel protection untouched.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Timeout {
    #[default]
    Off,
    One,
    Five,
    Ten,
}
impl Timeout {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::One,
            Self::One => Self::Five,
            Self::Five => Self::Ten,
            Self::Ten => Self::Off,
        }
    }
    fn duration(self) -> Option<Duration> {
        match self {
            Self::Off => None,
            Self::One => Some(Duration::from_secs(60)),
            Self::Five => Some(Duration::from_secs(300)),
            Self::Ten => Some(Duration::from_secs(600)),
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "stateOff",
            Self::One => "saverOne",
            Self::Five => "saverFive",
            Self::Ten => "saverTen",
        }
    }
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub timeout: Timeout,
    pub photos: bool,
}
pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Preferences, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or_default())
}
pub struct Session {
    pub active: bool,
    last_input: Instant,
    started: Instant,
    frame: Instant,
    pub wake_key: Option<sdl2::keyboard::Keycode>,
    pub wake_mouse: Option<sdl2::mouse::MouseButton>,
}
impl Default for Session {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            active: false,
            last_input: now,
            started: now,
            frame: now,
            wake_key: None,
            wake_mouse: None,
        }
    }
}
impl Session {
    pub fn consume_event(&mut self, event: &sdl2::event::Event, now: Instant) -> bool {
        use sdl2::event::Event;
        if let Event::KeyDown {
            keycode: Some(key), ..
        }
        | Event::KeyUp {
            keycode: Some(key), ..
        } = event
            && self.wake_key == Some(*key)
        {
            if matches!(event, Event::KeyUp { .. }) {
                self.wake_key = None;
            }
            self.last_input = now;
            return true;
        }
        if let Event::MouseButtonDown { mouse_btn, .. } | Event::MouseButtonUp { mouse_btn, .. } =
            event
            && self.wake_mouse == Some(*mouse_btn)
        {
            if matches!(event, Event::MouseButtonUp { .. }) {
                self.wake_mouse = None;
            }
            self.last_input = now;
            return true;
        }
        let activity = match event {
            Event::KeyDown {
                keycode: Some(_), ..
            }
            | Event::KeyUp {
                keycode: Some(_), ..
            }
            | Event::MouseButtonDown { .. }
            | Event::MouseButtonUp { .. }
            | Event::MouseWheel { .. }
            | Event::TextInput { .. } => true,
            Event::MouseMotion { xrel, yrel, .. } => *xrel != 0 || *yrel != 0,
            _ => false,
        };
        if activity && self.activity(now) {
            if let Event::KeyDown { keycode, .. } = event {
                self.wake_key = *keycode;
            }
            if let Event::MouseButtonDown { mouse_btn, .. } = event {
                self.wake_mouse = Some(*mouse_btn);
            }
            return true;
        }
        false
    }
    pub fn activity(&mut self, now: Instant) -> bool {
        self.last_input = now;
        std::mem::replace(&mut self.active, false)
    }
    pub fn tick(&mut self, now: Instant, eligible: bool, timeout: Timeout) -> bool {
        let before = self.active;
        if !eligible || timeout == Timeout::Off {
            self.activity(now);
        } else if !self.active
            && timeout
                .duration()
                .is_some_and(|d| now.saturating_duration_since(self.last_input) >= d)
        {
            self.active = true;
            self.started = now;
        }
        let redraw =
            self.active && now.saturating_duration_since(self.frame) >= Duration::from_secs(1);
        if redraw {
            self.frame = now;
        }
        before != self.active || redraw
    }
    pub fn elapsed(&self) -> u64 {
        self.started.elapsed().as_secs()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waking_key_is_consumed_through_repeats_and_release() {
        use sdl2::{
            event::Event,
            keyboard::{Keycode, Mod},
        };
        let now = Instant::now();
        let mut s = Session {
            active: true,
            ..Session::default()
        };
        let down = Event::KeyDown {
            timestamp: 0,
            window_id: 1,
            keycode: Some(Keycode::Return),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        let up = Event::KeyUp {
            timestamp: 0,
            window_id: 1,
            keycode: Some(Keycode::Return),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        assert!(s.consume_event(&down, now));
        assert!(!s.active);
        assert!(s.consume_event(&down, now));
        assert!(s.consume_event(&up, now));
        assert!(!s.consume_event(&down, now));
    }
    #[test]
    fn idle_only_activates_inside_the_focused_app_and_activity_dismisses() {
        let mut s = Session::default();
        let now = Instant::now();
        s.activity(now);
        assert!(!s.tick(now + Duration::from_secs(59), true, Timeout::One));
        assert!(s.tick(now + Duration::from_secs(60), true, Timeout::One));
        assert!(s.active);
        assert!(s.activity(now + Duration::from_secs(61)));
        assert!(!s.active);
        s.tick(now + Duration::from_secs(10000), false, Timeout::One);
        s.tick(now + Duration::from_secs(10001), true, Timeout::One);
        assert!(!s.active);
        s.tick(now + Duration::from_secs(20000), true, Timeout::Off);
        assert!(!s.active);
    }
}
