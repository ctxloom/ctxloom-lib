//! Test support: a fake program a test can run by path.
//!
//! A test that needs a program (an editor, a `git`, a container runtime) on a
//! path writes a script and runs it. Run as written, that is racy in a
//! multithreaded test process: a thread that forks while the script is open
//! for writing hands its child a copy of the descriptor, held until that
//! child execs; until then exec'ing the script fails with "text file busy"
//! (ETXTBSY), though the script is closed. Guarding against it (a retry, a
//! sleep) hides the race; this crate removes it.
//!
//! [`install_program`] makes the program a symlink to a shim that is committed
//! in this crate ([`SHIM`], `interpret.sh`) with the script beside it as
//! `<program>.body`. Running the program execs the shim and the shim runs the
//! body with `sh`, which only reads it. Neither file is ever exec'd after being
//! written by the process: the shim is not written by any test at all (it is
//! read from the crate's own directory, which is a stable file the build put
//! there before any test started), and the body is read, not exec'd, and a
//! reader never conflicts with a writer. That is why it is race-free where
//! "install the shim once per process" is not: any write in a process whose
//! test threads fork is exposed until its writers' copies are gone.
//!
//! Unix only (it needs `sh` and links): [`install_program`] fails with
//! `Unsupported` where the filesystem's flavor is not unix, and a test skips
//! there as it already must.

use std::io;
use std::path::{Path, PathBuf};

use file_system::{Fs, Os, Perms};
use os::{Flavor, Mode};

/// The committed shim every program links to.
pub const SHIM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/interpret.sh");

/// Install a program at `path` of `fs` that runs `body`, a script for `sh`
/// (its `#!` line, if any, is a comment): the script written beside it as
/// `<path>.body`, the program a link to [`SHIM`]. A program already at `path`
/// is replaced. Returns the program's path on the host, to run (or to put in
/// an environment variable or on a `PATH`).
///
/// In the body `$0` is `<program>.body`, so `dirname "$0"` is the program's
/// directory. The parent directory must exist (`NotFound`).
pub fn install_program(fs: &dyn Fs, path: &Path, body: &str) -> io::Result<PathBuf> {
    if fs.os().flavor() != Flavor::Unix {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "a fixture program needs sh and links: unix only",
        ));
    }
    let host = host_fs()?;
    shim_ready(&host, Path::new(SHIM))?;
    fs.write_atomic(&beside(path), body.as_bytes(), Perms::Exact(Mode::FILE))?;
    let real = fs.real_path(path)?;
    link(&host, Path::new(SHIM), &real)?;
    Ok(real)
}

/// `<program>.body`.
fn beside(program: &Path) -> PathBuf {
    let mut body = program.as_os_str().to_owned();
    body.push(".body");
    PathBuf::from(body)
}

/// The host's own filesystem: a link's target is a real path, which a
/// re-rooted filesystem would re-root.
fn host_fs() -> io::Result<Os> {
    let host = os::host();
    let top = host.flavor().top();
    Os::new(host, Path::new(top))
}

/// The shim is there and executable; its mode is the one thing a checkout
/// that does not keep modes loses.
fn shim_ready(host: &dyn Fs, shim: &Path) -> io::Result<()> {
    if host.metadata(shim)?.executable {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("{} lost its execute bit", shim.display()),
    ))
}

/// A link at `at` to `shim`, replacing what is there.
fn link(host: &dyn Fs, shim: &Path, at: &Path) -> io::Result<()> {
    if host.symlink_metadata(at).is_ok() {
        host.remove_file(at)?;
    }
    host.symlink_dir(shim, at)
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
