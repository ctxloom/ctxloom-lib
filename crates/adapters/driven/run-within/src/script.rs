//! Test support: a program to run.

use std::io;
use std::path::{Path, PathBuf};

use file_system::{Fs, Perms};
use os::Mode;

/// Write `body` (the whole script, its `#!` line included) as the
/// executable `name` in `dir` of `fs`: written, synced, closed and made
/// executable, atomically. Returns the path on the host, for a `Command` to
/// run.
///
/// A script that is run at once may still be "text file busy" (ETXTBSY): a
/// thread of the same process that forked while this file was open for
/// writing gives the fork's copy of the descriptor up only when that child
/// executes. [`run_within`](crate::run_within) waits that out when this
/// feature is on, so a test needs no retry of its own.
pub fn install_script(fs: &dyn Fs, dir: &Path, name: &str, body: &str) -> io::Result<PathBuf> {
    let path = dir.join(name);
    fs.write_atomic(&path, body.as_bytes(), Perms::Exact(Mode::from_bits(0o755)))?;
    fs.real_path(&path)
}

#[cfg(test)]
#[path = "script.test.rs"]
mod tests;
