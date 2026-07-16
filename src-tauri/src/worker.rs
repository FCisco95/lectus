//! A single dedicated OS thread for whisper inference.
//!
//! whisper.cpp / ggml (built with OpenMP) can deadlock when `whisper_full` is
//! called from different OS threads across successive calls — which is exactly
//! what `tokio::task::spawn_blocking` does, since its blocking pool hands out a
//! different worker thread each time. The first dictation succeeds; the second,
//! landing on another thread, hits an OpenMP barrier deadlock (0% CPU, frozen
//! forever). Pinning every transcription to ONE thread removes that hazard and
//! also naturally serializes inference (a single user can't dictate twice at
//! once anyway).

use std::sync::mpsc::{channel, Sender};

type Job = Box<dyn FnOnce() + Send + 'static>;

/// Handle to the dedicated whisper worker thread. Cheap to clone (clones the
/// channel sender); all clones target the same single thread.
#[derive(Clone)]
pub struct TranscribeWorker {
    tx: Sender<Job>,
}

impl TranscribeWorker {
    pub fn new() -> Self {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name("whisper-worker".into())
            .spawn(move || {
                // Process queued jobs sequentially on this one thread for the
                // lifetime of the app.
                while let Ok(job) = rx.recv() {
                    job();
                }
            })
            .expect("failed to spawn whisper worker thread");
        Self { tx }
    }

    /// Run `f` on the dedicated whisper thread, blocking the caller until it
    /// returns. Call from a blocking context (e.g. inside `spawn_blocking`),
    /// never directly on an async task — it parks the caller on a channel recv.
    pub fn run<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (rtx, rrx) = channel::<R>();
        if self
            .tx
            .send(Box::new(move || {
                let _ = rtx.send(f());
            }))
            .is_err()
        {
            panic!("whisper worker thread is not running");
        }
        rrx.recv()
            .expect("whisper worker dropped the job without replying")
    }
}

impl Default for TranscribeWorker {
    fn default() -> Self {
        Self::new()
    }
}
