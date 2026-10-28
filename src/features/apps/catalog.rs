//! Full read-only app catalog and reversible changes to launcher-owned visibility.
//! LG manifest/SAM mutation is unavailable until a firmware adapter is verified.
use super::management::{self, RemovalTarget};
use crate::{
    features::system::Access,
    model::{Inventory, valid_app_id},
    state::{Effect, Overlay, Page, State, Subpage, Target},
};
use serde::Serialize;
use serde_json::Value;

pub const PAGE_SIZE: usize = 20;
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub system: Option<bool>,
    pub visible: Option<bool>,
    pub protected: bool,
    pub removal: Option<RemovalTarget>,
}
#[derive(Default, Serialize)]
pub struct Report {
    pub preview: bool,
    pub access: Access,
    pub root: Access,
    pub sdk: Option<String>,
    pub apps: Vec<Entry>,
}
fn text(value: &Value, max: usize) -> Option<String> {
    value
        .as_str()
        .map(|s| {
            s.chars()
                .filter(|c| !c.is_control())
                .take(max)
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .filter(|s| !s.is_empty())
}
impl Report {
    pub fn from_reply(reply: &Value) -> Result<Self, String> {
        if reply["schema"].as_u64() != Some(1) || reply["returnValue"].as_bool() != Some(true) {
            return Err("Invalid app catalog".into());
        }
        let access = Access::parse(&reply["inventoryAccess"]);
        let mut report = Self {
            access,
            root: Access::parse(&reply["rootAccess"]),
            sdk: text(&reply["sdk"], 80),
            ..Self::default()
        };
        if !matches!(access, Access::Available) {
            return Ok(report);
        }
        let apps = reply["apps"]
            .as_array()
            .filter(|a| a.len() <= 1000)
            .ok_or("Invalid catalog inventory")?;
        let mut seen = std::collections::HashSet::new();
        for raw in apps {
            let id = raw["id"]
                .as_str()
                .filter(|id| valid_app_id(id))
                .ok_or("Invalid catalog identity")?;
            if !seen.insert(id.to_owned()) {
                return Err("Duplicate catalog identity".into());
            }
            let system_path = raw["folderPath"].as_str().is_some_and(|folder| {
                [
                    format!("/usr/palm/applications/{id}"),
                    format!("/media/system/apps/usr/palm/applications/{id}"),
                ]
                .contains(&folder.to_owned())
            });
            let system = match (raw["systemApp"].as_bool(), system_path) {
                (Some(false), true) => None,
                (Some(flag), _) => Some(flag),
                (None, true) => Some(true),
                _ => None,
            };
            report.apps.push(Entry {
                id: id.into(),
                title: text(&raw["title"], 120).unwrap_or_else(|| id.into()),
                system,
                visible: raw["visible"].as_bool(),
                protected: management::protected(id),
                removal: RemovalTarget::from_metadata(id, raw),
            });
        }
        if !seen.contains("com.webos.app.home") {
            return Err("Incomplete app catalog".into());
        }
        report
            .apps
            .sort_by_key(|app| (app.title.to_lowercase(), app.id.clone()));
        Ok(report)
    }
    pub fn preview(inventory: &Inventory) -> Self {
        let mut apps: Vec<_> = inventory
            .apps
            .iter()
            .map(|app| Entry {
                id: app.id.clone(),
                title: app.title.clone(),
                system: Some(app.id == "com.lgtvod.kr.app"),
                visible: Some(true),
                protected: management::protected(&app.id),
                removal: app.removal.clone(),
            })
            .collect();
        apps.push(Entry {
            id: "com.webos.app.home".into(),
            title: "LG Home".into(),
            system: Some(true),
            visible: Some(false),
            protected: true,
            removal: None,
        });
        apps.sort_by_key(|app| (app.title.to_lowercase(), app.id.clone()));
        Self {
            preview: true,
            apps,
            ..Self::default()
        }
    }
    pub fn available(&self) -> bool {
        self.preview || matches!(self.access, Access::Available)
    }
}
#[derive(Clone, Copy, Default, Serialize)]
pub enum Filter {
    #[default]
    All,
    System,
    Hidden,
}
impl Filter {
    pub fn key(self) -> &'static str {
        match self {
            Self::All => "catalogAll",
            Self::System => "catalogSystem",
            Self::Hidden => "catalogHidden",
        }
    }
    fn next(self) -> Self {
        match self {
            Self::All => Self::System,
            Self::System => Self::Hidden,
            Self::Hidden => Self::All,
        }
    }
}
struct Change {
    id: String,
    before: bool,
    after: bool,
}
#[derive(Default)]
pub struct Catalog {
    pub report: Option<Report>,
    pub system_visibility: super::system_visibility::Session,
    pub loading: bool,
    pub failed: bool,
    pub filter: Filter,
    pub page: usize,
    generation: u64,
    undo: Vec<Change>,
    return_scroll: f32,
}
impl Catalog {
    pub fn request(&mut self) -> Option<u64> {
        if self.loading {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.loading = true;
        self.failed = false;
        Some(self.generation)
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    fn current(&self) -> bool {
        !self.loading && !self.failed && self.report.as_ref().is_some_and(Report::available)
    }
}
impl State {
    pub fn catalog_request(&mut self) -> Vec<Effect> {
        let Some(generation) = self.catalog.request() else {
            return vec![];
        };
        self.catalog.system_visibility.invalidate();
        vec![Effect::AppCatalog { generation }]
    }
    pub fn catalog_accept(&mut self, generation: u64, result: Result<Report, String>) {
        if generation != self.catalog.generation || !self.catalog.loading {
            return;
        }
        self.catalog.loading = false;
        self.catalog.failed = result.is_err();
        if let Ok(mut report) = result {
            report
                .apps
                .retain(|app| !self.removed_apps.contains(&app.id));
            self.catalog.report = Some(report);
        }
        self.catalog.page = self
            .catalog
            .page
            .min(self.catalog_pages().saturating_sub(1));
        self.reconcile_focus();
        self.dirty = true;
    }
    pub fn catalog_entries(&self) -> Vec<&Entry> {
        let hidden: std::collections::HashSet<&str> =
            if matches!(self.catalog.filter, Filter::Hidden) {
                self.preferences.hidden.iter().map(String::as_str).collect()
            } else {
                std::collections::HashSet::new()
            };
        self.catalog
            .report
            .iter()
            .flat_map(|report| &report.apps)
            .filter(|app| match self.catalog.filter {
                Filter::All => true,
                Filter::System => app.system == Some(true),
                Filter::Hidden => hidden.contains(app.id.as_str()),
            })
            .collect()
    }
    pub fn catalog_pages(&self) -> usize {
        self.catalog_entries().len().div_ceil(PAGE_SIZE).max(1)
    }
    pub fn catalog_entry(&self, id: &str) -> Option<&Entry> {
        self.catalog
            .report
            .as_ref()?
            .apps
            .iter()
            .find(|app| app.id == id)
    }
    pub fn catalog_can_hide(&self, id: &str) -> bool {
        self.catalog.current()
            && self.preferences.hidden.len() < 1000
            && self.catalog_entry(id).is_some_and(|app| !app.protected)
            && self.inventory.apps.iter().any(|app| app.id == id)
    }
    pub fn catalog_removal(&self) -> Option<RemovalTarget> {
        let Some(Subpage::ManagedApp(id)) = &self.subpage else {
            return None;
        };
        if !self.catalog.current()
            || self
                .catalog
                .report
                .as_ref()
                .is_some_and(|report| !report.preview && !matches!(report.root, Access::Available))
        {
            return None;
        }
        self.catalog_entry(id)?
            .removal
            .clone()
            .filter(|target| (target.route == management::Route::Preview) != self.webos)
    }
    pub fn catalog_removal_status(&self) -> bool {
        let Some(Subpage::ManagedApp(id)) = &self.subpage else {
            return false;
        };
        self.management.active()
            && self
                .management
                .target
                .as_ref()
                .is_some_and(|target| &target.id == id)
    }
    pub fn catalog_rows(&self) -> Vec<Target> {
        if matches!(self.subpage, Some(Subpage::ManagedApp(_))) {
            let mut rows = vec![
                Target::CatalogBack,
                Target::CatalogToggle,
                Target::CatalogSystemInspect,
                Target::CatalogSystemRestore,
            ];
            if self.catalog.system_visibility.confirming {
                rows.push(Target::CatalogSystemCancel);
            }
            rows.extend([Target::CatalogLgHome, Target::CatalogRemove]);
            return rows;
        }
        let mut rows = vec![
            Target::CatalogRefresh,
            Target::CatalogFilter,
            Target::CatalogPrevious,
            Target::CatalogNext,
            Target::CatalogUndo,
            Target::CatalogRestoreAll,
        ];
        rows.extend(
            self.catalog_entries()
                .into_iter()
                .skip(self.catalog.page * PAGE_SIZE)
                .take(PAGE_SIZE)
                .map(|app| Target::CatalogApp(app.id.clone())),
        );
        rows
    }
    pub fn open_app_catalog(&mut self) -> Vec<Effect> {
        self.page = Page::Settings;
        self.section = crate::state::SYSTEM_SECTION;
        self.subpage = Some(Subpage::AppManagement);
        self.focus = Target::CatalogRefresh;
        self.scroll = 0.;
        self.dirty = true;
        self.catalog_request()
    }
    pub fn catalog_back(&mut self) {
        self.catalog.system_visibility.invalidate();
        if let Some(Subpage::ManagedApp(id)) = self.subpage.clone() {
            self.subpage = Some(Subpage::AppManagement);
            let visible = self
                .catalog_entries()
                .into_iter()
                .skip(self.catalog.page * PAGE_SIZE)
                .take(PAGE_SIZE)
                .any(|app| app.id == id);
            self.focus = if visible {
                Target::CatalogApp(id)
            } else {
                Target::CatalogFilter
            };
            self.scroll = if visible {
                self.catalog.return_scroll
            } else {
                0.
            };
        }
        self.dirty = true;
    }
    fn local_visibility(&mut self, ids: Vec<String>, hidden: bool) -> Vec<Effect> {
        let mut changes = vec![];
        for id in ids {
            let before = self.preferences.hidden.contains(&id);
            if before == hidden || (hidden && !self.catalog_can_hide(&id)) {
                continue;
            }
            self.preferences.hidden.retain(|saved| saved != &id);
            if hidden {
                self.preferences.hidden.push(id.clone());
            }
            changes.push(Change {
                id,
                before,
                after: hidden,
            });
        }
        if changes.is_empty() {
            return vec![];
        }
        self.catalog.undo = changes;
        self.catalog.page = self
            .catalog
            .page
            .min(self.catalog_pages().saturating_sub(1));
        self.home_scroll = 0.;
        self.reconcile_focus();
        self.dirty = true;
        vec![Effect::Save]
    }
    pub fn catalog_action(&mut self) -> Vec<Effect> {
        if self.overlay.is_some()
            || !matches!(
                self.subpage,
                Some(Subpage::AppManagement | Subpage::ManagedApp(_))
            )
        {
            return vec![];
        }
        self.dirty = true;
        match self.focus.clone() {
            Target::CatalogRefresh => return self.catalog_request(),
            Target::CatalogSystemCancel => {
                self.catalog.system_visibility.confirming = false;
                self.focus = Target::CatalogLgHome;
            }
            Target::CatalogLgHome => {
                if self.catalog.system_visibility.confirming {
                    return self.system_visibility_request(super::system_visibility::Command::Hide);
                }
                if self
                    .system_visibility_report()
                    .is_some_and(super::system_visibility::Report::can_hide)
                {
                    self.catalog.system_visibility.confirming = true;
                    self.focus = Target::CatalogSystemCancel;
                }
            }
            Target::CatalogSystemInspect => {
                return self.system_visibility_request(super::system_visibility::Command::Inspect);
            }
            Target::CatalogSystemRestore => {
                return self.system_visibility_request(super::system_visibility::Command::Restore);
            }
            Target::CatalogFilter => {
                self.catalog.filter = self.catalog.filter.next();
                self.catalog.page = 0;
                self.scroll = 0.;
            }
            Target::CatalogPrevious => {
                self.catalog.page = self.catalog.page.saturating_sub(1);
                self.scroll = 0.;
            }
            Target::CatalogNext => {
                self.catalog.page = (self.catalog.page + 1).min(self.catalog_pages() - 1);
                self.scroll = 0.;
            }
            Target::CatalogApp(id) if self.catalog_entry(&id).is_some() => {
                self.catalog.system_visibility.invalidate();
                self.catalog.return_scroll = self.scroll;
                self.subpage = Some(Subpage::ManagedApp(id));
                self.focus = Target::CatalogBack;
                self.scroll = 0.;
            }
            Target::CatalogBack => self.catalog_back(),
            Target::CatalogToggle => {
                if let Some(Subpage::ManagedApp(id)) = &self.subpage {
                    let id = id.clone();
                    let hide = !self.preferences.hidden.contains(&id);
                    return self.local_visibility(vec![id], hide);
                }
            }
            Target::CatalogRestoreAll => {
                return self.local_visibility(self.preferences.hidden.clone(), false);
            }
            Target::CatalogUndo => {
                let changes = std::mem::take(&mut self.catalog.undo);
                let mut changed = false;
                for change in changes {
                    if self.preferences.hidden.contains(&change.id) != change.after
                        || (change.before
                            && (self.preferences.hidden.len() >= 1000
                                || management::protected(&change.id)))
                    {
                        continue;
                    }
                    self.preferences.hidden.retain(|id| id != &change.id);
                    if change.before {
                        self.preferences.hidden.push(change.id);
                    }
                    changed = true;
                }
                self.home_scroll = 0.;
                self.reconcile_focus();
                if changed {
                    return vec![Effect::Save];
                }
            }
            Target::CatalogRemove => {
                if self.catalog_removal_status() {
                    self.overlay = Some(Overlay::RemovalStatus {
                        restore: Target::CatalogRemove,
                    });
                    self.focus = Target::RemovalCheck;
                } else if !self.management.active()
                    && let Some(target) = self.catalog_removal()
                {
                    let title = self
                        .catalog_entry(&target.id)
                        .map_or(target.id.clone(), |app| app.title.clone());
                    self.overlay = Some(Overlay::RemovalConfirm {
                        target,
                        title,
                        restore: Target::CatalogRemove,
                    });
                    self.focus = Target::RemovalCancel;
                }
            }
            _ => {}
        }
        vec![]
    }
    pub fn catalog_invalidate_removed(&mut self, id: &str) {
        self.catalog.system_visibility.invalidate();
        self.catalog.generation = self.catalog.generation.wrapping_add(1);
        self.catalog.loading = false;
        if let Some(report) = &mut self.catalog.report {
            report.apps.retain(|app| app.id != id);
        }
        self.catalog.undo.retain(|change| change.id != id);
        if self.subpage == Some(Subpage::ManagedApp(id.into())) {
            self.subpage = Some(Subpage::AppManagement);
            self.focus = Target::CatalogRefresh;
            self.scroll = 0.;
        }
    }
}

#[cfg(test)]
mod tests;
