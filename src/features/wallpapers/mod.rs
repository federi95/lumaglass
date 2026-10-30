//! Explicit image acquisition and bounded offline derivatives, away from SDL.
use image::{DynamicImage, ImageReader, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    hash::{Hash, Hasher},
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

const MAX_SOURCE: u64 = 8 * 1024 * 1024;
const MAX_CACHE: u64 = 64 * 1024 * 1024;
const MAX_IMAGES: usize = 12;
pub(crate) mod immich;
pub(crate) mod rotation;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub rotation: rotation::Preferences,
    pub folder: String,
    pub url: String,
    pub selected: Option<String>,
    pub contain: bool,
}
pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Preferences, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or_default())
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Photo {
    pub id: String,
    pub title: String,
    #[serde(skip)]
    pub preview: Option<std::sync::Arc<crate::app_artwork::Artwork>>,
}
pub fn valid_id(id: &str) -> bool {
    id.strip_prefix("personal-")
        .is_some_and(|s| s.len() == 16 && s.bytes().all(|c| c.is_ascii_hexdigit()))
}
#[derive(Default)]
pub struct Session {
    pub root: PathBuf,
    pub photos: Vec<Photo>,
    pub loading: bool,
    pub pending: bool,
    pub decoded:
        std::collections::HashMap<String, Option<std::sync::Arc<crate::app_artwork::Artwork>>>,
    pub decoding: std::collections::HashSet<String>,
}
impl Session {
    pub fn contains(&self, id: &str) -> bool {
        valid_id(id) && self.photos.iter().any(|p| p.id == id)
    }
}
enum Job {
    Local(Preferences),
    Immich(u64, immich::Command),
}
pub enum Reply {
    Initial(Result<Vec<Photo>, String>),
    Local(Result<Vec<Photo>, String>),
    Immich(u64, Result<immich::Outcome, &'static str>),
}
pub struct Service {
    sender: mpsc::SyncSender<Job>,
    pub receiver: mpsc::Receiver<Reply>,
}
impl Service {
    pub fn new(root: PathBuf, credentials: PathBuf) -> Self {
        let (sender, jobs) = mpsc::sync_channel(1);
        let (results, receiver) = mpsc::channel();
        thread::spawn(move || {
            if results
                .send(Reply::Initial(load(&root).map(|p| previews(&root, p))))
                .is_err()
            {
                return;
            }
            for job in jobs {
                let reply = match job {
                    Job::Local(preferences) => Reply::Local(
                        import(&root, &preferences)
                            .map(|p| previews(&root, p))
                            .map_err(|_| "Image import failed".into()),
                    ),
                    Job::Immich(generation, command) => {
                        Reply::Immich(generation, immich::run(&credentials, &root, command))
                    }
                };
                if results.send(reply).is_err() {
                    break;
                }
            }
        });
        Self { sender, receiver }
    }
    pub fn import(&self, preferences: Preferences) -> bool {
        self.sender.try_send(Job::Local(preferences)).is_ok()
    }
    pub fn immich(&self, generation: u64, command: immich::Command) -> bool {
        self.sender
            .try_send(Job::Immich(generation, command))
            .is_ok()
    }
}

fn load(root: &Path) -> Result<Vec<Photo>, String> {
    if !root.exists() {
        return Ok(vec![]);
    }
    if fs::symlink_metadata(root)
        .map_err(|_| "Cache unavailable")?
        .file_type()
        .is_symlink()
    {
        return Err("Invalid cache directory".into());
    }
    let path = root.join("catalog.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "Catalog unavailable")?
        .take(32769)
        .read_to_end(&mut bytes)
        .map_err(|_| "Invalid catalog")?;
    if bytes.len() > 32768 {
        return Err("Catalog too large".into());
    }
    let mut photos: Vec<Photo> = serde_json::from_slice(&bytes).map_err(|_| "Invalid catalog")?;
    photos.truncate(MAX_IMAGES + 1);
    photos.retain(|p| {
        valid_id(&p.id)
            && p.title.chars().count() <= 80
            && ["", "-blur-soft", "-blur-strong"]
                .iter()
                .all(|s| root.join(format!("{}{s}.jpg", p.id)).is_file())
    });
    Ok(photos)
}
fn bytes(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "Image unavailable")?;
    if !metadata.is_file() || metadata.len() > MAX_SOURCE {
        return Err("Invalid image file".into());
    }
    let mut data = vec![];
    fs::File::open(path)
        .map_err(|_| "Image unavailable")?
        .take(MAX_SOURCE + 1)
        .read_to_end(&mut data)
        .map_err(|_| "Image read failed")?;
    if data.len() as u64 > MAX_SOURCE {
        return Err("Image too large".into());
    }
    Ok(data)
}
fn remote(url: &str) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://")
        || url.len() > 2048
        || url.chars().any(char::is_control)
        || url[8..]
            .split('/')
            .next()
            .is_none_or(|authority| authority.is_empty() || authority.contains('@'))
    {
        return Err("Use an HTTPS image URL".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(8)))
        .max_redirects(0)
        .proxy(None)
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|_| "Image download failed")?;
    let mut data = vec![];
    response
        .body_mut()
        .as_reader()
        .take(MAX_SOURCE + 1)
        .read_to_end(&mut data)
        .map_err(|_| "Image download failed")?;
    if data.len() as u64 > MAX_SOURCE {
        return Err("Image too large".into());
    }
    Ok(data)
}
fn derivative(data: &[u8], contain: bool) -> Result<DynamicImage, String> {
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|_| "Invalid image")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| "Unsupported image")?;
    use image::ImageDecoder;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > 24_000_000 {
        return Err("Too many decoded pixels".into());
    }
    let orientation = decoder.orientation().map_err(|_| "Invalid orientation")?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|_| "Image decoding failed")?;
    image.apply_orientation(orientation);
    if contain {
        let fitted = image.resize(1920, 1080, FilterType::Triangle).to_rgb8();
        let mut canvas = image::RgbImage::new(1920, 1080);
        image::imageops::replace(
            &mut canvas,
            &fitted,
            i64::from((1920 - fitted.width()) / 2),
            i64::from((1080 - fitted.height()) / 2),
        );
        Ok(DynamicImage::ImageRgb8(canvas))
    } else {
        Ok(image
            .resize_to_fill(1920, 1080, FilterType::Triangle)
            .into_rgb8()
            .into())
    }
}
fn cache_size(root: &Path) -> u64 {
    fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
fn store(root: &Path, path: &Path, data: &[u8]) -> Result<(), String> {
    if cache_size(root)
        .saturating_sub(fs::symlink_metadata(path).map_or(0, |m| m.len()))
        .saturating_add(data.len() as u64)
        > MAX_CACHE
    {
        return Err("Image cache is full".into());
    }
    crate::storage::write_atomic(path, data)
}
fn prepare(root: &Path, data: Vec<u8>, title: String, contain: bool) -> Result<Photo, String> {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hash);
    contain.hash(&mut hash);
    let id = format!("personal-{:016x}", hash.finish());
    if [".jpg", "-blur-soft.jpg", "-blur-strong.jpg", ".png"]
        .iter()
        .any(|suffix| !root.join(format!("{id}{suffix}")).is_file())
    {
        let image = derivative(&data, contain)?;
        for (suffix, sigma) in [("", 0.), ("-blur-soft", 8.), ("-blur-strong", 20.)] {
            let mut encoded = vec![];
            let image = if sigma == 0. {
                image.clone()
            } else {
                image.blur(sigma)
            };
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 85)
                .encode_image(&image)
                .map_err(|_| "Image encoding failed")?;
            store(root, &root.join(format!("{id}{suffix}.jpg")), &encoded)?;
        }
        let mut png = Cursor::new(vec![]);
        image
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|_| "Splash encoding failed")?;
        store(root, &root.join(format!("{id}.png")), &png.into_inner())?;
    }
    Ok(Photo {
        id,
        title,
        preview: None,
    })
}

