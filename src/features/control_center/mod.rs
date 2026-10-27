//! Launcher-local quick controls, independent of Apple's system integration.
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Action {
    Standby,
    Network,
    Audio,
    Screensaver,
    Sleep,
    Search,
    Inputs,
    Recents,
    Settings,
}
impl Action {
    pub fn key(self) -> &'static str {
        match self {
            Self::Standby => "ccStandby",
            Self::Network => "ccNetwork",
            Self::Audio => "ccAudio",
            Self::Screensaver => "ccScreensaver",
            Self::Sleep => "ccSleepTimer",
            Self::Search => "shortcutSearch",
            Self::Inputs => "inputs",
            Self::Recents => "shortcutRecents",
            Self::Settings => "settings",
        }
    }
}
pub const SLEEP_MINUTES: [u16; 6] = [0, 10, 30, 60, 90, 120];
