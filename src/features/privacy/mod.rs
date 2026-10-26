//! Read-only firmware settings and consent records. No service-activity inference.
use serde::Serialize;
use serde_json::Value;

/// Fixed query order is also the remote-navigation order. No arbitrary key requests.
pub const FIELDS: [(&str, &str, bool); 9] = [
    ("livePlus", "privacyLivePlus", false),
    ("lmt", "privacyLimitTracking", false),
    ("acrAllowed", "privacyAcrConsent", true),
    ("acrOnAllowed", "privacyAcrMaster", true),
    ("customAdAllowed", "privacyPersonalAds", true),
    ("customadsAllowed", "privacyPersonalAdsExtra", true),
    ("cookiesAllowed", "privacyCookies", true),
    ("remoteDiagAllowed", "privacyDiagnostics", true),
    ("thirdPartySharingAllowed", "privacySharing", true),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub enum Observation {
    On,
    Off,
    Denied,
    Unavailable,
    #[default]
    Unknown,
}

impl Observation {
    fn parse(value: &Value) -> Self {
        match value.as_str() {
            Some("on") => Self::On,
            Some("off") => Self::Off,
            Some("denied") => Self::Denied,
            Some("unavailable") => Self::Unavailable,
            _ => Self::Unknown,
        }
    }
    pub fn label(self, consent: bool) -> &'static str {
        match self {
            Self::On if consent => "privacyGranted",
            Self::Off if consent => "privacyNotGranted",
            Self::On => "stateOn",
            Self::Off => "stateOff",
            Self::Denied => "infoDenied",
            Self::Unavailable => "infoUnavailable",
            Self::Unknown => "infoUnknown",
        }
    }
}

#[derive(Default, Serialize)]
pub struct Report {
    pub values: [Observation; FIELDS.len()],
}

impl Report {
    pub fn from_reply(reply: &Value) -> Result<Self, String> {
        if reply["schema"].as_u64() != Some(1) || !reply["values"].is_object() {
            return Err("Invalid privacy observation reply".into());
        }
        Ok(Self {
            values: std::array::from_fn(|index| {
                Observation::parse(&reply["values"][FIELDS[index].0])
            }),
        })
    }
}

#[derive(Default)]
pub struct Privacy {
    pub report: Option<Report>,
    pub loading: bool,
    pub failed: bool,
    generation: u64,
}

impl Privacy {
    pub fn request(&mut self) -> Option<u64> {
        if self.loading {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.loading = true;
        self.failed = false;
        Some(self.generation)
    }
    pub fn accept(&mut self, generation: u64, result: Result<Report, String>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        match result {
            Ok(report) => {
                self.report = Some(report);
                self.failed = false;
            }
            Err(_) => {
                // Do not print consent payloads or caller diagnostics containing private values.
                eprintln!("Lumaglass privacy: observation failed");
                self.failed = true;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn absent_or_unrecognised_observations_never_become_off() {
        let report = Report::from_reply(&json!({"schema": 1, "values": {
            "livePlus": "on", "lmt": "off", "acrAllowed": false,
            "acrOnAllowed": "denied", "customAdAllowed": 0,
            "customadsAllowed": "unavailable"
        }}))
        .unwrap();
        assert_eq!(report.values[0], Observation::On);
        assert_eq!(report.values[1], Observation::Off);
        assert_eq!(report.values[2], Observation::Unknown);
        assert_eq!(report.values[3], Observation::Denied);
        assert_eq!(report.values[4], Observation::Unknown);
        assert_eq!(report.values[5], Observation::Unavailable);
        assert_eq!(report.values[8], Observation::Unknown);
        assert!(Report::from_reply(&json!({"schema": 2, "values": {}})).is_err());
    }

    #[test]
    fn refresh_coalesces_and_rejects_late_results() {
        let mut state = Privacy::default();
        let first = state.request().unwrap();
        assert!(state.request().is_none());
        assert!(!state.accept(first + 1, Ok(Report::default())));
        assert!(state.loading);
        assert!(state.accept(first, Ok(Report::default())));
        let next = state.request().unwrap();
        assert!(!state.accept(first, Err("old failure".into())));
        assert!(state.accept(next, Err("current failure".into())));
        assert!(state.failed);
        assert!(state.report.is_some());
    }
}
