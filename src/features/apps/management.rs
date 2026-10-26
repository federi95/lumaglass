//! One-shot removals followed only by bounded read-only verification.
use crate::model::valid_app_id;
use serde::Serialize;
use serde_json::Value;
use std::time::{Duration, Instant};

pub fn protected(id: &str) -> bool {
    matches!(
        id,
        "org.lumaglass.app"
            | "com.ecklf.luma"
            | "com.luma.native"
            | "com.luma.launcher"
            | "com.luma.launcher.native"
            | "com.webos.app.home"
            | "org.webosbrew.hbchannel"
            | "org.webosbrew.inputhook"
            | "com.webos.app.livetv"
            | "com.webos.app.settings"
            | "com.palm.app.settings"
            | "com.webos.app.screensaver"
            | "com.webos.app.inputpicker"
            | "com.webos.app.externalinput"
            | "com.tvweb.dashboard"
            | "com.tvweb.home"
            | "com.webos.app.browser"
            | "com.palm.app.browser"
            | "com.webos.app.firstuse"
            | "com.webos.app.eula"
            | "com.webos.app.container"
            | "com.webos.app.inputcommon"
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Route {
    Store,
    Developer,
    Preview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RemovalTarget {
    pub id: String,
    pub folder: String,
    pub version: Option<String>,
    pub package: Option<String>,
    pub route: Route,
}

impl RemovalTarget {
    pub fn from_metadata(id: &str, value: &Value) -> Option<Self> {
        if !valid_app_id(id)
            || protected(id)
            || crate::scroll_preview::is_placeholder(id)
            || value["removable"].as_bool() != Some(true)
            || value["systemApp"].as_bool() != Some(false)
        {
            return None;
        }
        let folder = value["folderPath"].as_str()?;
        let route = if folder == format!("/media/developer/apps/usr/palm/applications/{id}") {
            Route::Developer
        } else if folder == format!("/media/cryptofs/apps/usr/palm/applications/{id}") {
            Route::Store
        } else {
            return None;
        };
        let version = match value.get("version") {
            None | Some(Value::Null) => None,
            Some(Value::String(text))
                if !text.is_empty() && text.len() <= 64 && !text.chars().any(char::is_control) =>
            {
                Some(text.clone())
            }
            _ => return None,
        };
        let package = match value.get("packageId") {
            None | Some(Value::Null) => None,
            Some(Value::String(id)) if valid_app_id(id) && !protected(id) => Some(id.clone()),
            _ => return None,
        };
        Some(Self {
            id: id.into(),
            folder: folder.into(),
            version,
            package,
            route,
        })
    }
    pub fn preview(id: &str) -> Self {
        Self {
            id: id.into(),
            folder: String::new(),
            version: None,
            package: None,
            route: Route::Preview,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", content = "target", rename_all = "lowercase")]
pub enum Command {
    Remove(RemovalTarget),
    Verify(RemovalTarget),
}
impl Command {
    pub fn target(&self) -> &RemovalTarget {
        match self {
            Self::Remove(t) | Self::Verify(t) => t,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    Removed,
    Accepted,
    Refused,
    Present,
    Unknown,
}
impl Outcome {
    pub fn from_reply(reply: &Value, id: &str) -> Result<Self, String> {
        if reply["schema"].as_u64() != Some(1) || reply["id"].as_str() != Some(id) {
            return Err("Invalid app management reply".into());
        }
        Ok(match reply["status"].as_str() {
            Some("removed") => Self::Removed,
            Some("accepted") => Self::Accepted,
            Some("refused") => Self::Refused,
            Some("present") => Self::Present,
            _ => Self::Unknown,
        })
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Phase {
    #[default]
    Idle,
    Sending,
    Verifying,
    NeedsCheck,
    Removed,
    Refused,
}

#[derive(Default)]
pub struct Management {
    pub target: Option<RemovalTarget>,
    pub title: String,
    pub phase: Phase,
    pub busy: bool,
    generation: u64,
    next: Option<Instant>,
    deadline: Option<Instant>,
}
impl Management {
    pub fn active(&self) -> bool {
        matches!(
            self.phase,
            Phase::Sending | Phase::Verifying | Phase::NeedsCheck
        )
    }
    pub fn begin(&mut self, target: RemovalTarget, title: String) -> Option<(u64, Command)> {
        if self.active()
            || protected(&target.id)
            || !valid_app_id(&target.id)
            || crate::scroll_preview::is_placeholder(&target.id)
        {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.target = Some(target.clone());
        self.title = title;
        self.phase = Phase::Sending;
        self.busy = true;
        self.deadline = Some(Instant::now() + Duration::from_secs(30));
        Some((self.generation, Command::Remove(target)))
    }
    pub fn poll(&mut self) -> Option<(u64, Command)> {
        if self.phase != Phase::Verifying || self.busy {
            return None;
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.phase = Phase::NeedsCheck;
            return None;
        }
        if self.next.is_none_or(|next| Instant::now() >= next) {
            return self.check();
        }
        None
    }
    pub fn check(&mut self) -> Option<(u64, Command)> {
        if self.busy || !self.active() {
            return None;
        }
        let target = self.target.clone()?;
        self.busy = true;
        Some((self.generation, Command::Verify(target)))
    }
    /// True means absence was verified, never merely that a request was accepted.
    pub fn accept(
        &mut self,
        generation: u64,
        command: &Command,
        result: Result<Outcome, String>,
    ) -> bool {
        if generation != self.generation
            || self.target.as_ref() != Some(command.target())
            || !self.active()
        {
            return false;
        }
        self.busy = false;
        match result.unwrap_or(Outcome::Unknown) {
            Outcome::Removed => {
                self.phase = Phase::Removed;
                return true;
            }
            Outcome::Refused if matches!(command, Command::Remove(_)) => {
                self.phase = Phase::Refused
            }
            _ => {
                self.phase = if self.deadline.is_some_and(|d| Instant::now() >= d) {
                    Phase::NeedsCheck
                } else {
                    Phase::Verifying
                };
                self.next = Some(Instant::now() + Duration::from_secs(1));
            }
        }
        false
    }
    pub fn status_key(&self) -> &'static str {
        match self.phase {
            Phase::Sending => "uninstallSending",
            Phase::Verifying => "uninstallVerifying",
            Phase::NeedsCheck => "uninstallUncertain",
            Phase::Removed => "uninstallDone",
            Phase::Refused => "uninstallRefused",
            Phase::Idle => "uninstallUncertain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn policy_requires_explicit_metadata_and_never_assumes_user_apps_are_removable() {
        let id = "org.example.player";
        let mut value = json!({"removable":true, "systemApp":false,
            "folderPath":format!("/media/cryptofs/apps/usr/palm/applications/{id}"), "version":"1.0.0"});
        assert!(RemovalTarget::from_metadata(id, &value).is_some());
        value["systemApp"] = Value::Null;
        assert!(RemovalTarget::from_metadata(id, &value).is_none());
        value["systemApp"] = json!(false);
        value["removable"] = json!("true");
        assert!(RemovalTarget::from_metadata(id, &value).is_none());
        for core in [
            "org.lumaglass.app",
            "org.webosbrew.hbchannel",
            "com.webos.app.home",
            "com.tvweb.home",
        ] {
            assert!(protected(core));
        }
    }

    #[test]
    fn ambiguous_results_only_generate_read_only_checks() {
        let mut state = Management::default();
        let target = RemovalTarget::preview("org.xbmc.kodi");
        let (generation, remove) = state.begin(target, "Kodi".into()).unwrap();
        assert!(!state.accept(generation, &remove, Err("timeout".into())));
        state.next = Some(Instant::now());
        let (_, verify) = state.poll().unwrap();
        assert!(matches!(verify, Command::Verify(_)));
        assert!(!state.accept(generation, &verify, Ok(Outcome::Present)));
        state.deadline = Some(Instant::now());
        assert!(state.poll().is_none());
        assert!(state.phase == Phase::NeedsCheck);
        assert!(
            state
                .begin(RemovalTarget::preview("org.xbmc.kodi"), "Kodi".into())
                .is_none()
        );
        let (_, check) = state.check().unwrap();
        assert!(matches!(check, Command::Verify(_)));
        assert!(state.accept(generation, &check, Ok(Outcome::Removed)));
        assert!(!state.accept(generation, &check, Ok(Outcome::Removed)));
    }
}
