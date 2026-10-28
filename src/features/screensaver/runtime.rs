//! Dedicated native saver process. No launcher, networking, preference writer or glass.
use super::system::{Config, ID};
use crate::{
    raster::Size,
    renderer::{Draw, Renderer, TextStyle},
    state::Rect,
};
use sdl2::{
    event::{Event, WindowEvent},
    video::GLProfile,
};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
use time::OffsetDateTime;

pub fn entry() -> bool {
    let args: Vec<_> = std::env::args().collect();
    let alias = args
        .first()
        .and_then(|s| Path::new(s).file_name())
        .is_some_and(|s| s == "lumaglass-screensaver");
    let metadata = cfg!(feature = "webos")
        && args
            .get(1)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .is_some_and(|v| v["appId"].as_str() == Some(ID) || v["nid"].as_str() == Some(ID));
    if !alias && !metadata && !args.iter().any(|s| s == "--system-screensaver") {
        return false;
    }
    if let Err(error) = run(&args) {
        eprintln!("Lumaglass system screensaver: {error}");
        std::process::exit(1);
    }
    true
}
fn read_config(path: &Path) -> Result<Config, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 32768 {
        return Err("Invalid saver configuration".into());
    }
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
    };
    #[cfg(not(unix))]
    let file = fs::File::open(path);
    let mut bytes = vec![];
    file.map_err(|e| e.to_string())?
        .take(32769)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 32768 {
        return Err("Oversized saver configuration".into());
    }
    let config: Config =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid saver configuration")?;
    if !config.valid() {
        return Err("Invalid saver configuration".into());
    }
    Ok(config)
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Power {
    Saver,
    Active,
    Other,
}
fn power(raw: &str) -> Power {
    match raw {
        "Screen Saver" => Power::Saver,
        "Active" => Power::Active,
        _ => Power::Other,
    }
}
fn physical(event: &Event) -> bool {
    match event {
        Event::KeyDown {
            keycode,
            scancode,
            repeat: false,
            ..
        } => keycode.is_some() || scancode.is_some(),
        Event::MouseButtonDown { .. } | Event::MouseWheel { .. } => true,
        Event::MouseMotion { xrel, yrel, .. } => *xrel != 0 || *yrel != 0,
        _ => false,
    }
}
fn scene(
    config: &Config,
    now: OffsetDateTime,
    elapsed: u64,
    image: Option<Arc<crate::app_artwork::Artwork>>,
) -> Vec<Draw> {
    let rect = Rect::new(0., 0., 1920., 1080.);
    let mut out = vec![Draw::Shape(rect, 0., 0., [0., 0., 0., 1.])];
    if let Some(image) = image {
        out.push(Draw::AppImage(image, rect, [1.; 4]));
        out.push(Draw::Shape(rect, 0., 0., [0., 0., 0., 0.72]));
    }
    let step = elapsed / 15;
    out.push(Draw::Text(
        config.language.clock(now, config.hour_24, false),
        Rect::new(
            120. + ((step * 317) % 1240) as f32,
            120. + ((step * 193) % 650) as f32,
            440.,
            130.,
        ),
        TextStyle::new(76.),
        [0.8, 0.8, 0.8, 0.7],
        true,
    ));
    out
}
fn run(args: &[String]) -> Result<(), String> {
    let webos = cfg!(feature = "webos");
    if !webos && Path::new("/etc/starfish-release").exists() {
        return Err("Desktop saver cannot run on a TV".into());
    }
    let value = |name: &str| {
        args.iter()
            .position(|s| s == name)
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
    };
    if webos {
        unsafe {
            std::env::set_var("APPID", ID);
            std::env::set_var("SDL_VIDEO_WAYLAND_WMCLASS", ID);
        }
        sdl2::hint::set("SDL_VIDEO_ALLOW_SCREENSAVER", "1");
        sdl2::hint::set("SDL_WEBOS_ACCESS_POLICY_KEYS_BACK", "true");
        sdl2::hint::set("SDL_WEBOS_ACCESS_POLICY_KEYS_EXIT", "true");
    }
    // Resolve time while the process is still single-threaded.
    let offset = OffsetDateTime::now_local()
        .map(|n| n.offset())
        .unwrap_or(time::UtcOffset::UTC);
    let root = if webos {
        PathBuf::from("/usr/palm/applications").join(ID)
    } else {
        value("--saver-root").ok_or("Specify --saver-root for a desktop capture")?
    };
    let config = read_config(&root.join("settings.json"))?;
    if webos && power(&crate::platform::screensaver_power(false)?) != Power::Saver {
        return Err("LG has not entered its screen saver power state".into());
    }
    let assets = if webos {
        root.join("assets")
    } else {
        value("--assets")
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".build/assets"))
    };
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let attributes = video.gl_attr();
    attributes.set_context_profile(if webos {
        GLProfile::GLES
    } else {
        GLProfile::Core
    });
    if webos {
        attributes.set_context_version(2, 0);
    } else {
        attributes.set_context_version(3, 2);
    }
    attributes.set_double_buffer(true);
    attributes.set_depth_size(0);
    attributes.set_stencil_size(0);
    attributes.set_alpha_size(8);
    let mut builder = video.window("Lumaglass screensaver", 1920, 1080);
    builder.opengl().position_centered();
    if webos {
        builder.fullscreen();
    }
    let window = builder.build().map_err(|e| e.to_string())?;
    let _context = window.gl_create_context()?;
    gl::load_with(|n| video.gl_get_proc_address(n) as *const _);
    let _ = video.gl_set_swap_interval(1);
    let mut renderer = Renderer::new(&assets.join("font.ttf"), webos, 0.)?;
    let mut pump = sdl.event_pump()?;
    let decoder = config
        .photos
        .then(|| crate::features::wallpapers::Decoder::new(root.join("photos")));
    let (wake, requests) = mpsc::sync_channel(1);
    let (results, power_results) = mpsc::sync_channel(1);
    if webos {
        std::thread::spawn(move || {
            loop {
                let wake = match requests.recv_timeout(Duration::from_secs(2)) {
                    Ok(()) => true,
                    Err(mpsc::RecvTimeoutError::Timeout) => false,
                    Err(_) => break,
                };
                let result = crate::platform::screensaver_power(wake).map(|s| power(&s));
                if results.send(result).is_err() {
                    break;
                }
            }
        });
    }
    let started = Instant::now();
    let mut last_frame = None;
    let mut pending = None;
    let mut image = None;
    let mut image_key = None;
    let mut redraw = true;
    let mut focused = false;
    let capture = if webos { None } else { value("--screenshot") };
    let elapsed = if webos {
        0
    } else {
        args.iter()
            .position(|s| s == "--saver-elapsed")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
            .min(86400)
    };
    loop {
        let seconds = started.elapsed().as_secs() + elapsed;
        if let Some(decoder) = &decoder {
            let key = &config.images[(seconds / 30) as usize % config.images.len()];
            if pending.as_ref() != Some(key)
                && image_key.as_ref() != Some(key)
                && decoder.request(key.clone())
            {
                pending = Some(key.clone());
            }
            while let Ok((key, decoded)) = decoder.receiver.try_recv() {
                if pending.as_ref() == Some(&key) {
                    pending = None;
                    image_key = Some(key);
                    image = decoded;
                    redraw = true;
                }
            }
        }
        while let Ok(result) = power_results.try_recv() {
            if result.is_ok_and(|p| p != Power::Saver) {
                return Ok(());
            }
        }
        let mut event = crate::sdl_events::wait(&mut pump, 100, webos);
        while let Some(next) = event {
            match next {
                Event::Quit { .. }
                | Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => return Ok(()),
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => focused = true,
                Event::Window {
                    win_event: WindowEvent::Hidden | WindowEvent::Minimized,
                    ..
                } => return Ok(()),
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } if focused => return Ok(()),
                Event::Window {
                    win_event: WindowEvent::Exposed,
                    ..
                } => redraw = true,
                _ => {}
            }
            if physical(&next)
                && (!matches!(next, Event::MouseMotion { .. })
                    || started.elapsed() >= Duration::from_millis(500))
            {
                if webos {
                    let _ = wake.try_send(());
                } else {
                    return Ok(());
                }
            }
            event = crate::sdl_events::poll(&mut pump, webos);
        }
        if last_frame != Some(seconds) || redraw {
            let now = OffsetDateTime::now_utc().to_offset(offset);
            let draw = scene(&config, now, seconds, image.clone());
            let (w, h) = window.drawable_size();
            renderer.render(&draw, w, h, None);
            window.gl_swap_window();
            last_frame = Some(seconds);
            redraw = false;
            if let Some(path) = &capture
                && (decoder.is_none()
                    || image_key.is_some()
                    || started.elapsed() >= Duration::from_secs(5))
            {
                let stats = renderer.capture(&draw, path, Size::HD)?;
                fs::write(
                    path.with_extension("json"),
                    serde_json::to_vec(&stats).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn power_guard_never_treats_standby_or_unknown_as_saver() {
        assert_eq!(power("Screen Saver"), Power::Saver);
        for raw in [
            "",
            "screen saver",
            "Screen Off",
            "Suspend",
            "Standby",
            "Active",
        ] {
            assert_ne!(power(raw), Power::Saver);
        }
    }
    #[test]
    fn clock_moves_without_glass_or_launcher_layers() {
        let config = Config::default();
        let a = scene(&config, OffsetDateTime::UNIX_EPOCH, 0, None);
        let b = scene(&config, OffsetDateTime::UNIX_EPOCH, 15, None);
        assert_eq!(a.len(), 2);
        let Draw::Text(_, r, ..) = &a[1] else {
            panic!()
        };
        let Draw::Text(_, s, ..) = &b[1] else {
            panic!()
        };
        assert!(r.x != s.x && r.y != s.y);
        assert!(s.x + s.w <= 1920. && s.y + s.h <= 1080.);
    }
}