fn decoded(
    root: &Path,
    key: &str,
    small: bool,
) -> Option<std::sync::Arc<crate::app_artwork::Artwork>> {
    let id = key.split("-blur-").next()?;
    if !valid_id(id)
        || ![
            id.to_owned(),
            format!("{id}-blur-soft"),
            format!("{id}-blur-strong"),
        ]
        .iter()
        .any(|allowed| allowed == key)
    {
        return None;
    }
    let data = bytes(&root.join(format!("{key}.jpg"))).ok()?;
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(1920);
    limits.max_image_height = Some(1080);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let pixels = if small {
        image.thumbnail(320, 180).into_rgba8()
    } else {
        image.into_rgba8()
    };
    Some(std::sync::Arc::new(crate::app_artwork::Artwork {
        key: format!(
            "personal:{}:{key}:{}",
            root.display(),
            if small { "thumb" } else { "full" }
        ),
        pixels,
        backdrop: [[0; 3]; 2],
    }))
}
fn previews(root: &Path, mut photos: Vec<Photo>) -> Vec<Photo> {
    for photo in &mut photos {
        photo.preview = decoded(root, &photo.id, true);
    }
    photos
}
type Decoded = Option<std::sync::Arc<crate::app_artwork::Artwork>>;
pub struct Decoder {
    sender: mpsc::SyncSender<String>,
    pub receiver: mpsc::Receiver<(String, Decoded)>,
}
impl Decoder {
    pub fn new(root: PathBuf) -> Self {
        let (sender, jobs) = mpsc::sync_channel::<String>(2);
        let (results, receiver) = mpsc::channel();
        thread::spawn(move || {
            for key in jobs {
                let image = decoded(&root, &key, false);
                if results.send((key, image)).is_err() {
                    break;
                }
            }
        });
        Self { sender, receiver }
    }
    pub fn request(&self, key: String) -> bool {
        self.sender.try_send(key).is_ok()
    }
}
fn ensure_cache(root: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let result = fs::DirBuilder::new().mode(0o700).create(root);
        if result
            .as_ref()
            .is_err_and(|e| e.kind() != std::io::ErrorKind::AlreadyExists)
        {
            return Err("Cannot create image cache".into());
        }
    }
    #[cfg(not(unix))]
    fs::create_dir_all(root).map_err(|_| "Cannot create image cache")?;
    if fs::symlink_metadata(root)
        .map_err(|_| "Invalid cache")?
        .file_type()
        .is_symlink()
    {
        return Err("Invalid cache".into());
    }
    Ok(())
}
fn import(root: &Path, preferences: &Preferences) -> Result<Vec<Photo>, String> {
    ensure_cache(root)?;
    let previous = load(root)?;
    let mut photos = vec![];
    if !preferences.url.trim().is_empty() {
        photos.push(prepare(
            root,
            remote(preferences.url.trim())?,
            "Remote image".into(),
            preferences.contain,
        )?);
    }
    if !preferences.folder.trim().is_empty() {
        let directory = Path::new(preferences.folder.trim());
        if !directory.is_absolute() {
            return Err("Use an absolute image directory".into());
        }
        let mut paths = fs::read_dir(directory)
            .map_err(|_| "Image directory unavailable")?
            .take(512)
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| {
                    matches!(
                        e.to_str().unwrap_or("").to_ascii_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "webp"
                    )
                })
            })
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            if photos.len() >= MAX_IMAGES {
                break;
            }
            let title = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .filter(|c| !c.is_control())
                .take(80)
                .collect();
            if let Ok(photo) =
                bytes(&path).and_then(|data| prepare(root, data, title, preferences.contain))
                && !photos.iter().any(|p| p.id == photo.id)
            {
                photos.push(photo);
            }
        }
    }
    if photos.is_empty() {
        return Err("No valid images; previous cache retained".into());
    }
    finish_import(root, photos, previous, preferences.selected.as_deref())
}
fn finish_import(
    root: &Path,
    mut photos: Vec<Photo>,
    previous: Vec<Photo>,
    selected: Option<&str>,
) -> Result<Vec<Photo>, String> {
    if let Some(old) = previous
        .into_iter()
        .find(|p| Some(p.id.as_str()) == selected)
        && !photos.iter().any(|p| p.id == old.id)
    {
        photos.push(old);
    }
    store(
        root,
        &root.join("catalog.json"),
        &serde_json::to_vec(&photos).map_err(|_| "Invalid catalog")?,
    )?;
    Ok(photos)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_ids_and_insecure_urls_are_rejected_without_network_access() {
        for id in [
            "../../etc/passwd",
            "personal-../bad",
            "personal-123",
            "dunes",
        ] {
            assert!(!valid_id(id));
        }
        assert!(valid_id("personal-0123456789abcdef"));
        for url in [
            "http://localhost/a.jpg",
            "file:///tmp/a.jpg",
            "https://user:pass@example.com/a.jpg",
            "https://",
        ] {
            assert!(remote(url).is_err());
        }
        assert!(derivative(b"not an image", false).is_err());
    }
    #[test]
    fn failed_import_keeps_the_existing_catalog() {
        let root =
            std::env::temp_dir().join(format!("lumaglass-photo-failure-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let before = b"[]";
        crate::storage::write_atomic(&root.join("catalog.json"), before).unwrap();
        assert!(import(&root, &Preferences::default()).is_err());
        assert_eq!(fs::read(root.join("catalog.json")).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }
}
