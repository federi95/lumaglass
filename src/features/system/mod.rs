//! Read-only observations, not permission promises for future TV mutations.
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub enum Access {
    Available,
    Denied,
    Unavailable,
    #[default]
    Unknown,
}

impl Access {
    pub fn key(self) -> &'static str {
        match self {
            Self::Available => "infoAvailable",
            Self::Denied => "infoDenied",
            Self::Unavailable => "infoUnavailable",
            Self::Unknown => "infoUnknown",
        }
    }
    fn parse(value: &Value) -> Self {
        match value.as_str() {
            Some("available") => Self::Available,
            Some("denied") => Self::Denied,
            Some("unavailable") => Self::Unavailable,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub enum Home {
    Lumaglass,
    Other,
    Stock,
    #[default]
    Unknown,
}
impl Home {
    pub fn key(self) -> &'static str {
        match self {
            Self::Lumaglass => "infoHomeLumaglass",
            Self::Other => "infoHomeOther",
            Self::Stock => "infoHomeStock",
            Self::Unknown => "infoUnknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub enum Installation {
    Developer,
    Production,
    System,
    #[default]
    Unknown,
}
impl Installation {
    pub fn key(self) -> &'static str {
        match self {
            Self::Developer => "infoInstallDeveloper",
            Self::Production => "infoInstallProduction",
            Self::System => "infoInstallSystem",
            Self::Unknown => "infoUnknown",
        }
    }
    pub fn from_path(path: &std::path::Path) -> Self {
        if path.starts_with("/media/developer/apps") {
            Self::Developer
        } else if path.starts_with("/media/cryptofs/apps") {
            Self::Production
        } else if path.starts_with("/usr/palm/applications") {
            Self::System
        } else {
            Self::Unknown
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Report {
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub sdk: Option<String>,
    pub root: Access,
    pub system_info: Access,
    pub home: Home,
    pub installation: Installation,
}

impl Report {
    pub fn from_reply(value: &Value, installation: Installation) -> Result<Self, String> {
        if value.get("schema").and_then(Value::as_u64) != Some(1) {
            return Err("Unsupported system-information reply".into());
        }
        let text = |key| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(|text| {
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(80)
                        .collect::<String>()
                })
                .filter(|text| !text.trim().is_empty())
        };
        Ok(Self {
            model: text("model"),
            firmware: text("firmware"),
            sdk: text("sdk"),
            root: Access::parse(&value["rootAccess"]),
            system_info: Access::parse(&value["systemAccess"]),
            home: match value["home"].as_str() {
                Some("lumaglass") => Home::Lumaglass,
                Some("other") => Home::Other,
                Some("stock") => Home::Stock,
                _ => Home::Unknown,
            },
            installation,
        })
    }
}

#[derive(Default, Serialize)]
pub struct Information {
    pub report: Option<Report>,
    pub loading: bool,
    pub failed: bool,
    pub generation: u64,
}

impl Information {
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
            Err(error) => {
                eprintln!("Lumaglass TV information: {error}");
                self.failed = true;
            }
        }
        true
    }
}
