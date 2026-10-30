//! Depth-one DAV discovery, with authenticated downloads confined to the configured folder.
use super::*;
const PROPS: &str = r#"<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:displayname/><d:resourcetype/><d:getcontenttype/><d:getcontentlength/></d:prop></d:propfind>"#;
fn clean_path(path: &str) -> bool {
    if !path.starts_with('/') {
        return false;
    }
    path.split('/').all(|part| {
        let mut bytes = vec![];
        let raw = part.as_bytes();
        let mut i = 0;
        while i < raw.len() {
            if raw[i] == b'%' {
                if i + 2 >= raw.len() {
                    return false;
                }
                let Some(a) = (raw[i + 1] as char).to_digit(16) else {
                    return false;
                };
                let Some(b) = (raw[i + 2] as char).to_digit(16) else {
                    return false;
                };
                bytes.push((a * 16 + b) as u8);
                i += 3;
            } else {
                bytes.push(raw[i]);
                i += 1;
            }
        }
        let Ok(value) = std::str::from_utf8(&bytes) else {
            return false;
        };
        !matches!(value, "." | "..")
            && !value.contains(['/', '\\'])
            && !value.chars().any(char::is_control)
    })
}
pub(super) fn root(url: &str) -> Result<String> {
    let uri = https(url)?;
    if uri.query().is_some() || !clean_path(uri.path()) {
        return Err("cloudInvalidServer");
    }
    Ok(format!(
        "https://{}{}{}",
        uri.authority().ok_or("cloudInvalidServer")?,
        uri.path(),
        if uri.path().ends_with('/') { "" } else { "/" }
    ))
}
fn href(base: &str, value: &str) -> Result<String> {
    if value.starts_with("//") {
        return Err("cloudInvalidServer");
    }
    let authority = https(base)?
        .authority()
        .ok_or("cloudInvalidServer")?
        .as_str()
        .to_owned();
    let url = if value.starts_with('/') && !value.starts_with("//") {
        format!("https://{authority}{value}")
    } else if value.starts_with("https://") {
        value.to_owned()
    } else if !value.contains(':') {
        format!("{base}{value}")
    } else {
        return Err("cloudInvalidServer");
    };
    let uri = https(&url)?;
    if uri.query().is_some()
        || uri.authority().is_none_or(|a| a.as_str() != authority)
        || !clean_path(uri.path())
    {
        return Err("cloudInvalidServer");
    }
    let prefix = https(base)?.path().to_owned();
    if uri.path() == prefix.trim_end_matches('/') {
        return Ok(format!("{url}/"));
    }
    if !uri.path().starts_with(&prefix) {
        return Err("cloudInvalidServer");
    }
    Ok(url)
}
pub(super) struct Entry {
    url: String,
    name: String,
    directory: bool,
    mime: String,
    size: Option<u64>,
}
pub(super) fn entries(base: &str, xml: &[u8]) -> Result<Vec<Entry>> {
    let text = std::str::from_utf8(xml).map_err(|_| "cloudUnavailable")?;
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 12000,
            ..Default::default()
        },
    )
    .map_err(|_| "cloudUnavailable")?;
    if !doc.root_element().has_tag_name(("DAV:", "multistatus")) {
        return Err("cloudUnavailable");
    }
    let mut result = vec![];
    let mut seen = HashSet::new();
    for response in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name(("DAV:", "response")))
    {
        if result.len() >= 512 {
            return Err("cloudLimit");
        }
        let value = response
            .children()
            .find(|n| n.has_tag_name(("DAV:", "href")))
            .and_then(|n| n.text())
            .ok_or("cloudUnavailable")?;
        let url = href(base, value)?;
        let relative = url.strip_prefix(base).ok_or("cloudInvalidServer")?;
        if relative.trim_end_matches('/').contains('/') {
            continue;
        }
        let prop = response
            .children()
            .filter(|n| n.has_tag_name(("DAV:", "propstat")))
            .find_map(|p| {
                let status = p
                    .children()
                    .find(|n| n.has_tag_name(("DAV:", "status")))
                    .and_then(|n| n.text())?;
                if status.split_whitespace().nth(1) != Some("200") {
                    return None;
                }
                p.children().find(|n| n.has_tag_name(("DAV:", "prop")))
            });
        let Some(prop) = prop else {
            continue;
        };
        let get = |name: &str| {
            prop.children()
                .find(|n| n.has_tag_name(("DAV:", name)))
                .and_then(|n| n.text())
                .unwrap_or("")
        };
        let directory = prop
            .children()
            .find(|n| n.has_tag_name(("DAV:", "resourcetype")))
            .is_some_and(|n| n.children().any(|n| n.has_tag_name(("DAV:", "collection"))));
        if !seen.insert(url.clone()) {
            return Err("cloudUnavailable");
        }
        result.push(Entry {
            url,
            name: title(get("displayname")),
            directory,
            mime: get("getcontenttype").into(),
            size: get("getcontentlength").parse().ok(),
        });
    }
    Ok(result)
}
fn list(net: &Net, url: &str, auth: &str) -> Result<Vec<Entry>> {
    entries(
        url,
        &net.bytes(
            "PROPFIND",
            url,
            auth,
            PROPS.as_bytes().to_vec(),
            "application/xml; charset=utf-8",
            JSON_LIMIT,
        )?,
    )
}
pub(super) fn run(
    net: &Net,
    c: &Connection,
    cache: &Path,
    command: &Command,
    path: &Path,
) -> Result<Outcome> {
    let base = root(&c.server)?;
    let auth = format!(
        "Basic {}",
        STANDARD.encode(format!("{}:{}", c.username, c.app_password))
    );
    let items = list(net, &base, &auth)?;
    let mut collections = vec![Collection {
        id: base.clone(),
        title: "WebDAV".into(),
    }];
    for item in items.iter().filter(|i| i.directory && i.url != base) {
        collections.push(Collection {
            id: format!("{}/", item.url.trim_end_matches('/')),
            title: if item.name.is_empty() {
                "Folder".into()
            } else {
                item.name.clone()
            },
        });
    }
    if let Command::Import {
        selected,
        preferences,
        ..
    } = command
    {
        if selected.is_empty()
            || selected.len() > 3
            || selected.iter().collect::<HashSet<_>>().len() != selected.len()
            || !selected
                .iter()
                .all(|id| collections.iter().any(|c| &c.id == id))
        {
            return Err("cloudChooseFolder");
        }
        let mut import = Importer::new(cache, preferences)?;
        let mut seen = HashSet::new();
        'folders: for folder in selected {
            let files = if folder == &base {
                items
                    .iter()
                    .map(|e| Entry {
                        url: e.url.clone(),
                        name: e.name.clone(),
                        directory: e.directory,
                        mime: e.mime.clone(),
                        size: e.size,
                    })
                    .collect()
            } else {
                list(net, folder, &auth)?
            };
            for item in files {
                if item.directory
                    || !image_type(&item.mime)
                    || item.size.is_none_or(|s| s > super::super::MAX_SOURCE)
                    || !seen.insert(item.url.clone())
                {
                    continue;
                }
                let bytes = net.bytes(
                    "GET",
                    &item.url,
                    &auth,
                    vec![],
                    "",
                    super::super::MAX_SOURCE,
                )?;
                import.add(
                    bytes,
                    if item.name.is_empty() {
                        "WebDAV photo"
                    } else {
                        &item.name
                    },
                )?;
                if import.full() {
                    break 'folders;
                }
            }
        }
        retain_identity(path, Provider::Webdav, c.fingerprint())?;
        return import.finish().map(Outcome::Imported);
    }
    Ok(Outcome::Ready(Report {
        collections,
        connection: c.fingerprint(),
    }))
}
