use super::*;
use crate::state::{State, Subpage, Target};
#[derive(Default)]
pub struct Session {
    pub loading: bool,
    pub preview: bool,
    pub importing: bool,
    browsing: bool,
    pub confirming: bool,
    pub status: &'static str,
    pub albums: Vec<Album>,
    pub selected: Vec<String>,
    pub page: usize,
    pub version: String,
    pub connection: Option<u64>,
    pub return_scroll: f32,
    pub pending: Option<(u64, Command)>,
    generation: u64,
}
impl Session {
    pub fn demo(&mut self) {
        self.preview = true;
        self.status = "immichPreview";
        self.albums = (1..=18)
            .map(|i| Album {
                id: format!("{i:08x}-1111-4111-8111-111111111111"),
                title: format!("Demo album {i}"),
            })
            .collect();
    }
    pub fn connected(&self) -> bool {
        self.connection.is_some()
    }
}
impl State {
    pub fn immich_rows(&self) -> Vec<Target> {
        let mut rows = vec![Target::Immich(0), Target::Immich(1), Target::Immich(2)];
        if self.immich.confirming {
            rows.extend([Target::Immich(3), Target::Immich(4)]);
        }
        rows.extend([Target::Immich(5), Target::Immich(6)]);
        rows.extend(
            self.immich
                .albums
                .iter()
                .skip(self.immich.page * PAGE)
                .take(PAGE)
                .map(|a| Target::ImmichAlbum(a.id.clone())),
        );
        rows
    }
    pub fn immich_enabled(&self, index: usize) -> bool {
        if self.subpage != Some(Subpage::Immich)
            || self.immich.loading
            || self.wallpapers.loading
            || self.wallpapers.pending
        {
            return false;
        }
        if self.immich.confirming {
            return matches!(index, 3 | 4);
        }
        match index {
            0 | 2 => !self.immich.preview,
            1 => self.immich.connected() && !self.immich.selected.is_empty(),
            5 => self.immich.page > 0,
            6 => (self.immich.page + 1) * PAGE < self.immich.albums.len(),
            _ => false,
        }
    }
    pub fn immich_request(&mut self, command: Command) {
        if self.immich.preview {
            return;
        }
        let importing = matches!(command, Command::Import { .. });
        let browsing = matches!(command, Command::Browse);
        let session = &mut self.immich;
        session.generation = session.generation.wrapping_add(1);
        session.pending = Some((session.generation, command));
        session.loading = true;
        session.importing = importing;
        session.browsing = browsing;
        session.confirming = false;
        session.status = "immichLoading";
        if importing {
            self.wallpapers.loading = true;
        }
        self.dirty = true;
    }
    pub fn immich_action(&mut self, index: usize) {
        if !self.immich_enabled(index) {
            return;
        }
        match index {
            0 => self.immich_request(Command::Browse),
            1 => {
                if let Some(connection) = self.immich.connection {
                    self.immich_request(Command::Import {
                        selected: self.immich.selected.clone(),
                        connection,
                        preferences: self.preferences.wallpapers.clone(),
                    });
                }
            }
            2 => {
                self.immich.confirming = true;
                self.focus = Target::Immich(3);
            }
            3 => {
                self.immich.confirming = false;
                self.focus = Target::Immich(2);
            }
            4 => self.immich_request(Command::Forget {
                connection: self.immich.connection,
            }),
            5 => {
                self.immich.page = self.immich.page.saturating_sub(1);
                self.scroll = 0.;
                self.focus = Target::Immich(0);
            }
            6 => {
                self.immich.page += 1;
                self.scroll = 0.;
                self.focus = Target::Immich(0);
            }
            _ => {}
        }
        self.dirty = true;
    }
    pub fn immich_toggle(&mut self, id: &str) {
        if self.subpage != Some(Subpage::Immich)
            || self.immich.loading
            || self.immich.confirming
            || self.wallpapers.loading
            || !self.immich.albums.iter().any(|a| a.id == id)
        {
            return;
        }
        if self.immich.selected.iter().any(|v| v == id) {
            self.immich.selected.retain(|v| v != id);
        } else if self.immich.selected.len() < SELECT_LIMIT {
            self.immich.selected.push(id.into());
        } else {
            self.notify(self.preferences.language.t("immichSelectionLimit"));
        }
        self.dirty = true;
    }
    pub fn immich_accept(&mut self, generation: u64, result: Result<Outcome>) {
        if !self.immich.loading || self.immich.generation != generation {
            return;
        }
        if self.immich.importing {
            self.wallpapers.loading = false;
        }
        self.immich.loading = false;
        self.immich.importing = false;
        match result {
            Ok(Outcome::Albums(report)) => {
                self.immich.albums = report.albums;
                self.immich.selected = report.selected;
                self.immich.connection = Some(report.connection);
                self.immich.version = report.version;
                self.immich.page = 0;
                self.immich.status = "immichReady";
            }
            Ok(Outcome::Imported(photos)) => {
                self.wallpapers.photos = photos;
                self.wallpapers.decoded.clear();
                self.immich.status = "immichImported";
            }
            Ok(Outcome::Forgotten) => {
                self.immich.albums.clear();
                self.immich.selected.clear();
                self.immich.connection = None;
                self.immich.version.clear();
                self.immich.page = 0;
                self.immich.status = "immichForgotten";
            }
            Err(key) => {
                self.immich.status = key;
                if self.immich.browsing {
                    self.immich.connection = None;
                    self.immich.version.clear();
                    self.immich.page = 0;
                    self.immich.albums.clear();
                    self.immich.selected.clear();
                }
            }
        }
        if self.subpage == Some(Subpage::Immich) {
            self.focus = Target::Immich(0);
            self.scroll = 0.;
        }
        self.dirty = true;
    }
}
