//! Original read-only Immich client. Credentials never enter preferences or render state.
use super::{Photo, Preferences};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
mod state;
pub use state::Session;
#[cfg(test)]
mod tests;
const JSON_LIMIT: u64 = 2 * 1024 * 1024;
pub const PAGE: usize = 8;
const ALBUM_LIMIT: usize = 512;
const SELECT_LIMIT: usize = 3;
type Result<T> = std::result::Result<T, &'static str>;
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Connection {
    schema: u8,
    server: String,
    api_key: String,
    #[serde(default)]
    albums: Vec<String>,
}
impl Connection {
    fn valid(&self) -> bool {
        self.schema == 1
            && (8..=512).contains(&self.api_key.len())
            && self.api_key.bytes().all(|c| c.is_ascii_graphic())
            && selection_valid(&self.albums)
    }
    fn fingerprint(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&self.server, &self.api_key).hash(&mut h);
        h.finish()
    }
    fn read(path: &Path) -> Result<Self> {
        let mut file = private_file(path)?;
        let mut bytes = vec![];
        file.by_ref()
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| "immichCredentials")?;
        if bytes.len() > 4096 {
            return Err("immichCredentials");
        }
        let connection: Self = serde_json::from_slice(&bytes).map_err(|_| "immichCredentials")?;
        if !connection.valid() {
            return Err("immichCredentials");
        }
        Ok(connection)
    }
    fn save(&self, path: &Path) -> Result<()> {
        let _ = private_file(path)?;
        crate::storage::write_atomic(
            path,
            &serde_json::to_vec(self).map_err(|_| "immichCredentials")?,
        )
        .map_err(|_| "immichCredentials")
    }
}
pub(super) fn private_file(path: &Path) -> Result<fs::File> {
    let meta = fs::symlink_metadata(path).map_err(|_| "immichCredentials")?;
    let parent = fs::symlink_metadata(path.parent().ok_or("immichCredentials")?)
        .map_err(|_| "immichCredentials")?;
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.len() > 4096
        || !parent.is_dir()
        || parent.file_type().is_symlink()
    {
        return Err("immichCredentials");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let uid = unsafe { libc::geteuid() };
        if meta.uid() != uid
            || meta.mode() & 0o077 != 0
            || parent.uid() != uid
            || parent.mode() & 0o022 != 0
        {
            return Err("immichCredentials");
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| "immichCredentials")?;
        let opened = file.metadata().map_err(|_| "immichCredentials")?;
        if opened.ino() != meta.ino() || opened.dev() != meta.dev() {
            return Err("immichCredentials");
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        Err("immichCredentials")
    }
}
fn api_base(server: &str) -> Result<String> {
    if server.len() > 2048
        || server.contains(['@', '#', '\\', '%'])
        || server
            .bytes()
            .any(|c| c.is_ascii_whitespace() || c.is_ascii_control())
    {
        return Err("immichInvalidServer");
    }
    let uri: ureq::http::Uri = server.parse().map_err(|_| "immichInvalidServer")?;
    if uri.scheme_str() != Some("https")
        || uri.authority().is_none_or(|a| a.host().is_empty())
        || uri.query().is_some()
        || !["/", "/api", "/api/"].contains(&uri.path())
    {
        return Err("immichInvalidServer");
    }
    Ok(format!(
        "https://{}/api",
        uri.authority().ok_or("immichInvalidServer")?
    ))
}
pub fn uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit() && !c.is_ascii_uppercase()
            }
        })
        && id.as_bytes()[14] == b'4'
        && b"89ab".contains(&id.as_bytes()[19])
}
fn selection_valid(ids: &[String]) -> bool {
    ids.len() <= SELECT_LIMIT
        && ids.iter().all(|s| uuid(s))
        && ids.iter().collect::<HashSet<_>>().len() == ids.len()
}
#[derive(Clone)]
pub struct Album {
    pub id: String,
    pub title: String,
}
pub struct Report {
    pub albums: Vec<Album>,
    pub selected: Vec<String>,
    pub connection: u64,
    pub version: String,
}
pub enum Outcome {
    Albums(Report),
    Imported(Vec<Photo>),
    Forgotten,
}
#[derive(Clone)]
pub enum Command {
    Browse,
    Import {
        selected: Vec<String>,
        connection: u64,
        preferences: Preferences,
    },
    Forget {
        connection: Option<u64>,
    },
}
struct Client {
    agent: ureq::Agent,
    base: String,
    key: String,
    modern: bool,
    started: Instant,
}
impl Client {
    fn new(c: &Connection) -> Result<Self> {
        Self::at(api_base(&c.server)?, c.api_key.clone())
    }
    fn at(base: String, key: String) -> Result<Self> {
        Ok(Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(8)))
                .max_redirects(0)
                .proxy(None)
                .build()
                .new_agent(),
            base,
            key,
            modern: false,
            started: Instant::now(),
        })
    }
    fn budget(&self) -> Result<()> {
        if self.started.elapsed() >= Duration::from_secs(60) {
            Err("immichLimit")
        } else {
            Ok(())
        }
    }
    fn response(
        &self,
        path: &str,
        body: Option<Value>,
        authenticated: bool,
        limit: u64,
    ) -> Result<Vec<u8>> {
        self.budget()?;
        let url = format!("{}{path}", self.base);
        let response = if let Some(body) = body {
            self.agent
                .post(&url)
                .header("x-api-key", &self.key)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("User-Agent", "LumaglassNative")
                .send(serde_json::to_vec(&body).map_err(|_| "immichUnavailable")?)
        } else {
            let request = self.agent.get(&url).header("User-Agent", "LumaglassNative");
            let request = if authenticated {
                request.header("x-api-key", &self.key)
            } else {
                request
            };
            request.call()
        };
        let mut response = response.map_err(|e| match e {
            ureq::Error::StatusCode(401 | 403) => "immichDenied",
            _ => "immichUnavailable",
        })?;
        if !response.status().is_success() {
            return Err("immichUnavailable");
        }
        let mut data = vec![];
        response
            .body_mut()
            .as_reader()
            .take(limit + 1)
            .read_to_end(&mut data)
            .map_err(|_| "immichUnavailable")?;
        if data.len() as u64 > limit {
            return Err("immichLimit");
        }
        self.budget()?;
        Ok(data)
    }
    fn json(&self, path: &str, body: Option<Value>, authenticated: bool) -> Result<Value> {
        serde_json::from_slice(&self.response(path, body, authenticated, JSON_LIMIT)?)
            .map_err(|_| "immichUnavailable")
    }
    fn version(&mut self) -> Result<String> {
        let v = self.json("/server/version", None, false)?;
        let major = v["major"].as_u64().ok_or("immichUnsupported")?;
        let minor = v["minor"].as_u64().ok_or("immichUnsupported")?;
        let patch = v["patch"].as_u64().ok_or("immichUnsupported")?;
        if !((major == 2 && minor >= 5) || major == 3) {
            return Err("immichUnsupported");
        }
        self.modern = major == 3 && minor >= 2;
        Ok(format!("{major}.{minor}.{patch}"))
    }
    fn albums(&self) -> Result<Vec<Album>> {
        let value = self.json("/albums", None, true)?;
        let items = value.as_array().ok_or("immichUnavailable")?;
        if items.len() > ALBUM_LIMIT {
            return Err("immichLimit");
        }
        let mut ids = HashSet::new();
        let mut albums = vec![];
        for item in items {
            let id = item["id"]
                .as_str()
                .filter(|s| uuid(s))
                .ok_or("immichUnavailable")?;
            if !ids.insert(id) {
                return Err("immichUnavailable");
            }
            let name = item["albumName"].as_str().ok_or("immichUnavailable")?;
            let title = name
                .chars()
                .filter(|c| !c.is_control())
                .take(80)
                .collect::<String>();
            albums.push(Album {
                id: id.into(),
                title: if title.trim().is_empty() {
                    "Immich".into()
                } else {
                    title
                },
            });
        }
        albums.sort_by_cached_key(|a| a.title.to_lowercase());
        Ok(albums)
    }
    fn assets(&self, album: &str) -> Result<Vec<String>> {
        if !uuid(album) {
            return Err("immichUnavailable");
        }
        let request = if self.modern {
            json!({"size":12,"filter":{"albumIds":{"any":[album]},"type":{"eq":"IMAGE"},"visibility":{"eq":"timeline"},"isOffline":{"eq":false},"trashedAt":{"eq":null}}})
        } else {
            json!({"size":12,"page":1,"albumIds":[album],"type":"IMAGE","visibility":"timeline","isOffline":false})
        };
        let value = self.json("/search/metadata", Some(request), true)?;
        let items = value["assets"]["items"]
            .as_array()
            .ok_or("immichUnavailable")?;
        if items.len() > 12 {
            return Err("immichLimit");
        }
        let mut ids = HashSet::new();
        let mut assets = vec![];
        for item in items {
            if item["type"].as_str() != Some("IMAGE")
                || item["isTrashed"].as_bool() != Some(false)
                || item["isOffline"].as_bool() != Some(false)
                || item["isArchived"].as_bool() != Some(false)
                || item["visibility"].as_str() != Some("timeline")
            {
                continue;
            }
            let id = item["id"]
                .as_str()
                .filter(|s| uuid(s))
                .ok_or("immichUnavailable")?;
            if ids.insert(id) {
                assets.push(id.into());
            }
        }
        Ok(assets)
    }
    fn import(
        &self,
        root: &Path,
        selected: &[String],
        preferences: &Preferences,
    ) -> Result<Vec<Photo>> {
        if selected.is_empty() || !selection_valid(selected) {
            return Err("immichSelectAlbum");
        }
        let listed = self.albums()?;
        if !selected.iter().all(|id| listed.iter().any(|a| &a.id == id)) {
            return Err("immichChanged");
        }
        super::ensure_cache(root).map_err(|_| "photoImportFailed")?;
        let previous = super::load(root).map_err(|_| "photoImportFailed")?;
        let mut photos = vec![];
        let mut seen = HashSet::new();
        let mut downloaded = 0u64;
        'albums: for album in selected {
            for id in self.assets(album)? {
                if !seen.insert(id.clone()) {
                    continue;
                }
                let data = self.response(
                    &format!("/assets/{id}/thumbnail?size=preview"),
                    None,
                    true,
                    super::MAX_SOURCE,
                )?;
                downloaded += data.len() as u64;
                if downloaded > 32 * 1024 * 1024 {
                    return Err("immichLimit");
                }
                if let Ok(photo) = super::prepare(
                    root,
                    data,
                    format!("Immich {}", photos.len() + 1),
                    preferences.contain,
                ) && !photos.iter().any(|p: &Photo| p.id == photo.id)
                {
                    photos.push(photo);
                }
                self.budget()?;
                if photos.len() == super::MAX_IMAGES {
                    break 'albums;
                }
            }
        }
        if photos.is_empty() {
            return Err("immichNoPhotos");
        }
        super::finish_import(root, photos, previous, preferences.selected.as_deref())
            .map_err(|_| "photoImportFailed")
    }
}
pub(super) fn run(path: &Path, root: &Path, command: Command) -> Result<Outcome> {
    if let Command::Forget { connection } = command {
        let _ = private_file(path)?;
        if let Some(expected) = connection
            && Connection::read(path)?.fingerprint() != expected
        {
            return Err("immichChanged");
        }
        fs::remove_file(path).map_err(|_| "immichCredentials")?;
        fs::File::open(path.parent().ok_or("immichCredentials")?)
            .and_then(|f| f.sync_all())
            .map_err(|_| "immichCredentials")?;
        return Ok(Outcome::Forgotten);
    }
    let mut connection = Connection::read(path)?;
    let mut client = Client::new(&connection)?;
    if let Command::Import {
        selected,
        connection: expected,
        preferences,
    } = command
    {
        if connection.fingerprint() != expected {
            return Err("immichChanged");
        }
        if selected.is_empty() || !selection_valid(&selected) {
            return Err("immichSelectAlbum");
        }
        client.version()?;
        connection.albums = selected.clone();
        connection.save(path)?;
        return client
            .import(root, &selected, &preferences)
            .map(|photos| Outcome::Imported(super::previews(root, photos)));
    }
    let version = client.version()?;
    let albums = client.albums()?;
    connection
        .albums
        .retain(|id| albums.iter().any(|a| &a.id == id));
    Ok(Outcome::Albums(Report {
        albums,
        selected: connection.albums.clone(),
        connection: connection.fingerprint(),
        version,
    }))
}
pub fn credentials_path(preferences: &Path) -> PathBuf {
    preferences.with_extension("immich.json")
}
