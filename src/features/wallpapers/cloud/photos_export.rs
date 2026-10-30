//! Deliberate user exports are local files, rather than a long-lived Google API cache.
use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u8,
    source: String,
    files: Vec<String>,
}
fn manifest(c: &Connection) -> Result<(PathBuf, Manifest)> {
    let folder = PathBuf::from(&c.export_folder);
    let meta = fs::symlink_metadata(&folder).map_err(|_| "cloudExportInvalid")?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("cloudExportInvalid");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err("cloudExportInvalid");
        }
    }
    let path = folder.join("google-photos-export.json");
    let mut file = super::super::immich::private_file(&path).map_err(|_| "cloudExportInvalid")?;
    let mut data = vec![];
    file.by_ref()
        .take(4097)
        .read_to_end(&mut data)
        .map_err(|_| "cloudExportInvalid")?;
    if data.len() > 4096 {
        return Err("cloudExportInvalid");
    }
    let manifest: Manifest = serde_json::from_slice(&data).map_err(|_| "cloudExportInvalid")?;
    if manifest.schema != 1
        || manifest.source != "google-photos-picker"
        || manifest.files.is_empty()
        || manifest.files.len() > 12
        || manifest.files.iter().collect::<HashSet<_>>().len() != manifest.files.len()
        || !manifest
            .files
            .iter()
            .all(|s| (1..=12).any(|i| s == &format!("photo-{i:02}.jpg")))
    {
        return Err("cloudExportInvalid");
    }
    Ok((folder, manifest))
}
pub(super) fn run(c: &Connection, cache: &Path, command: &Command, path: &Path) -> Result<Outcome> {
    let (folder, manifest) = manifest(c)?;
    if let Command::Import { preferences, .. } = command {
        let mut import = Importer::new(cache, preferences)?;
        for (index, name) in manifest.files.iter().enumerate() {
            let source = folder.join(name);
            let mut file = private_image(&source)?;
            let mut data = vec![];
            file.by_ref()
                .take(super::super::MAX_SOURCE + 1)
                .read_to_end(&mut data)
                .map_err(|_| "cloudExportInvalid")?;
            if data.len() as u64 > super::super::MAX_SOURCE {
                return Err("cloudLimit");
            }
            import.add(data, &format!("Google Photos {}", index + 1))?;
        }
        retain_identity(path, Provider::GooglePhotos, c.fingerprint())?;
        return import.finish().map(Outcome::Imported);
    }
    Ok(Outcome::Ready(Report {
        collections: vec![],
        connection: c.fingerprint(),
    }))
}
fn private_image(path: &Path) -> Result<fs::File> {
    let meta = fs::symlink_metadata(path).map_err(|_| "cloudExportInvalid")?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > super::super::MAX_SOURCE {
        return Err("cloudExportInvalid");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err("cloudExportInvalid");
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| "cloudExportInvalid")?;
        let opened = file.metadata().map_err(|_| "cloudExportInvalid")?;
        if opened.ino() != meta.ino() || opened.dev() != meta.dev() {
            return Err("cloudExportInvalid");
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        fs::File::open(path).map_err(|_| "cloudExportInvalid")
    }
}
