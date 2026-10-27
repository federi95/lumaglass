use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Off,
    Startup,
    Five,
    Fifteen,
    Hour,
}
impl Mode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Startup,
            Self::Startup => Self::Five,
            Self::Five => Self::Fifteen,
            Self::Fifteen => Self::Hour,
            Self::Hour => Self::Off,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "stateOff",
            Self::Startup => "rotationStartup",
            Self::Five => "rotationFive",
            Self::Fifteen => "rotationFifteen",
            Self::Hour => "rotationHour",
        }
    }
    fn interval(self) -> Option<Duration> {
        match self {
            Self::Five => Some(Duration::from_secs(300)),
            Self::Fifteen => Some(Duration::from_secs(900)),
            Self::Hour => Some(Duration::from_secs(3600)),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub mode: Mode,
    pub shuffle: bool,
    pub selected: Vec<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            mode: Mode::Off,
            shuffle: false,
            selected: crate::model::BACKGROUNDS
                .iter()
                .map(|(id, _)| (*id).into())
                .collect(),
        }
    }
}
#[derive(Default)]
pub struct Session {
    last: Option<Instant>,
    elapsed: Duration,
    mode: Mode,
    started: bool,
    sequence: u64,
}
impl Session {
    pub fn due(&mut self, now: Instant, active: bool, mode: Mode) -> bool {
        if self.mode != mode {
            self.mode = mode;
            self.reset();
        }
        let previous = self.last.take();
        if !active {
            return false;
        }
        self.last = Some(now);
        let startup = !self.started;
        self.started = true;
        if startup && mode == Mode::Startup {
            return true;
        }
        if let Some(interval) = mode.interval() {
            if let Some(previous) = previous {
                self.elapsed += now.saturating_duration_since(previous);
            }
            if self.elapsed >= interval {
                self.elapsed = Duration::ZERO;
                return true;
            }
        }
        false
    }
    pub fn reset(&mut self) {
        self.last = None;
        self.elapsed = Duration::ZERO;
    }
    pub fn choose<'a>(
        &mut self,
        pool: &'a [String],
        current: &str,
        shuffle: bool,
    ) -> Option<&'a str> {
        let choices = pool
            .iter()
            .filter(|id| id.as_str() != current)
            .collect::<Vec<_>>();
        if choices.is_empty() {
            return None;
        }
        if shuffle {
            if self.sequence == 0 {
                self.sequence = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos() as u64)
                    | 1;
            }
            self.sequence ^= self.sequence << 13;
            self.sequence ^= self.sequence >> 7;
            self.sequence ^= self.sequence << 17;
            Some(choices[self.sequence as usize % choices.len()])
        } else {
            Some(
                pool.iter()
                    .position(|id| id == current)
                    .map_or(choices[0].as_str(), |index| {
                        pool[(index + 1) % pool.len()].as_str()
                    }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standby_does_not_accumulate_intervals_or_catch_up() {
        let now = Instant::now();
        let mut s = Session::default();
        assert!(!s.due(now, true, Mode::Five));
        assert!(!s.due(now + Duration::from_secs(100), true, Mode::Five));
        assert!(!s.due(now + Duration::from_secs(101), false, Mode::Five));
        assert!(!s.due(now + Duration::from_secs(10000), true, Mode::Five));
        assert!(s.due(now + Duration::from_secs(10200), true, Mode::Five));
        assert!(!s.due(now + Duration::from_secs(10200), true, Mode::Five));
    }
    #[test]
    fn selection_never_repeats_the_current_image_or_changes_a_single_image() {
        let mut s = Session::default();
        assert_eq!(s.choose(&[], "a", false), None);
        assert_eq!(s.choose(&["a".into()], "a", true), None);
        let pool = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(s.choose(&pool, "b", false), Some("c"));
        for _ in 0..20 {
            assert_ne!(s.choose(&pool, "b", true), Some("b"));
        }
    }
}
