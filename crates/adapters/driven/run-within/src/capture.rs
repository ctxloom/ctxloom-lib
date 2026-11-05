//! A pipe read as it comes, kept up to a cap, by a thread that can be
//! abandoned: what it has read so far stays readable however long the pipe
//! stays open.

use std::io::Read;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};

/// How much is read from the pipe at a time.
const CHUNK: usize = 8192;

#[derive(Default)]
struct Held {
    bytes: Vec<u8>,
    truncated: bool,
}

/// What a reader thread has kept of one pipe.
pub(crate) struct Capture {
    held: Arc<Mutex<Held>>,
}

/// Read `pipe` (none: nothing to read) on a thread of its own, keeping at
/// most `max` bytes and discarding the rest, so the writer is never stuck on
/// a full pipe. The thread sends on `done` when the pipe reaches its end,
/// which is when no process holds its write end any more.
pub(crate) fn start<R: Read + Send + 'static>(
    pipe: Option<R>,
    max: usize,
    done: Sender<()>,
) -> Capture {
    let held = Arc::new(Mutex::new(Held::default()));
    let keep = Arc::clone(&held);
    std::thread::spawn(move || {
        if let Some(mut pipe) = pipe {
            read_all(&mut pipe, max, &keep);
        }
        let _ = done.send(());
    });
    Capture { held }
}

fn read_all<R: Read>(pipe: &mut R, max: usize, held: &Mutex<Held>) {
    let mut chunk = [0u8; CHUNK];
    loop {
        let read = match pipe.read(&mut chunk) {
            Ok(0) => return,
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        };
        let mut held = held.lock().unwrap_or_else(PoisonError::into_inner);
        let room = max - held.bytes.len();
        held.bytes.extend_from_slice(&chunk[..read.min(room)]);
        held.truncated |= read > room;
    }
}

impl Capture {
    /// What has been kept so far, and whether more was discarded. The reader
    /// may still be running, if it was abandoned.
    pub(crate) fn take(&self) -> (Vec<u8>, bool) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        (std::mem::take(&mut held.bytes), held.truncated)
    }
}

#[cfg(test)]
#[path = "capture.test.rs"]
mod tests;
