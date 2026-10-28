//! Native system-saver configuration and firmware-qualified transactions.
use crate::{
    i18n::Language,
    state::{Effect, State, Subpage, Target},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
pub const ID: &str = "com.webos.app.screensaver";
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: u8,
    pub photos: bool,
    pub language: Language,
    pub hour_24: bool,
    pub images: Vec<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            schema: 1,
            photos: false,
            language: Language::En,
            hour_24: true,
            images: vec![],
        }
    }
}
impl Config {
    pub fn valid(&self) -> bool {
        self.schema == 1
            && self.images.len() <= 12
            && self.images.iter().all(|id| {
                crate::features::wallpapers::valid_id(id)
                    && !id.bytes().any(|c| c.is_ascii_uppercase())
            })
            && (self.photos != self.images.is_empty())
            && self
                .images
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                == self.images.len()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    Inspect,
    Apply,
    Restore,
    Request,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Intent {
    pub command: Command,
    pub config: Config,
    pub photo_root: Option<PathBuf>,
    pub plan: Option<String>,
}
#[derive(Default, Serialize)]
pub struct Session {
    pub loading: bool,
    pub confirming: bool,
    pub report: Option<Report>,
    generation: u64,
}
#[derive(Serialize)]
pub struct Report {
    pub status: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub sdk: Option<String>,
    pub plan: Option<String>,
    pub can_restore: bool,
}
impl Report {
    pub fn preview() -> Self {
        Self {
            status: "unavailable".into(),
            model: None,
            firmware: None,
            sdk: None,
            plan: None,
            can_restore: false,
        }
    }
    pub fn parse(value: &Value) -> Result<Self, String> {
        let status = value["status"]
            .as_str()
            .filter(|s| {
                [
                    "ready",
                    "owned",
                    "restored",
                    "restored-pending",
                    "root-required",
                    "unverified",
                    "conflict",
                    "expired",
                    "changed",
                    "unconfirmed",
                    "unavailable",
                    "requested",
                ]
                .contains(s)
            })
            .ok_or("Invalid saver status")?;
        if value["schema"].as_u64() != Some(1)
            || value["id"].as_str() != Some(ID)
            || value["returnValue"].as_bool() != Some(true)
        {
            return Err("Invalid saver reply".into());
        }
        let text = |key: &str| {
            value[key]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 80 && !s.chars().any(char::is_control))
                .map(str::to_owned)
        };
        let plan = value["plan"]
            .as_str()
            .filter(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            })
            .map(str::to_owned);
        Ok(Self {
            status: status.into(),
            model: text("model"),
            firmware: text("firmware"),
            sdk: text("sdk"),
            plan,
            can_restore: matches!(status, "owned" | "restored-pending")
                && value["canRestore"].as_bool() == Some(true),
        })
    }
    pub fn can_apply(&self) -> bool {
        self.status == "ready"
            && self.plan.is_some()
            && self.model.is_some()
            && self.firmware.is_some()
            && self.sdk.is_some()
    }
    pub fn key(&self) -> &'static str {
        match self.status.as_str() {
            "ready" => "infoAvailable",
            "owned" => "systemSaverOwned",
            "restored" => "systemVisibilityRestored",
            "restored-pending" | "unconfirmed" => "systemVisibilityPending",
            "unverified" => "systemVisibilityUnverified",
            "conflict" | "changed" => "systemVisibilityConflict",
            "root-required" => "infoDenied",
            "requested" => "systemSaverRequested",
            _ => "infoUnavailable",
        }
    }
}
impl Session {
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.loading = false;
        self.confirming = false;
        self.report = None;
    }
}
impl State {
    pub fn system_saver_config(&self) -> Config {
        Config {
            schema: 1,
            photos: self.preferences.screensaver.system_photos,
            language: self.preferences.language,
            hour_24: self.preferences.hour_24,
            images: if self.preferences.screensaver.system_photos {
                self.wallpapers
                    .photos
                    .iter()
                    .take(12)
                    .map(|p| p.id.clone())
                    .collect()
            } else {
                vec![]
            },
        }
    }
    pub fn system_saver_request(&mut self, command: Command) -> Vec<Effect> {
        if self.subpage != Some(Subpage::SystemScreensaver) || self.system_saver.loading {
            return vec![];
        }
        if command == Command::Apply
            && (!self.system_saver.confirming
                || !self
                    .system_saver
                    .report
                    .as_ref()
                    .is_some_and(Report::can_apply))
        {
            return vec![];
        }
        if command == Command::Restore
            && !self
                .system_saver
                .report
                .as_ref()
                .is_some_and(|r| r.can_restore)
        {
            return vec![];
        }
        let config = self.system_saver_config();
        if matches!(command, Command::Apply | Command::Inspect) && !config.valid() {
            self.notify(self.preferences.language.t("systemSaverNoPhotos"));
            return vec![];
        }
        let intent = Intent {
            command,
            photo_root: config.photos.then(|| self.wallpapers.root.clone()),
            config,
            plan: self
                .system_saver
                .report
                .as_ref()
                .and_then(|r| r.plan.clone()),
        };
        let session = &mut self.system_saver;
        session.generation = session.generation.wrapping_add(1);
        session.loading = true;
        session.confirming = false;
        if command == Command::Inspect {
            session.report = None;
        }
        self.dirty = true;
        vec![Effect::SystemScreensaver {
            generation: session.generation,
            intent,
        }]
    }
    pub fn system_saver_accept(&mut self, generation: u64, result: Result<Report, String>) {
        let session = &mut self.system_saver;
        if !session.loading || generation != session.generation {
            return;
        }
        session.loading = false;
        session.confirming = false;
        session.report = Some(result.unwrap_or_else(|_| Report::preview()));
        if self.subpage == Some(Subpage::SystemScreensaver) {
            self.focus = Target::SystemSaver(0);
            self.scroll = 0.;
        }
        self.dirty = true;
    }
    pub fn system_saver_action(&mut self, index: usize) -> Vec<Effect> {
        if self.subpage != Some(Subpage::SystemScreensaver)
            || !self.target_enabled(&Target::SystemSaver(index))
        {
            return vec![];
        }
        match index {
            0 => return self.system_saver_request(Command::Inspect),
            1 => {
                self.preferences.screensaver.system_photos =
                    !self.preferences.screensaver.system_photos;
                self.system_saver.invalidate();
                return vec![Effect::Save];
            }
            2 => return self.system_saver_request(Command::Request),
            3 => return self.system_saver_request(Command::Restore),
            4 => {
                if self.system_saver.confirming {
                    return self.system_saver_request(Command::Apply);
                }
                self.system_saver.confirming = true;
                self.focus = Target::SystemSaver(5);
            }
            5 => {
                self.system_saver.confirming = false;
                self.focus = Target::SystemSaver(0);
            }
            _ => {}
        }
        self.dirty = true;
        vec![]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confirmation_defaults_to_cancel_and_style_invalidates_the_plan() {
        let mut state = State::new(Default::default());
        state.webos = true;
        state.subpage = Some(Subpage::SystemScreensaver);
        state.system_saver.report=Some(Report::parse(&serde_json::json!({"schema":1,"id":ID,"returnValue":true,"status":"ready","model":"fixture","firmware":"1","sdk":"10","plan":"a".repeat(64)})).unwrap());
        assert!(state.system_saver_action(4).is_empty());
        assert!(state.system_saver.confirming);
        assert_eq!(state.focus, Target::SystemSaver(5));
        assert!(!state.target_enabled(&Target::SystemSaver(1)));
        assert!(state.system_saver_action(5).is_empty());
        assert!(!state.system_saver.confirming);
        assert_eq!(state.system_saver_action(1), vec![Effect::Save]);
        assert!(state.preferences.screensaver.system_photos);
        assert!(!state.preferences.screensaver.photos);
        assert!(state.system_saver.report.is_none());
        assert!(!state.target_enabled(&Target::SystemSaver(4)));
    }
    #[test]
    fn late_inspection_cannot_restore_an_invalidated_capability() {
        let mut state = State::new(Default::default());
        state.subpage = Some(Subpage::SystemScreensaver);
        let effects = state.system_saver_request(Command::Inspect);
        let Effect::SystemScreensaver { generation, .. } = effects[0] else {
            panic!()
        };
        state.system_saver.invalidate();
        state.system_saver_accept(generation, Ok(Report::preview()));
        assert!(state.system_saver.report.is_none());
        assert!(!state.system_saver.loading);
    }
    #[test]
    fn configuration_is_bounded_and_cannot_escape_photo_root() {
        let mut c = Config::default();
        assert!(c.valid());
        c.photos = true;
        assert!(!c.valid());
        c.images = vec!["personal-1234567890abcdef".into()];
        assert!(c.valid());
        c.images.push(c.images[0].clone());
        assert!(!c.valid());
        c.images = vec!["../private".into()];
        assert!(!c.valid());
        c.images = vec!["personal-1234567890abcdeF".into()];
        assert!(!c.valid());
    }
    #[test]
    fn reply_requires_saver_identity_and_complete_context() {
        let raw = serde_json::json!({"schema":1,"id":ID,"returnValue":true,"status":"ready","plan":"a".repeat(64)});
        assert!(!Report::parse(&raw).unwrap().can_apply());
        let mut raw = raw;
        raw["id"] = serde_json::json!("com.webos.app.home");
        assert!(Report::parse(&raw).is_err());
    }
}
