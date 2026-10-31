//! Bounded child helpers for an already-root native Home. No extra idle worker.
use std::{
    io::Read,
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Job {
    child: Child,
    running: bool,
}
impl Drop for Job {
    fn drop(&mut self) {
        if self.running {
            // SAFETY: process_group(0) gave this child its own group. A live,
            // unreaped child reserves its PID; only this job's group is killed.
            unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) };
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
fn run(program: &str, args: &[&str], timeout: Duration, limit: usize) -> Result<String, String> {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|_| "Could not start the TV helper")?;
    let mut job = Job {
        child,
        running: true,
    };
    let mut stdout = job.child.stdout.take().ok_or("Missing helper output")?;
    let fd = stdout.as_raw_fd();
    // SAFETY: stdout owns a live pipe descriptor. Existing flags are preserved.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err("Could not bound TV helper output".into());
    }
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut status = None;
    loop {
        let mut chunk = [0; 4096];
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) => break,
                Ok(size) => {
                    if bytes.len() + size > limit {
                        return Err("TV helper output exceeds its limit".into());
                    }
                    bytes.extend_from_slice(&chunk[..size]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err("Could not read TV helper output".into()),
            }
            if Instant::now() >= deadline {
                return Err("TV helper timed out".into());
            }
        }
        if let Some(status) = status {
            return if status {
                String::from_utf8(bytes).map_err(|_| "Invalid TV helper encoding".into())
            } else {
                Err("TV helper failed".into())
            };
        }
        if let Some(exited) = job.child.try_wait().map_err(|_| "TV helper wait failed")? {
            job.running = false;
            status = Some(exited.success());
            // Drain any bytes written between the read and process exit.
            continue;
        }
        if Instant::now() >= deadline {
            return Err("TV helper timed out".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
}
pub(super) fn node(script: &str, payload: Option<&str>) -> Result<String, String> {
    let mut args = vec!["-e", script];
    if let Some(payload) = payload {
        args.push(payload);
    }
    run(
        "/usr/bin/node",
        &args,
        Duration::from_secs(7),
        2 * 1024 * 1024,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_helper_preserves_complete_output_and_rejects_failure() {
        assert_eq!(
            run(
                "sh",
                &["-c", "printf '{\"schema\":1}\\n'"],
                Duration::from_secs(2),
                128
            )
            .unwrap(),
            "{\"schema\":1}\n"
        );
        assert!(run("sh", &["-c", "exit 3"], Duration::from_secs(2), 128).is_err());
    }
    #[test]
    fn bounded_helper_stops_timeout_and_excess_output() {
        assert!(
            run("sh", &["-c", "sleep 10"], Duration::from_millis(30), 128)
                .unwrap_err()
                .contains("timed out")
        );
        assert!(
            run(
                "sh",
                &["-c", "printf '123456789'"],
                Duration::from_secs(2),
                4
            )
            .unwrap_err()
            .contains("limit")
        );
    }
}
