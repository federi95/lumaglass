//! CI-only protocol fixtures: no account, token or personal service is contacted.
use super::*;
use serde_json::json;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "lumaglass-cloud-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self(p)
    }
    fn config(&self, p: Provider, c: &Connection) -> PathBuf {
        let path = credentials_path(&self.0.join("preferences.json"), p);
        crate::storage::write_atomic(&path, &serde_json::to_vec(c).unwrap()).unwrap();
        path
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn connection(p: Provider) -> Connection {
    Connection {
        schema: 1,
        export_folder: if p == Provider::GooglePhotos {
            "/private/fixture".into()
        } else {
            String::new()
        },
        server: if p == Provider::Webdav {
            "https://cloud.example/dav/".into()
        } else {
            String::new()
        },
        username: if p == Provider::Webdav {
            "fixture-user".into()
        } else {
            String::new()
        },
        app_password: if p == Provider::Webdav {
            "fixture-password".into()
        } else {
            String::new()
        },
        client_id: if p == Provider::Drive {
            "fixture.apps.googleusercontent.com".into()
        } else {
            String::new()
        },
        client_secret: if p == Provider::Drive {
            "fixture-secret".into()
        } else {
            String::new()
        },
        refresh_token: if p == Provider::Drive {
            "fixture-refresh".into()
        } else {
            String::new()
        },
        scope: if p == Provider::Drive {
            p.scope().into()
        } else {
            String::new()
        },
        folders: if p == Provider::Drive {
            vec!["folder".into()]
        } else {
            vec![]
        },
    }
}
#[derive(Clone)]
struct Request {
    method: String,
    path: String,
    headers: String,
    body: String,
}
struct Server {
    base: String,
    requests: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(handler: impl Fn(&Request) -> (u16, &'static str, Vec<u8>) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let logged = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let join = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let mut data = vec![];
                        let mut b = [0u8; 1024];
                        let end = loop {
                            let n = stream.read(&mut b).unwrap_or(0);
                            if n == 0 {
                                break None;
                            }
                            data.extend_from_slice(&b[..n]);
                            if let Some(i) = data.windows(4).position(|v| v == b"\r\n\r\n") {
                                break Some(i + 4);
                            }
                            if data.len() > 32768 {
                                break None;
                            }
                        };
                        let Some(end) = end else {
                            continue;
                        };
                        let headers = String::from_utf8_lossy(&data[..end]).into_owned();
                        let length = headers
                            .lines()
                            .find_map(|l| {
                                l.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if length > 32768 {
                            continue;
                        }
                        while data.len() < end + length {
                            let n = stream.read(&mut b).unwrap_or(0);
                            if n == 0 {
                                break;
                            }
                            data.extend_from_slice(&b[..n]);
                        }
                        let mut start = headers.lines().next().unwrap().split_whitespace();
                        let req = Request {
                            method: start.next().unwrap().into(),
                            path: start.next().unwrap().into(),
                            headers: headers.clone(),
                            body: String::from_utf8_lossy(&data[end..]).into_owned(),
                        };
                        logged.lock().unwrap().push(req.clone());
                        let (status, mime, body) = handler(&req);
                        let response = format!(
                            "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nContent-Type: {mime}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.write_all(&body);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            base,
            requests,
            stop,
            join: Some(join),
        }
    }
    fn net(&self) -> Net {
        let mut n = Net::new();
        n.mock = Some(self.base.clone());
        n
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.base.trim_start_matches("http://"));
        if let Some(j) = self.join.take() {
            j.join().unwrap();
        }
    }
}
fn png() -> Vec<u8> {
    let mut data = std::io::Cursor::new(vec![]);
    image::DynamicImage::new_rgb8(8, 8)
        .write_to(&mut data, image::ImageFormat::Png)
        .unwrap();
    data.into_inner()
}
fn json_response(v: Value) -> (u16, &'static str, Vec<u8>) {
    (200, "application/json", v.to_string().into_bytes())
}
fn grant() -> (u16, &'static str, Vec<u8>) {
    json_response(json!({"access_token":"fixture-access","token_type":"Bearer"}))
}
#[test]
fn credentials_are_private_and_changed_identity_is_rejected() {
    let f = Files::new();
    for p in Provider::ALL {
        let c = connection(p);
        let path = f.config(p, &c);
        let loaded = Connection::read(&path, p).unwrap();
        assert_eq!(loaded.fingerprint(), c.fingerprint());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(matches!(
                Connection::read(&path, p),
                Err("cloudCredentials")
            ));
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let mut changed = connection(p);
        changed.schema = 2;
        crate::storage::write_atomic(&path, &serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(Connection::read(&path, p).is_err());
    }
}
#[test]
fn webdav_rejects_foreign_hrefs_traversal_and_xml_entities() {
    let body = |href: &str| {
        format!(
            "<d:multistatus xmlns:d=\"DAV:\"><d:response><d:href>{href}</d:href><d:propstat><d:prop><d:resourcetype/></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"
        )
    };
    for href in [
        "https://evil.example/dav/a.jpg",
        "//evil.example/a.jpg",
        "/dav/%2e%2e/secret",
        "/dav/folder%2fsecret",
        "/other/photo.jpg",
    ] {
        assert!(webdav::entries("https://cloud.example/dav/", body(href).as_bytes()).is_err());
    }
    assert!(webdav::entries("https://cloud.example/dav/",b"<!DOCTYPE d [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><d:multistatus xmlns:d='DAV:'>&x;</d:multistatus>").is_err());
    assert!(webdav::root("http://cloud.example/dav/").is_err());
    assert!(webdav::root("https://user:pass@cloud.example/dav/").is_err());
}
#[test]
fn webdav_import_uses_propfind_and_get_and_keeps_originals_remote() {
    let f = Files::new();
    let c = connection(Provider::Webdav);
    let path = f.config(Provider::Webdav, &c);
    let image = png();
    let len = image.len();
    let xml = format!(
        "<d:multistatus xmlns:d='DAV:'><d:response><d:href>/dav/photo.png</d:href><d:propstat><d:prop><d:displayname>Photo &amp; test</d:displayname><d:resourcetype/><d:getcontenttype>image/png</d:getcontenttype><d:getcontentlength>{len}</d:getcontentlength></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"
    );
    let server = Server::new(move |r| {
        if r.method == "PROPFIND" {
            (207, "application/xml", xml.as_bytes().to_vec())
        } else {
            (200, "image/png", image.clone())
        }
    });
    let command = Command::Import {
        connection: c.fingerprint(),
        selected: vec![c.server.clone()],
        preferences: Preferences::default(),
    };
    let out = webdav::run(&server.net(), &c, &f.0.join("cache"), &command, &path).unwrap();
    let Outcome::Imported(photos) = out else {
        panic!("expected local import")
    };
    assert_eq!(photos.len(), 1);
    assert_eq!(photos[0].title, "Photo & test");
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|r| matches!(r.method.as_str(), "PROPFIND" | "GET"))
    );
    assert!(requests[0].headers.to_lowercase().contains("depth: 1"));
}
#[test]
fn drive_import_uses_fixed_metadata_and_download_endpoints() {
    let f = Files::new();
    let c = connection(Provider::Drive);
    let path = f.config(Provider::Drive, &c);
    let image = png();
    let len = image.len().to_string();
    let server = Server::new(move |r| {
        if r.path.starts_with("/oauth2.googleapis.com/") {
            assert!(r.body.contains("grant_type=refresh_token"));
            return grant();
        }
        if r.path.contains("/files/folder?") {
            return json_response(
                json!({"id":"folder","name":"Fixture folder","mimeType":"application/vnd.google-apps.folder","trashed":false}),
            );
        }
        if r.path.contains("/files/photo?") {
            return (200, "image/png", image.clone());
        }
        json_response(
            json!({"files":[{"id":"photo","name":"Drive photo","mimeType":"image/png","size":len,"trashed":false,"capabilities":{"canDownload":true}}]}),
        )
    });
    let cmd = Command::Import {
        connection: c.fingerprint(),
        selected: vec!["folder".into()],
        preferences: Preferences::default(),
    };
    let out = google::drive(&server.net(), &c, &f.0.join("cache"), &cmd, &path).unwrap();
    assert!(matches!(out,Outcome::Imported(ref p)if p.len()==1));
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(
        requests
            .iter()
            .all(|r| r.method == "GET" || r.path.starts_with("/oauth2.googleapis.com/token"))
    );
    assert!(requests[3].headers.contains("fixture-access"));
}
#[test]
fn forget_does_not_download_and_retains_existing_offline_files() {
    let f = Files::new();
    let c = connection(Provider::Drive);
    let _ = f.config(Provider::Drive, &c);
    let cache = f.0.join("cache");
    fs::create_dir(&cache).unwrap();
    fs::write(cache.join("offline-marker"), b"retained").unwrap();
    assert!(matches!(
        run(
            &f.0.join("preferences.json"),
            &cache,
            Provider::Drive,
            Command::Forget {
                connection: Some(c.fingerprint())
            }
        ),
        Ok(Outcome::Forgotten)
    ));
    assert!(cache.join("offline-marker").exists());
}
#[test]
fn user_photo_export_import_is_local_and_manifest_paths_cannot_escape() {
    let f = Files::new();
    let export = f.0.join("export");
    fs::create_dir(&export).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&export, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let original = png();
    crate::storage::write_atomic(&export.join("photo-01.jpg"), &original).unwrap();
    let manifest = export.join("google-photos-export.json");
    crate::storage::write_atomic(
        &manifest,
        &json!({"schema":1,"source":"google-photos-picker","files":["photo-01.jpg"]})
            .to_string()
            .into_bytes(),
    )
    .unwrap();
    let mut c = connection(Provider::GooglePhotos);
    c.export_folder = export.to_string_lossy().into_owned();
    let path = f.config(Provider::GooglePhotos, &c);
    let cache = f.0.join("cache");
    let ready = run(
        &f.0.join("preferences.json"),
        &cache,
        Provider::GooglePhotos,
        Command::Browse,
    )
    .unwrap();
    assert!(matches!(ready, Outcome::Ready(_)));
    let cmd = Command::Import {
        connection: c.fingerprint(),
        selected: vec![],
        preferences: Preferences::default(),
    };
    let imported = photos_export::run(&c, &cache, &cmd, &path).unwrap();
    assert!(matches!(imported,Outcome::Imported(ref p)if p.len()==1));
    assert_eq!(fs::read(export.join("photo-01.jpg")).unwrap(), original);
    let catalog = fs::read(cache.join("catalog.json")).unwrap();
    crate::storage::write_atomic(
        &manifest,
        &json!({"schema":1,"source":"google-photos-picker","files":["../secret.jpg"]})
            .to_string()
            .into_bytes(),
    )
    .unwrap();
    assert!(matches!(
        photos_export::run(&c, &cache, &cmd, &path),
        Err("cloudExportInvalid")
    ));
    assert_eq!(fs::read(cache.join("catalog.json")).unwrap(), catalog);
}
