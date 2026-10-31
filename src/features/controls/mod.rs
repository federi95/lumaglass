//! Fixed privileged intents; no arbitrary endpoints, paths or commands.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Command {
    QuickStatus,
    Standby,
    AudioMute {
        expected: bool,
        value: bool,
    },
    SleepTimer {
        expected: usize,
        value: usize,
    },
    PrivacySet {
        index: usize,
        expected: bool,
        value: bool,
    },
    PrivacyRestore {
        index: usize,
    },
    BlockerStatus,
    BlockerEnable,
    BlockerDisable,
}
impl Command {
    pub fn key(self) -> &'static str {
        match self {
            Self::QuickStatus => "infoRefresh",
            Self::Standby => "ccStandby",
            Self::AudioMute { .. } => "ccAudio",
            Self::SleepTimer { .. } => "ccSleepTimer",
            Self::PrivacySet { .. } => "controlPrivacyChange",
            Self::PrivacyRestore { .. } => "controlRestore",
            Self::BlockerStatus => "infoRefresh",
            Self::BlockerEnable => "blockerEnable",
            Self::BlockerDisable => "blockerDisable",
        }
    }
    pub fn privacy(self) -> bool {
        matches!(self, Self::PrivacySet { .. } | Self::PrivacyRestore { .. })
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Report {
    pub accepted: bool,
    pub network: String,
    pub ssid: Option<String>,
    pub volume: Option<u8>,
    pub muted: Option<bool>,
    pub power_active: bool,
    pub sleep_index: Option<usize>,
    pub schema: u8,
    pub verified: bool,
    pub blocker: String,
    pub restore: [bool; 2],
}
#[derive(Default)]
pub struct Controls {
    pub loading: bool,
    pub report: Report,
    generation: u64,
}
impl Controls {
    pub fn request(&mut self, command: Command) -> Option<(u64, Command)> {
        if self.loading {
            return None;
        }
        self.loading = true;
        self.generation = self.generation.wrapping_add(1);
        Some((self.generation, command))
    }
    pub fn accept(&mut self, generation: u64, result: Result<Report, String>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        match result {
            Ok(mut report) if report.schema == 1 => {
                let verified = report.verified;
                if report.network.is_empty() {
                    report.network = std::mem::take(&mut self.report.network);
                    report.ssid = self.report.ssid.take();
                    report.power_active = self.report.power_active;
                    report.sleep_index = report.sleep_index.or(self.report.sleep_index);
                    report.volume = report.volume.or(self.report.volume);
                    report.muted = report.muted.or(self.report.muted);
                }
                if report.blocker.is_empty() || report.blocker == "unknown" {
                    report.blocker = std::mem::take(&mut self.report.blocker);
                }
                self.report = report;
                verified
            }
            _ => false,
        }
    }
}
