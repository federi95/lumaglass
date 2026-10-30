//! Explicit read-only cloud acquisition. Tokens and server addresses stay on the worker.
use super::{Photo, Preferences};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
mod google;
mod photos_export;
mod state;
mod webdav;
pub use state::Session;
#[cfg(test)]
mod tests;
pub const PAGE: usize = 8;
const JSON_LIMIT: u64 = 2 * 1024 * 1024;
type Result<T> = std::result::Result<T, &'static str>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Webdav,
    Drive,
    GooglePhotos,
}
impl Provider {
    pub const ALL: [Self; 3] = [Self::Webdav, Self::Drive, Self::GooglePhotos];
    pub fn key(self) -> &'static str {
        match self {
            Self::Webdav => "cloudWebdav",
            Self::Drive => "cloudDrive",
            Self::GooglePhotos => "cloudGooglePhotos",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Webdav => "webdav.json",
            Self::Drive => "google-drive.json",
            Self::GooglePhotos => "google-photos.json",
        }
    }
    pub fn scope(self) -> &'static str {
        match self {
            Self::Drive => "https://www.googleapis.com/auth/drive.readonly",
            Self::GooglePhotos => {
                "https://www.googleapis.com/auth/photospicker.mediaitems.readonly"
            }
            Self::Webdav => "",
        }
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Connection {
    schema: u8,
    #[serde(default)]
    export_folder: String,
    #[serde(default)]
    server: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    app_password: String,
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    folders: Vec<String>,
}
impl Connection {
    fn read(path: &Path, provider: Provider) -> Result<Self> {
        let mut f = super::immich::private_file(path).map_err(|_| "cloudCredentials")?;
        let mut bytes = vec![];
        f.by_ref()
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| "cloudCredentials")?;
        if bytes.len() > 4096 {
            return Err("cloudCredentials");
        }
        let c: Self = serde_json::from_slice(&bytes).map_err(|_| "cloudCredentials")?;
        if c.schema != 1
            || c.folders.len() > 3
            || c.folders.iter().collect::<HashSet<_>>().len() != c.folders.len()
        {
            return Err("cloudCredentials");
        }
        let valid = if provider == Provider::GooglePhotos {
            Path::new(&c.export_folder).is_absolute()
                && c.export_folder.len() <= 1024
                && !c.export_folder.chars().any(char::is_control)
                && c.server.is_empty()
                && c.username.is_empty()
                && c.app_password.is_empty()
                && c.client_id.is_empty()
                && c.client_secret.is_empty()
                && c.refresh_token.is_empty()
                && c.scope.is_empty()
                && c.folders.is_empty()
        } else if provider == Provider::Webdav {
            c.export_folder.is_empty()
                && c.client_id.is_empty()
                && c.refresh_token.is_empty()
                && c.client_secret.is_empty()
                && c.scope.is_empty()
                && c.folders.is_empty()
                && !c.username.is_empty()
                && c.username.len() <= 256
                && !c.username.contains(':')
                && !c.username.chars().any(char::is_control)
                && (1..=512).contains(&c.app_password.len())
                && !c.app_password.chars().any(char::is_control)
                && webdav::root(&c.server).is_ok()
        } else {
            c.export_folder.is_empty()
                && c.server.is_empty()
                && c.username.is_empty()
                && c.app_password.is_empty()
                && c.client_id.ends_with(".apps.googleusercontent.com")
                && token(&c.client_id, 256)
                && token(&c.client_secret, 512)
                && token(&c.refresh_token, 2048)
                && c.scope == provider.scope()
                && c.folders.iter().all(|id| google::id(id))
                && (provider != Provider::GooglePhotos || c.folders.is_empty())
        };
        if !valid {
            return Err("cloudCredentials");
        }
        Ok(c)
    }
    fn fingerprint(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (
            &self.export_folder,
            &self.server,
            &self.username,
            &self.app_password,
            &self.client_id,
            &self.client_secret,
            &self.refresh_token,
            &self.scope,
            &self.folders,
        )
            .hash(&mut h);
        h.finish()
    }
}
fn token(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_graphic())
}
pub fn credentials_path(preferences: &Path, p: Provider) -> PathBuf {
    preferences.with_extension(p.extension())
}
#[derive(Clone)]
pub struct Collection {
    pub id: String,
    pub title: String,
}
pub struct Report {
    pub collections: Vec<Collection>,
    pub connection: u64,
}
pub enum Outcome {
    Ready(Report),
    Imported(Vec<Photo>),
    Forgotten,
}
#[derive(Clone)]
pub enum Command {
    Browse,
    Import {
        connection: u64,
        selected: Vec<String>,
        preferences: Preferences,
    },
    Forget {
        connection: Option<u64>,
    },
}
struct Net {
    #[cfg(test)]
    mock: Option<String>,
    agent: ureq::Agent,
    started: Instant,
}
impl Net {
    fn new() -> Self {
        Self {
            #[cfg(test)]
            mock: None,
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(8)))
                .max_redirects(0)
                .proxy(None)
                .build()
                .new_agent(),
            started: Instant::now(),
        }
    }
    fn budget(&self) -> Result<()> {
        if self.started.elapsed() > Duration::from_secs(60) {
            Err("cloudLimit")
        } else {
            Ok(())
        }
    }
    fn bytes(
        &self,
        method: &str,
        url: &str,
        authorization: &str,
        body: Vec<u8>,
        content_type: &str,
        limit: u64,
    ) -> Result<Vec<u8>> {
        self.budget()?;
        #[cfg(test)]
        if self.mock.is_none() {
            return Err("cloudUnavailable");
        }
        #[cfg(test)]
        let rewritten = self.mock.as_ref().map(|endpoint| {
            let uri: ureq::http::Uri = url.parse().expect("mock URL");
            format!(
                "{endpoint}/{}{}",
                uri.authority().expect("mock authority"),
                uri.path_and_query().map(|p| p.as_str()).unwrap_or("/")
            )
        });
        #[cfg(test)]
        let url = rewritten.as_deref().unwrap_or(url);
        let response = if method == "GET" {
            self.agent
                .get(url)
                .header("Authorization", authorization)
                .header("User-Agent", "LumaglassNative")
                .call()
        } else {
            let mut req = ureq::http::Request::builder()
                .method(method)
                .uri(url)
                .header("Authorization", authorization)
                .header("Content-Type", content_type)
                .header("User-Agent", "LumaglassNative");
            if method == "PROPFIND" {
                req = req
                    .header("Depth", "1")
                    .header("X-Requested-With", "XMLHttpRequest");
            }
            let req = req.body(body).map_err(|_| "cloudUnavailable")?;
            let req = self
                .agent
                .configure_request(req)
                .allow_non_standard_methods(method == "PROPFIND")
                .build();
            self.agent.run(req)
        };
        let mut response = response.map_err(|e| match e {
            ureq::Error::StatusCode(401 | 403) => "cloudDenied",
            _ => "cloudUnavailable",
        })?;
        if !response.status().is_success() {
            return Err("cloudUnavailable");
        }
        let mut bytes = vec![];
        response
            .body_mut()
            .as_reader()
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "cloudUnavailable")?;
        if bytes.len() as u64 > limit {
            return Err("cloudLimit");
        }
        self.budget()?;
        Ok(bytes)
    }
    fn json(&self, method: &str, url: &str, auth: &str, body: Option<Value>) -> Result<Value> {
        let data = self.bytes(
            method,
            url,
            auth,
            body.map(|v| v.to_string().into_bytes()).unwrap_or_default(),
            "application/json",
            JSON_LIMIT,
        )?;
        serde_json::from_slice(&data).map_err(|_| "cloudUnavailable")
    }
}
fn title(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).take(80).collect()
}
fn image_type(value: &str) -> bool {
    matches!(value, "image/jpeg" | "image/png" | "image/webp")
}
fn escaped(s: &str) -> String {
    let mut v = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            v.push(b as char)
        } else {
            v.push_str(&format!("%{b:02X}"))
        }
    }
    v
}
fn https(url: &str) -> Result<ureq::http::Uri> {
    if url.len() > 4096 || url.contains(['#', '\\']) || url.chars().any(char::is_control) {
        return Err("cloudInvalidServer");
    }
    let u: ureq::http::Uri = url.parse().map_err(|_| "cloudInvalidServer")?;
    if u.scheme_str() != Some("https")
        || u.host().is_none_or(str::is_empty)
        || u.authority().is_none_or(|a| a.as_str().contains('@'))
    {
        return Err("cloudInvalidServer");
    }
    Ok(u)
}
fn retain_identity(path: &Path, p: Provider, expected: u64) -> Result<()> {
    if Connection::read(path, p)?.fingerprint() != expected {
        Err("cloudChanged")
    } else {
        Ok(())
    }
}
struct Importer<'a> {
    root: &'a Path,
    preferences: &'a Preferences,
    previous: Vec<Photo>,
    photos: Vec<Photo>,
    downloaded: u64,
}
impl<'a> Importer<'a> {
    fn new(root: &'a Path, preferences: &'a Preferences) -> Result<Self> {
        super::ensure_cache(root).map_err(|_| "photoImportFailed")?;
        Ok(Self {
            root,
            preferences,
            previous: super::load(root).map_err(|_| "photoImportFailed")?,
            photos: vec![],
            downloaded: 0,
        })
    }
    fn add(&mut self, bytes: Vec<u8>, label: &str) -> Result<()> {
        self.downloaded += bytes.len() as u64;
        if self.downloaded > 32 * 1024 * 1024 {
            return Err("cloudLimit");
        }
        if let Ok(photo) = super::prepare(self.root, bytes, title(label), self.preferences.contain)
            && !self.photos.iter().any(|p| p.id == photo.id)
        {
            self.photos.push(photo);
        }
        Ok(())
    }
    fn full(&self) -> bool {
        self.photos.len() >= super::MAX_IMAGES
    }
    fn finish(self) -> Result<Vec<Photo>> {
        if self.photos.is_empty() {
            return Err("cloudNoPhotos");
        }
        super::finish_import(
            self.root,
            self.photos,
            self.previous,
            self.preferences.selected.as_deref(),
        )
        .map(|p| super::previews(self.root, p))
        .map_err(|_| "photoImportFailed")
    }
}
pub(super) fn run(
    preferences_path: &Path,
    root: &Path,
    provider: Provider,
    command: Command,
) -> Result<Outcome> {
    let path = credentials_path(preferences_path, provider);
    if let Command::Forget { connection } = command {
        let _ = super::immich::private_file(&path).map_err(|_| "cloudCredentials")?;
        if let Some(expected) = connection {
            retain_identity(&path, provider, expected)?;
        }
        // Forget is local: no token is used and no remote photo/session is deleted.
        fs::remove_file(&path).map_err(|_| "cloudCredentials")?;
        fs::File::open(path.parent().ok_or("cloudCredentials")?)
            .and_then(|f| f.sync_all())
            .map_err(|_| "cloudCredentials")?;
        return Ok(Outcome::Forgotten);
    }
    let c = Connection::read(&path, provider)?;
    let fingerprint = c.fingerprint();
    let net = Net::new();
    if let Command::Import { connection, .. } = &command
        && *connection != fingerprint
    {
        return Err("cloudChanged");
    }
    let out = match provider {
        Provider::Webdav => webdav::run(&net, &c, root, &command, &path)?,
        Provider::Drive => google::drive(&net, &c, root, &command, &path)?,
        Provider::GooglePhotos => photos_export::run(&c, root, &command, &path)?,
    };
    Ok(out)
}
