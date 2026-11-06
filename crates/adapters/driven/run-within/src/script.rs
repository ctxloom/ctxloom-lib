//! Test support: a program to run.

use std::io;
use std::path::{Path, PathBuf};

use file_system::{Fs, Perms};
use os::Mode;

/// Write `body` (the whole script, its `#!` line included) as the
/// executable `name` in `dir` of `fs`: written, synced, closed and made
/// executable, atomically. Returns the path on the host.
///
/// Run it as `sh <path>` (the interpreter named in the `#!` line, the path as
/// its argument), never as the program itself. Exec'ing a file just written
/// is racy in a multithreaded test process: a thread that forks while this
/// file is open for writing gives the child a copy of the descriptor, held
/// until that child execs, and until then an exec of this file fails with
/// "text file busy" (ETXTBSY) even though this file is closed. An interpreter
/// only reads the script, which a writer does not prevent.
///
/// A test that needs a program on a path to run by itself (an editor, a
/// `git`) wants `fixture-program`'s `install_program` instead.
pub fn install_script(fs: &dyn Fs, dir: &Path, name: &str, body: &str) -> io::Result<PathBuf> {
    let path = dir.join(name);
    fs.write_atomic(&path, body.as_bytes(), Perms::Exact(Mode::from_bits(0o755)))?;
    fs.real_path(&path)
}

#[cfg(test)]
#[path = "script.test.rs"]
mod tests;
