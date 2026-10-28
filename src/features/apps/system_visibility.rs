//! Firmware-qualified global visibility: read-only preflight and ownership-scoped recovery.
use crate::{
    model::valid_app_id,
    state::{Effect, State, Subpage, Target},
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    Inspect,
    Restore,
    Hide,
}
#[derive(Default, Serialize)]
pub struct Session {
    pub loading: bool,
    pub confirming: bool,
    pub report: Option<Report>,
    generation: u64,
    id: Option<String>,
}
#[derive(Serialize)]
pub struct Report {
    pub id: String,
    pub status: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub sdk: Option<String>,
    pub manifest_hash: Option<String>,
    pub plan: Option<String>,
    pub can_restore: bool,
}
impl Session {
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.loading = false;
        self.report = None;
        self.confirming = false;
        self.id = None;
    }
}
impl Report {
    pub fn parse(id: &str, raw: &Value) -> Result<Self, String> {
        if !valid_app_id(id)
            || raw["schema"].as_u64() != Some(1)
            || raw["returnValue"].as_bool() != Some(true)
            || raw["id"].as_str() != Some(id)
        {
            return Err("Invalid system visibility reply".into());
        }
        let status = raw["status"]
            .as_str()
            .filter(|s| {
                [
                    "protected",
                    "root-required",
                    "not-system",
                    "conflict",
                    "unverified",
                    "ready",
                    "owned",
                    "restored",
                    "restored-pending",
                    "unavailable",
                    "unconfirmed",
                    "expired",
                    "changed",
                ]
                .contains(s)
            })
            .ok_or("Invalid visibility status")?;
        let text = |key: &str| {
            raw[key]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 80 && !s.chars().any(char::is_control))
                .map(str::to_owned)
        };
        let manifest_hash = raw["manifestHash"]
            .as_str()
            .filter(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            })
            .map(str::to_owned);
        Ok(Self {
            id: id.into(),
            status: status.into(),
            model: text("model"),
            firmware: text("firmware"),
            sdk: text("sdk"),
            plan: raw["plan"]
                .as_str()
                .filter(|s| {
                    s.len() == 64
                        && s.bytes()
                            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                })
                .map(str::to_owned),
            manifest_hash,
            can_restore: matches!(status, "owned" | "restored-pending")
                && raw["canRestore"].as_bool() == Some(true)
                && !super::management::protected(id),
        })
    }
    pub fn preview(id: &str) -> Self {
        Self {
            id: id.into(),
            status: "unavailable".into(),
            model: None,
            firmware: None,
            sdk: None,
            manifest_hash: None,
            plan: None,
            can_restore: false,
        }
    }
    pub fn can_hide(&self) -> bool {
        self.status == "ready"
            && self.plan.is_some()
            && self.manifest_hash.is_some()
            && self.model.is_some()
            && self.firmware.is_some()
            && self.sdk.is_some()
            && !super::management::protected(&self.id)
    }
    pub fn key(&self) -> &'static str {
        match self.status.as_str() {
            "protected" => "catalogProtected",
            "root-required" => "infoDenied",
            "not-system" => "catalogUser",
            "conflict" | "changed" => "systemVisibilityConflict",
            "owned" => "systemVisibilityOwned",
            "restored" => "systemVisibilityRestored",
            "restored-pending" | "unconfirmed" => "systemVisibilityPending",
            "ready" => "infoAvailable",
            "unverified" => "systemVisibilityUnverified",
            _ => "infoUnavailable",
        }
    }
}
impl State {
    pub fn system_visibility_report(&self) -> Option<&Report> {
        let Some(Subpage::ManagedApp(id)) = &self.subpage else {
            return None;
        };
        self.catalog
            .system_visibility
            .report
            .as_ref()
            .filter(|report| &report.id == id)
    }
    pub fn system_visibility_request(&mut self, command: Command) -> Vec<Effect> {
        let Some(Subpage::ManagedApp(id)) = self.subpage.clone() else {
            return vec![];
        };
        if self.catalog.system_visibility.loading
            || (command == Command::Restore
                && !self
                    .system_visibility_report()
                    .is_some_and(|r| r.can_restore))
        {
            return vec![];
        }
        let plan = if command == Command::Hide {
            if !self.catalog.system_visibility.confirming
                || !self
                    .system_visibility_report()
                    .is_some_and(Report::can_hide)
            {
                return vec![];
            }
            self.system_visibility_report().and_then(|r| r.plan.clone())
        } else {
            None
        };
        let session = &mut self.catalog.system_visibility;
        session.confirming = false;
        session.generation = session.generation.wrapping_add(1);
        session.loading = true;
        session.id = Some(id.clone());
        if command == Command::Inspect {
            session.report = None;
        }
        self.dirty = true;
        vec![Effect::SystemVisibility {
            generation: session.generation,
            id,
            command,
            plan,
        }]
    }
    pub fn system_visibility_accept(
        &mut self,
        generation: u64,
        id: &str,
        result: Result<Report, String>,
    ) {
        let session = &mut self.catalog.system_visibility;
        if !session.loading || generation != session.generation || session.id.as_deref() != Some(id)
        {
            return;
        }
        session.loading = false;
        session.report = Some(
            result
                .ok()
                .filter(|report| report.id == id)
                .unwrap_or_else(|| Report::preview(id)),
        );
        if self.focus == Target::CatalogSystemRestore
            && !self.target_enabled(&Target::CatalogSystemRestore)
        {
            self.focus = Target::CatalogSystemInspect;
        }
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests;
