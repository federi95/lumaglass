//! Re-resolve the OS offset in a bounded, single-threaded helper process.
//! `time` deliberately rejects local-offset lookups in a multithreaded process
//! on some Unix systems. Keep that safeguard; never enable unsound offset mode.
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use time::UtcOffset;

pub const HELPER: &str = "--local-offset";

pub struct Clock {
    pub offset: UtcOffset,
    next: Instant,
    pending: bool,
    sender: Option<mpsc::SyncSender<()>>,
    receiver: mpsc::Receiver<Result<UtcOffset, String>>,
    worker: Option<JoinHandle<()>>,
}

impl Clock {
    pub fn new(offset: UtcOffset) -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let (sender, jobs) = mpsc::sync_channel(1);
        let (responses, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("local-clock".into())
            .spawn(move || {
                for () in jobs {
                    let result = resolve(&executable);
                    if responses.send(result).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| format!("Could not start clock worker: {e}"))?;
        Ok(Self {
            offset,
            next: Instant::now() + Duration::from_secs(60),
            pending: false,
            sender: Some(sender),
            receiver,
            worker: Some(worker),
        })
    }

    pub fn refresh(&mut self) {
        if !self.pending
            && let Some(sender) = &self.sender
        {
            self.pending = sender.try_send(()).is_ok();
            self.next = Instant::now() + Duration::from_secs(60);
        }
    }

    pub fn tick(&mut self) -> bool {
        if Instant::now() >= self.next {
            self.refresh();
        }
        let mut changed = false;
        while let Ok(result) = self.receiver.try_recv() {
            self.pending = false;
            match result {
                Ok(offset) => {
                    changed |= self.offset != offset;
                    self.offset = offset;
                }
                Err(error) => eprintln!("Lumaglass local clock: {error}"),
            }
        }
        changed
    }
}

impl Drop for Clock {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn resolve(executable: &std::path::Path) -> Result<UtcOffset, String> {
    let mut child = Command::new(executable)
        .arg(HELPER)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err("The OS timezone could not be resolved".into());
                }
                let mut text = String::new();
                child
                    .stdout
                    .take()
                    .ok_or("Missing clock reply")?
                    .take(33)
                    .read_to_string(&mut text)
                    .map_err(|e| e.to_string())?;
                if text.len() > 32 {
                    return Err("Invalid clock reply size".into());
                }
                let seconds = text
                    .trim()
                    .parse()
                    .map_err(|_| "Invalid OS timezone offset")?;
                return UtcOffset::from_whole_seconds(seconds).map_err(|e| e.to_string());
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(match result {
                    Err(error) => error.to_string(),
                    _ => "OS timezone lookup timed out".into(),
                });
            }
        }
    }
}
