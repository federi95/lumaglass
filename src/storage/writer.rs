//! One ordered preference writer with a single replaceable pending snapshot.
use crate::model::Preferences;
use std::{
    cell::Cell,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, mpsc},
    thread::{self, JoinHandle},
};

struct Snapshot {
    generation: u64,
    preferences: Preferences,
}

#[derive(Default)]
struct Pending {
    snapshot: Option<Snapshot>,
    closed: bool,
}

pub struct Saved {
    pub background: String,
    pub result: Result<(), String>,
    generation: u64,
}

pub struct Writer {
    queue: Arc<(Mutex<Pending>, Condvar)>,
    generation: Cell<u64>,
    receiver: mpsc::Receiver<Saved>,
    worker: Option<JoinHandle<Result<Option<String>, String>>>,
}

impl Writer {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let queue = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let work = queue.clone();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("preferences".into())
            .spawn(move || {
                let mut last = Ok(None);
                loop {
                    let snapshot = {
                        let (lock, ready) = &*work;
                        let mut pending = lock.lock().unwrap();
                        while pending.snapshot.is_none() && !pending.closed {
                            pending = ready.wait(pending).unwrap();
                        }
                        match pending.snapshot.take() {
                            Some(snapshot) => snapshot,
                            None => return last,
                        }
                    };
                    let result = super::save(&path, &snapshot.preferences);
                    last = result
                        .clone()
                        .map(|()| Some(snapshot.preferences.background.clone()));
                    let _ = sender.send(Saved {
                        generation: snapshot.generation,
                        background: snapshot.preferences.background,
                        result,
                    });
                }
            })
            .map_err(|e| format!("Could not start preference writer: {e}"))?;
        Ok(Self {
            queue,
            generation: Cell::new(0),
            receiver,
            worker: Some(worker),
        })
    }

    pub fn save(&self, preferences: &Preferences) {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let (lock, ready) = &*self.queue;
        lock.lock().unwrap().snapshot = Some(Snapshot {
            generation,
            preferences: preferences.clone(),
        });
        ready.notify_one();
    }

    /// Only the newest requested snapshot may update the UI or the launch splash.
    pub fn poll(&self) -> Option<Saved> {
        self.receiver
            .try_iter()
            .find(|saved| saved.generation == self.generation.get())
    }

    pub fn finish(&mut self) -> Result<Option<String>, String> {
        let (lock, ready) = &*self.queue;
        lock.lock().unwrap().closed = true;
        ready.notify_one();
        match self.worker.take() {
            Some(worker) => worker
                .join()
                .map_err(|_| "Preference writer stopped unexpectedly".to_owned())?,
            None => Ok(None),
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("Lumaglass preferences: {error}");
        }
    }
}
