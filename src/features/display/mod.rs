//! Timed 4K trials with a watchdog outside SDL and atomic confirmation ownership.
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
pub static RESTART: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub four_k: bool,
}
pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Preferences, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or_default())
}
#[derive(Serialize, Deserialize)]
struct Request {
    token: String,
    staged: bool,
    trial: bool,
}
fn pending(path: &Path) -> PathBuf {
    path.with_extension("display-request.json")
}
fn fallback(path: &Path) -> PathBuf {
    path.with_extension("display-1080p")
}
fn read(path: &Path) -> Option<Request> {
    use std::io::Read;
    let mut bytes = vec![];
    std::fs::File::open(path)
        .ok()?
        .take(4097)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let request: Request = serde_json::from_slice(&bytes).ok()?;
    (request.token.len() <= 80
        && !request.token.is_empty()
        && request
            .token
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-'))
    .then_some(request)
}
fn write(path: &Path, request: &Request) -> Result<(), String> {
    crate::storage::write_atomic(
        path,
        &serde_json::to_vec(request).map_err(|_| "Invalid display request")?,
    )
}
pub fn stage(path: &Path, four_k: bool) -> Result<(), String> {
    if four_k {
        let token = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "Clock unavailable")?
                .as_nanos()
        );
        write(
            &pending(path),
            &Request {
                token,
                staged: true,
                trial: true,
            },
        )?;
    } else {
        crate::storage::write_atomic(&fallback(path), b"1080p\n")?;
        if pending(path).exists() {
            std::fs::remove_file(pending(path)).map_err(|_| "Cannot cancel display trial")?;
        }
    }
    RESTART.store(true, Ordering::Relaxed);
    Ok(())
}
pub struct Guard {
    path: PathBuf,
    token: String,
    pub trial: bool,
    pub four_k: bool,
    armed: bool,
}
impl Guard {
    pub fn begin(path: &Path, confirmed: bool, enabled: bool) -> Result<Self, String> {
        let mut guard = Self {
            path: path.into(),
            token: String::new(),
            trial: false,
            four_k: false,
            armed: false,
        };
        if !enabled {
            return Ok(guard);
        }
        let marker = pending(path);
        let request = if marker.exists() {
            match read(&marker) {
                Some(mut request) if request.staged => {
                    request.staged = false;
                    request
                }
                _ => {
                    crate::storage::write_atomic(&fallback(path), b"1080p\n")?;
                    let _ = std::fs::remove_file(marker);
                    return Ok(guard);
                }
            }
        } else if confirmed && !fallback(path).exists() {
            Request {
                token: format!("{}-startup", std::process::id()),
                staged: false,
                trial: false,
            }
        } else {
            return Ok(guard);
        };
        write(&marker, &request)?;
        Command::new(std::env::current_exe().map_err(|_| "Executable unavailable")?)
            .arg("--display-watchdog")
            .arg(path)
            .arg(&request.token)
            .arg("--")
            .args(std::env::args_os().skip(1))
            .spawn()
            .map_err(|_| "Display watchdog unavailable")?;
        guard.token = request.token;
        guard.trial = request.trial;
        guard.four_k = true;
        guard.armed = true;
        Ok(guard)
    }
    pub fn confirm(&mut self) -> bool {
        if !self.armed {
            return true;
        }
        let marker = pending(&self.path);
        if read(&marker).is_none_or(|r| r.token != self.token) {
            return false;
        }
        let claim = self
            .path
            .with_extension(format!("display-confirmed-{}", self.token));
        if std::fs::rename(&marker, &claim).is_err() {
            return false;
        }
        let _ = std::fs::remove_file(claim);
        let _ = std::fs::remove_file(fallback(&self.path));
        self.armed = false;
        self.trial = false;
        true
    }
    pub fn normal_exit(&mut self) {
        if self.armed {
            let _ = crate::storage::write_atomic(&fallback(&self.path), b"1080p\n");
            let _ = self.confirm();
            // An unconfirmed exit must still return to the safe mode next time.
            let _ = crate::storage::write_atomic(&fallback(&self.path), b"1080p\n");
        }
    }
}
pub fn restart() -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = Command::new(std::env::current_exe().map_err(|_| "Executable unavailable")?)
            .args(std::env::args_os().skip(1))
            .exec();
        Err(format!("Display restart failed: {error}"))
    }
    #[cfg(not(unix))]
    {
        Err("Display restart requires Unix".into())
    }
}
pub fn helper() -> bool {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.get(1).is_none_or(|a| a != "--display-watchdog") {
        return false;
    }
    #[cfg(unix)]
    {
        if args.len() < 5 || args[4] != "--" {
            return true;
        }
        let path = PathBuf::from(&args[2]);
        let token = args[3].to_string_lossy();
        // Only this process's actual parent can be terminated. No PID is accepted from input.
        let parent = unsafe { libc::getppid() };
        if parent <= 1 {
            return true;
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if read(&pending(&path)).is_none_or(|r| r.token != token) {
                return true;
            }
            if Instant::now() >= deadline || unsafe { libc::getppid() } != parent {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let claim = path.with_extension(format!("display-expired-{token}"));
        if std::fs::rename(pending(&path), &claim).is_err() {
            return true;
        }
        if crate::storage::write_atomic(&fallback(&path), b"1080p\n").is_err() {
            return true;
        }
        let _ = std::fs::remove_file(claim);
        if unsafe { libc::getppid() } == parent {
            // Async-signal-safe kernel operation against the verified actual parent.
            unsafe {
                libc::kill(parent, libc::SIGTERM);
            }
            std::thread::sleep(Duration::from_millis(500));
            if unsafe { libc::getppid() } == parent {
                unsafe {
                    libc::kill(parent, libc::SIGKILL);
                }
            }
            for _ in 0..20 {
                if unsafe { libc::getppid() } != parent {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            if unsafe { libc::getppid() } == parent {
                return true;
            }
        }
        if let Ok(executable) = std::env::current_exe() {
            let _ = Command::new(executable).args(args.iter().skip(5)).spawn();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_failed_previous_launch_falls_back_before_starting_sdl() {
        let path = std::env::temp_dir().join(format!(
            "lumaglass-display-stale-{}.json",
            std::process::id()
        ));
        write(
            &pending(&path),
            &Request {
                token: "stale".into(),
                staged: false,
                trial: true,
            },
        )
        .unwrap();
        let guard = Guard::begin(&path, true, true).unwrap();
        assert!(!guard.four_k);
        assert!(fallback(&path).exists());
        std::fs::remove_file(fallback(&path)).unwrap();
    }
    #[test]
    fn confirmation_and_expiry_compete_for_one_marker_atomically() {
        let path = std::env::temp_dir().join(format!(
            "lumaglass-display-claim-{}.json",
            std::process::id()
        ));
        write(
            &pending(&path),
            &Request {
                token: "claim".into(),
                staged: false,
                trial: true,
            },
        )
        .unwrap();
        let mut guard = Guard {
            path: path.clone(),
            token: "claim".into(),
            trial: true,
            four_k: true,
            armed: true,
        };
        assert!(guard.confirm());
        assert!(!pending(&path).exists());
        assert!(!guard.armed);
        write(
            &pending(&path),
            &Request {
                token: "claim".into(),
                staged: false,
                trial: true,
            },
        )
        .unwrap();
        let expired = path.with_extension("expired");
        std::fs::rename(pending(&path), &expired).unwrap();
        guard.armed = true;
        assert!(!guard.confirm());
        std::fs::remove_file(expired).unwrap();
    }
}
