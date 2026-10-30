use super::*;
use crate::state::{State, Subpage, Target};
pub struct Session {
    pub provider: Provider,
    pub preview: bool,
    pub loading: bool,
    pub importing: bool,
    pub confirming: bool,
    pub status: &'static str,
    pub collections: Vec<Collection>,
    pub selected: Vec<String>,
    pub connection: Option<u64>,
    pub page: usize,
    pub return_scroll: f32,
    pub pending: Option<(u64, Provider, Command)>,
    generation: u64,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            provider: Provider::Webdav,
            preview: false,
            loading: false,
            importing: false,
            confirming: false,
            status: "cloudDisconnected",
            collections: vec![],
            selected: vec![],
            connection: None,
            page: 0,
            return_scroll: 0.,
            pending: None,
            generation: 0,
        }
    }
}
impl Session {
    pub fn demo(&mut self, p: Provider) {
        self.preview = true;
        self.provider = p;
        self.status = "cloudPreview";
        self.connection = Some(1);
        self.page = 0;
        self.collections = if p == Provider::GooglePhotos {
            vec![]
        } else {
            (1..=18)
                .map(|i| Collection {
                    id: format!("demo-{i}"),
                    title: format!("Demo folder {i}"),
                })
                .collect()
        };
        self.selected = self
            .collections
            .first()
            .map(|c| vec![c.id.clone()])
            .unwrap_or_default();
    }
}
impl State {
    pub fn cloud_open(&mut self, p: Provider) {
        if self.cloud.loading || self.wallpapers.loading || self.immich.loading {
            self.notify(self.preferences.language.t("cloudLoading"));
            return;
        }
        if self.cloud.provider != p {
            let preview = self.cloud.preview;
            let generation = self.cloud.generation.wrapping_add(1);
            self.cloud = Session {
                provider: p,
                generation,
                ..Default::default()
            };
            if preview {
                self.cloud.demo(p);
            }
        }
        self.cloud.return_scroll = self.scroll;
        self.cloud.confirming = false;
        self.subpage = Some(Subpage::Cloud(p));
        self.focus = Target::Cloud(0);
        self.scroll = 0.;
        self.dirty = true;
    }
    pub fn cloud_rows(&self) -> Vec<Target> {
        let mut rows = vec![Target::Cloud(0), Target::Cloud(1), Target::Cloud(2)];
        if self.cloud.confirming {
            rows.extend([Target::Cloud(3), Target::Cloud(4)]);
        }
        if self.cloud.provider != Provider::GooglePhotos {
            rows.extend([Target::Cloud(5), Target::Cloud(6)]);
            rows.extend(
                (self.cloud.page * PAGE
                    ..self
                        .cloud
                        .collections
                        .len()
                        .min((self.cloud.page + 1) * PAGE))
                    .map(Target::CloudCollection),
            );
        }
        rows
    }
    pub fn cloud_enabled(&self, index: usize) -> bool {
        let s = &self.cloud;
        if self.subpage != Some(Subpage::Cloud(s.provider))
            || s.loading
            || self.wallpapers.loading
            || self.wallpapers.pending
            || self.immich.loading
        {
            return false;
        }
        if s.confirming {
            return matches!(index, 3 | 4);
        }
        match index {
            0 | 2 => !s.preview,
            1 => {
                !s.preview
                    && s.connection.is_some()
                    && (s.provider == Provider::GooglePhotos || !s.selected.is_empty())
            }
            5 => s.page > 0,
            6 => (s.page + 1) * PAGE < s.collections.len(),
            _ => false,
        }
    }
    fn cloud_request(&mut self, command: Command) {
        let s = &mut self.cloud;
        s.generation = s.generation.wrapping_add(1);
        s.loading = true;
        s.importing = matches!(command, Command::Import { .. });
        s.pending = Some((s.generation, s.provider, command));
        s.confirming = false;
        s.status = "cloudLoading";
        if s.importing {
            self.wallpapers.loading = true;
        }
        self.dirty = true;
    }
    pub fn cloud_action(&mut self, index: usize) {
        if !self.cloud_enabled(index) {
            return;
        }
        match index {
            0 => self.cloud_request(Command::Browse),
            1 => {
                if let Some(connection) = self.cloud.connection {
                    self.cloud_request(Command::Import {
                        connection,
                        selected: self.cloud.selected.clone(),
                        preferences: self.preferences.wallpapers.clone(),
                    });
                }
            }
            2 => {
                self.cloud.confirming = true;
                self.focus = Target::Cloud(3);
            }
            3 => {
                self.cloud.confirming = false;
                self.focus = Target::Cloud(2);
            }
            4 => self.cloud_request(Command::Forget {
                connection: self.cloud.connection,
            }),
            5 => {
                self.cloud.page = self.cloud.page.saturating_sub(1);
                self.scroll = 0.;
                self.focus = Target::Cloud(0);
            }
            6 => {
                self.cloud.page += 1;
                self.scroll = 0.;
                self.focus = Target::Cloud(0);
            }
            _ => {}
        }
        self.dirty = true;
    }
    pub fn cloud_toggle(&mut self, index: usize) {
        if !(self.cloud_enabled(0)
            || self.cloud.preview && matches!(self.subpage, Some(Subpage::Cloud(_))))
        {
            return;
        }
        let Some(c) = self.cloud.collections.get(index) else {
            return;
        };
        if self.cloud.selected.contains(&c.id) {
            self.cloud.selected.retain(|id| id != &c.id);
        } else if self.cloud.selected.len() < 3 {
            self.cloud.selected.push(c.id.clone());
        } else {
            self.notify(self.preferences.language.t("cloudSelectionLimit"));
        }
        self.dirty = true;
    }
    pub fn cloud_accept(&mut self, generation: u64, provider: Provider, result: Result<Outcome>) {
        if !self.cloud.loading
            || self.cloud.generation != generation
            || self.cloud.provider != provider
        {
            return;
        }
        let s = &mut self.cloud;
        if s.importing {
            self.wallpapers.loading = false;
        }
        s.loading = false;
        s.importing = false;
        match result {
            Ok(Outcome::Ready(report)) => {
                s.collections = report.collections;
                s.connection = Some(report.connection);
                s.page = 0;
                s.selected
                    .retain(|id| s.collections.iter().any(|c| &c.id == id));
                if s.selected.is_empty()
                    && let Some(first) = s.collections.first()
                {
                    s.selected.push(first.id.clone());
                }
                s.status = "cloudReady";
            }
            Ok(Outcome::Imported(photos)) => {
                self.wallpapers.photos = photos;
                self.wallpapers.decoded.clear();
                s.status = "cloudImported";
            }
            Ok(Outcome::Forgotten) => {
                s.collections.clear();
                s.selected.clear();
                s.connection = None;
                s.page = 0;
                s.status = "cloudForgotten";
            }
            Err(key) => {
                s.status = key;
                if matches!(
                    key,
                    "cloudCredentials" | "cloudChanged" | "cloudDenied" | "cloudExportInvalid"
                ) {
                    s.connection = None;
                }
            }
        }
        if self.subpage == Some(Subpage::Cloud(provider)) {
            self.focus = Target::Cloud(0);
            self.scroll = 0.;
        }
        self.dirty = true;
    }
}
