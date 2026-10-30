//! CI only: isolated files and HTTP loopback fixtures, no Immich account or TV.
use super::*;
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
};
const ALBUM: &str = "11111111-1111-4111-8111-111111111111";
const ASSET: &str = "22222222-2222-4222-8222-222222222222";
const KEY: &str = "fixture-key-not-a-real-credential";
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "lumaglass-immich-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self(root)
    }
    fn credentials(&self) -> PathBuf {
        self.0.join("preferences.immich.json")
    }
    fn put(&self) -> Connection {
        let c = Connection {
            schema: 1,
            server: "https://fixture.invalid".into(),
            api_key: KEY.into(),
            albums: vec![],
        };
        crate::storage::write_atomic(&self.credentials(), &serde_json::to_vec(&c).unwrap())
            .unwrap();
        c
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[derive(Clone)]
struct Request {
    method: String,
    path: String,
    key: Option<String>,
    body: Vec<u8>,
}
struct Response {
    code: u16,
    body: Vec<u8>,
    headers: String,
}
impl Response {
    fn json(value: Value) -> Self {
        Self {
            code: 200,
            body: serde_json::to_vec(&value).unwrap(),
            headers: "Content-Type: application/json\r\n".into(),
        }
    }
    fn bytes(body: Vec<u8>) -> Self {
        Self {
            code: 200,
            body,
            headers: "Content-Type: image/png\r\n".into(),
        }
    }
}
struct Server {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Server {
    fn new(handler: impl Fn(&Request) -> Response + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/api", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let received = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Some((mut stream, request)) = read_request(stream) {
                            received.lock().unwrap().push(request.clone());
                            let response = handler(&request);
                            let header = format!(
                                "HTTP/1.1 {} Fixture\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n",
                                response.code,
                                response.body.len(),
                                response.headers
                            );
                            let _ = stream
                                .write_all(header.as_bytes())
                                .and_then(|_| stream.write_all(&response.body));
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            requests,
            stop,
            thread: Some(thread),
        }
    }
    fn client(&self) -> Client {
        Client::at(self.url.clone(), KEY.into()).unwrap()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
fn read_request(stream: TcpStream) -> Option<(TcpStream, Request)> {
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.into();
    let path = parts.next()?.into();
    let mut key = None;
    let mut size = 0;
    let mut line = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line).ok()?;
        if line == "\r\n" {
            break;
        }
        if line.is_empty() {
            return None;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("x-api-key") {
                key = Some(value.trim().into());
            }
            if name.eq_ignore_ascii_case("content-length") {
                size = value.trim().parse::<usize>().ok()?;
            }
        }
    }
    if size > 8192 {
        return None;
    }
    let mut body = vec![0; size];
    reader.read_exact(&mut body).ok()?;
    Some((
        reader.into_inner(),
        Request {
            method,
            path,
            key,
            body,
        },
    ))
}
fn albums() -> Value {
    json!([{"id":ALBUM,"albumName":"Fixture album"}])
}
fn asset() -> Value {
    json!({"id":ASSET,"type":"IMAGE","isTrashed":false,"isOffline":false,"isArchived":false,"visibility":"timeline"})
}
#[test]
fn production_origins_cannot_leak_credentials_to_urls_or_plaintext() {
    for server in [
        "http://127.0.0.1:2283",
        "file:///tmp/a",
        "https://user:pass@example.test",
        "https://example.test?key=x",
        "https://example.test/#fragment",
        "https://example.test/prefix",
        "https://example.test/%2f",
        "https://example.test\\@other.test",
        "https://example.test\n",
    ] {
        assert!(api_base(server).is_err(), "{server:?}");
    }
    assert_eq!(
        api_base("https://example.test").unwrap(),
        "https://example.test/api"
    );
    assert_eq!(
        api_base("https://example.test:2283/api/").unwrap(),
        "https://example.test:2283/api"
    );
}
#[test]
fn private_connection_requires_owned_regular_restricted_file_and_bounded_valid_json() {
    let root = Directory::new();
    let c = root.put();
    assert_eq!(
        Connection::read(&root.credentials()).unwrap().fingerprint(),
        c.fingerprint()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        fs::set_permissions(root.credentials(), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Connection::read(&root.credentials()).is_err());
        fs::set_permissions(root.credentials(), fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.0.join("alias.immich.json");
        symlink(root.credentials(), &link).unwrap();
        assert!(Connection::read(&link).is_err());
    }
    crate::storage::write_atomic(&root.credentials(), &vec![b' '; 4097]).unwrap();
    assert!(Connection::read(&root.credentials()).is_err());
    crate::storage::write_atomic(
        &root.credentials(),
        br#"{"schema":1,"server":"https://example.test","api_key":"short"}"#,
    )
    .unwrap();
    assert!(Connection::read(&root.credentials()).is_err());
}
#[test]
fn uuids_and_selections_cannot_escape_routes_or_request_the_whole_library() {
    for id in [
        "../private",
        "https://elsewhere",
        "11111111-1111-1111-8111-111111111111",
        "11111111-1111-4111-c111-111111111111",
    ] {
        assert!(!uuid(id));
    }
    assert!(uuid(ALBUM));
    assert!(!selection_valid(&[ALBUM.into(), ALBUM.into()]));
    let server = Server::new(|_| panic!("No request is allowed"));
    let root = Directory::new();
    assert!(matches!(
        server
            .client()
            .import(&root.0, &[], &Preferences::default()),
        Err("immichSelectAlbum")
    ));
    assert!(server.requests.lock().unwrap().is_empty());
}
#[test]
fn supported_versions_use_scoped_modern_or_legacy_search_and_ignore_ineligible_assets() {
    for (major, minor, modern) in [(2, 5, false), (3, 1, false), (3, 3, true)] {
        let server = Server::new(move |request| match request.path.as_str() {
            "/api/server/version" => {
                assert!(request.key.is_none());
                Response::json(json!({"major":major,"minor":minor,"patch":5}))
            }
            "/api/search/metadata" => {
                assert_eq!(request.method, "POST");
                assert_eq!(request.key.as_deref(), Some(KEY));
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(body["size"], 12);
                if modern {
                    assert_eq!(body["filter"]["albumIds"]["any"], json!([ALBUM]));
                    assert_eq!(body["filter"]["type"]["eq"], "IMAGE");
                    assert_eq!(body["filter"]["visibility"]["eq"], "timeline");
                    assert_eq!(body["filter"]["trashedAt"]["eq"], Value::Null);
                } else {
                    assert_eq!(body["albumIds"], json!([ALBUM]));
                    assert_eq!(body["page"], 1);
                }
                let good = asset();
                let mut video = good.clone();
                video["type"] = json!("VIDEO");
                let mut locked = good.clone();
                locked["visibility"] = json!("locked");
                let mut trash = good.clone();
                trash["isTrashed"] = json!(true);
                let mut offline = good.clone();
                offline["isOffline"] = json!(true);
                let mut unknown = good.clone();
                unknown.as_object_mut().unwrap().remove("isArchived");
                Response::json(
                    json!({"assets":{"items":[good.clone(),good,video,locked,trash,offline,unknown]}}),
                )
            }
            _ => panic!("Unexpected route"),
        });
        let mut client = server.client();
        assert!(client.version().is_ok());
        assert_eq!(client.modern, modern);
        assert_eq!(client.assets(ALBUM).unwrap(), vec![ASSET.to_string()]);
    }
}
#[test]
fn unknown_api_versions_stop_before_any_authenticated_request() {
    for (major, minor) in [(1, 99), (2, 4), (4, 0)] {
        let server = Server::new(move |request| {
            assert_eq!(request.path, "/api/server/version");
            assert!(request.key.is_none());
            Response::json(json!({"major":major,"minor":minor,"patch":0}))
        });
        let mut client = server.client();
        assert!(matches!(client.version(), Err("immichUnsupported")));
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }
}
#[test]
fn permission_errors_and_redirects_are_sanitized_and_never_followed() {
    for code in [401, 403, 302, 500] {
        let server = Server::new(move |_| Response {
            code,
            body: format!("private album token {KEY}").into_bytes(),
            headers: "Location: http://127.0.0.1:1/stolen\r\n".into(),
        });
        let result = server.client().albums();
        assert!(matches!(result, Err("immichDenied" | "immichUnavailable")));
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }
}
#[test]
fn response_album_and_operation_limits_are_bounded() {
    let oversized = Server::new(|_| Response::bytes(vec![b' '; JSON_LIMIT as usize + 1]));
    assert!(matches!(oversized.client().albums(), Err("immichLimit")));
    let too_many = Server::new(|_| {
        Response::json(Value::Array(
            (0..513)
                .map(|_| json!({"id":ALBUM,"albumName":"fixture"}))
                .collect(),
        ))
    });
    assert!(matches!(too_many.client().albums(), Err("immichLimit")));
    let server = Server::new(|_| panic!("Expired operations cannot send requests"));
    let mut client = server.client();
    client.started = Instant::now() - Duration::from_secs(61);
    assert!(matches!(client.albums(), Err("immichLimit")));
}
#[test]
fn failed_import_preserves_existing_catalog_and_never_falls_back_to_unscoped_search() {
    let root = Directory::new();
    crate::storage::write_atomic(&root.0.join("catalog.json"), b"[]").unwrap();
    let server = Server::new(|request| match request.path.as_str() {
        "/api/albums" => Response::json(albums()),
        "/api/search/metadata" => Response::json(json!({"assets":{"items":[asset()]}})),
        _ => Response {
            code: 403,
            body: b"denied".to_vec(),
            headers: String::new(),
        },
    });
    let mut client = server.client();
    client.modern = true;
    assert!(matches!(
        client.import(&root.0, &[ALBUM.into()], &Preferences::default()),
        Err("immichDenied")
    ));
    assert_eq!(fs::read(root.0.join("catalog.json")).unwrap(), b"[]");
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[2].path,
        format!("/api/assets/{ASSET}/thumbnail?size=preview")
    );
    assert_eq!(requests[2].method, "GET");
}
#[test]
fn imported_photo_uses_existing_offline_derivatives_and_no_remote_metadata() {
    let root = Directory::new();
    let mut buffer = std::io::Cursor::new(vec![]);
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        32,
        18,
        image::Rgb([20, 80, 120]),
    ))
    .write_to(&mut buffer, image::ImageFormat::Png)
    .unwrap();
    let bytes = buffer.into_inner();
    let server = Server::new(move |request| match request.path.as_str() {
        "/api/albums" => Response::json(albums()),
        "/api/search/metadata" => Response::json(json!({"assets":{"items":[asset()]}})),
        _ => {
            assert_eq!(
                request.path,
                format!("/api/assets/{ASSET}/thumbnail?size=preview")
            );
            Response::bytes(bytes.clone())
        }
    });
    let mut client = server.client();
    client.modern = true;
    let imported = client
        .import(&root.0, &[ALBUM.into()], &Preferences::default())
        .unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].title, "Immich 1");
    for suffix in [".jpg", "-blur-soft.jpg", "-blur-strong.jpg", ".png"] {
        assert!(root.0.join(format!("{}{suffix}", imported[0].id)).is_file());
    }
    let catalog = fs::read_to_string(root.0.join("catalog.json")).unwrap();
    for private in [ALBUM, ASSET, KEY, "http://", "Fixture album"] {
        assert!(!catalog.contains(private));
    }
}
#[test]
fn changed_identity_refuses_stale_import_and_local_forget_never_erases_cached_photos() {
    let root = Directory::new();
    let c = root.put();
    let cache = root.0.join("cache");
    fs::create_dir(&cache).unwrap();
    fs::write(cache.join("retained.jpg"), b"offline-photo").unwrap();
    let changed = Command::Import {
        selected: vec![ALBUM.into()],
        connection: c.fingerprint().wrapping_add(1),
        preferences: Preferences::default(),
    };
    assert!(matches!(
        run(&root.credentials(), &cache, changed),
        Err("immichChanged")
    ));
    assert!(matches!(
        run(
            &root.credentials(),
            &cache,
            Command::Forget {
                connection: Some(c.fingerprint())
            }
        ),
        Ok(Outcome::Forgotten)
    ));
    assert!(!root.credentials().exists());
    assert_eq!(
        fs::read(cache.join("retained.jpg")).unwrap(),
        b"offline-photo"
    );
}
#[test]
fn native_selection_requires_an_explicit_import_and_forget_selects_cancel_first() {
    use crate::state::{State, Subpage, Target};
    let mut state = State::new(Default::default());
    state.page = crate::state::Page::Settings;
    state.subpage = Some(Subpage::Immich);
    state.immich.connection = Some(1);
    state.immich.albums = vec![Album {
        id: ALBUM.into(),
        title: "Private name".into(),
    }];
    state.immich_toggle(ALBUM);
    assert_eq!(state.immich.selected, vec![ALBUM.to_string()]);
    assert!(state.immich.pending.is_none());
    assert!(state.immich_enabled(1));
    state.immich_action(2);
    assert!(state.immich.confirming);
    assert_eq!(state.focus, Target::Immich(3));
    assert!(!state.immich_enabled(1));
    state.back();
    assert!(!state.immich.confirming);
    assert_eq!(state.subpage, Some(Subpage::Immich));
    state.immich_action(1);
    assert!(state.wallpapers.loading);
    assert!(state.immich.importing);
    assert!(matches!(
        state.immich.pending,
        Some((_, Command::Import { .. }))
    ));
}

#[test]
fn in_flight_import_cannot_lose_a_newly_selected_wallpaper() {
    use crate::state::{State, Target};
    let mut state = State::new(Default::default());
    state.immich.importing = true;
    for target in [
        Target::Photo(0),
        Target::PhotoStock,
        Target::Background(0),
        Target::RotationNext,
    ] {
        assert!(!state.target_enabled(&target));
    }
    assert!(state.rotate_background().is_empty());
}
