//! Fixed Google endpoints: Drive GET-only and explicit Photos Picker session lifecycle.
use super::*;
const TOKEN: &str = "https://oauth2.googleapis.com/token";
const DRIVE: &str = "https://www.googleapis.com/drive/v3/files";
pub(super) fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
}
fn access(net: &Net, c: &Connection) -> Result<String> {
    let form = format!(
        "client_id={}&client_secret={}&refresh_token={}&grant_type=refresh_token",
        escaped(&c.client_id),
        escaped(&c.client_secret),
        escaped(&c.refresh_token)
    );
    let data = net.bytes(
        "POST",
        TOKEN,
        "",
        form.into_bytes(),
        "application/x-www-form-urlencoded",
        32768,
    )?;
    let value: Value = serde_json::from_slice(&data).map_err(|_| "cloudDenied")?;
    if value["token_type"]
        .as_str()
        .is_none_or(|s| !s.eq_ignore_ascii_case("bearer"))
        || value["scope"]
            .as_str()
            .is_some_and(|s| !s.split_whitespace().any(|v| v == c.scope))
    {
        return Err("cloudDenied");
    }
    let t = value["access_token"]
        .as_str()
        .filter(|s| token(s, 4096))
        .ok_or("cloudDenied")?;
    Ok(format!("Bearer {t}"))
}
fn folder(net: &Net, auth: &str, folder: &str) -> Result<Collection> {
    if !id(folder) {
        return Err("cloudCredentials");
    }
    let value = net.json(
        "GET",
        &format!("{DRIVE}/{folder}?fields=id,name,mimeType,trashed&supportsAllDrives=true"),
        auth,
        None,
    )?;
    if value["mimeType"] != "application/vnd.google-apps.folder" || value["trashed"] != false {
        return Err("cloudChanged");
    }
    if folder != "root" && value["id"].as_str() != Some(folder) {
        return Err("cloudChanged");
    }
    Ok(Collection {
        id: folder.into(),
        title: title(value["name"].as_str().ok_or("cloudUnavailable")?),
    })
}
pub(super) fn drive(
    net: &Net,
    c: &Connection,
    cache: &Path,
    command: &Command,
    path: &Path,
) -> Result<Outcome> {
    let auth = access(net, c)?;
    let folders = if c.folders.is_empty() {
        vec!["root".into()]
    } else {
        c.folders.clone()
    };
    let collections = folders
        .iter()
        .map(|id| folder(net, &auth, id))
        .collect::<Result<Vec<_>>>()?;
    if let Command::Import {
        selected,
        preferences,
        ..
    } = command
    {
        if selected.is_empty()
            || selected.len() > 3
            || selected.iter().collect::<HashSet<_>>().len() != selected.len()
            || !selected.iter().all(|id| folders.contains(id))
        {
            return Err("cloudChooseFolder");
        }
        let mut import = Importer::new(cache, preferences)?;
        let mut seen = HashSet::new();
        'folders: for folder in selected {
            let mut page = String::new();
            let mut tokens = HashSet::new();
            for _ in 0..5 {
                let q = format!(
                    "'{folder}' in parents and trashed = false and mimeType contains 'image/'"
                );
                let url = format!(
                    "{DRIVE}?q={}&fields={}&pageSize=100&orderBy=name&supportsAllDrives=true&includeItemsFromAllDrives=true&pageToken={}",
                    escaped(&q),
                    escaped(
                        "nextPageToken,incompleteSearch,files(id,name,mimeType,size,trashed,capabilities(canDownload))"
                    ),
                    escaped(&page)
                );
                let response = net.json("GET", &url, &auth, None)?;
                if response["incompleteSearch"] == true {
                    return Err("cloudLimit");
                }
                let files = response["files"]
                    .as_array()
                    .filter(|v| v.len() <= 100)
                    .ok_or("cloudLimit")?;
                for file in files {
                    let Some(file_id) = file["id"].as_str().filter(|s| id(s)) else {
                        return Err("cloudUnavailable");
                    };
                    let size = file["size"].as_str().and_then(|s| s.parse::<u64>().ok());
                    if file["trashed"] != false
                        || file["capabilities"]["canDownload"] != true
                        || !image_type(file["mimeType"].as_str().unwrap_or(""))
                        || size.is_none_or(|s| s > super::super::MAX_SOURCE)
                        || !seen.insert(file_id.to_owned())
                    {
                        continue;
                    }
                    let bytes = net.bytes(
                        "GET",
                        &format!("{DRIVE}/{file_id}?alt=media&supportsAllDrives=true"),
                        &auth,
                        vec![],
                        "",
                        super::super::MAX_SOURCE,
                    )?;
                    import.add(bytes, file["name"].as_str().unwrap_or("Drive photo"))?;
                    if import.full() {
                        break 'folders;
                    }
                }
                match response["nextPageToken"].as_str() {
                    None | Some("") => break,
                    Some(next) if token(next, 2048) && tokens.insert(next.to_owned()) => {
                        page = next.into()
                    }
                    _ => return Err("cloudUnavailable"),
                }
            }
        }
        retain_identity(path, Provider::Drive, c.fingerprint())?;
        return import.finish().map(Outcome::Imported);
    }
    Ok(Outcome::Ready(Report {
        collections,
        connection: c.fingerprint(),
    }))
}
