use std::io::{self, Cursor, Read};
use std::sync::mpsc;
use std::time::Duration;

use super::start;

const SOON: Duration = Duration::from_secs(10);

fn kept(bytes: &[u8], max: usize) -> (Vec<u8>, bool) {
    let (tx, rx) = mpsc::channel();
    let capture = start(Some(Cursor::new(bytes.to_vec())), max, tx);
    rx.recv_timeout(SOON).expect("the reader ends");
    capture.take()
}

#[test]
fn everything_up_to_the_cap_is_kept_and_not_called_cut() {
    assert_eq!(kept(b"abcde", 5), (b"abcde".to_vec(), false));
    assert_eq!(kept(b"", 0), (Vec::new(), false));
    assert_eq!(kept(b"abc", 100), (b"abc".to_vec(), false));
}

#[test]
fn one_byte_over_the_cap_is_discarded_and_said() {
    assert_eq!(kept(b"abcdef", 5), (b"abcde".to_vec(), true));
    assert_eq!(kept(b"a", 0), (Vec::new(), true));
}

#[test]
fn the_cap_holds_across_many_chunks() {
    let big = vec![b'x'; 3 * super::CHUNK + 17];
    let (bytes, cut) = kept(&big, super::CHUNK + 5);
    assert_eq!(bytes.len(), super::CHUNK + 5);
    assert!(cut);
    let (bytes, cut) = kept(&big, big.len());
    assert_eq!(bytes, big);
    assert!(!cut);
}

#[test]
fn no_pipe_is_an_empty_stream_that_still_reports_done() {
    let (tx, rx) = mpsc::channel();
    let capture = start(None::<Cursor<Vec<u8>>>, 10, tx);
    rx.recv_timeout(SOON).expect("done");
    assert_eq!(capture.take(), (Vec::new(), false));
}

/// A pipe that is interrupted once, then speaks, then fails.
struct Flaky(u8);

impl Read for Flaky {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0 += 1;
        match self.0 {
            1 => Err(io::ErrorKind::Interrupted.into()),
            2 => {
                buf[..2].copy_from_slice(b"hi");
                Ok(2)
            }
            _ => Err(io::Error::other("broken")),
        }
    }
}

#[test]
fn an_interrupted_read_is_retried_and_another_error_ends_the_stream() {
    let (tx, rx) = mpsc::channel();
    let capture = start(Some(Flaky(0)), 10, tx);
    rx.recv_timeout(SOON).expect("done");
    assert_eq!(capture.take(), (b"hi".to_vec(), false));
}

/// A pipe that gives its first bytes, then waits to be released: an
/// abandoned reader, whose first bytes are still readable.
struct Held(mpsc::Receiver<()>, bool);

impl Read for Held {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.1 {
            let _ = self.0.recv();
            return Ok(0);
        }
        self.1 = true;
        buf[..3].copy_from_slice(b"one");
        Ok(3)
    }
}

#[test]
fn what_an_unfinished_reader_has_read_is_readable() {
    let (release, held) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let capture = start(Some(Held(held, false)), 10, tx);
    // Wait for the first bytes by the reader's own progress: poll take.
    let mut got = Vec::new();
    while got.is_empty() {
        got = capture.take().0;
        std::thread::yield_now();
    }
    assert_eq!(got, b"one");
    assert!(rx.try_recv().is_err(), "the pipe has not ended");
    release.send(()).unwrap();
    rx.recv_timeout(SOON).expect("done after release");
}
