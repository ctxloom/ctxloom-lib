//! Run a program under a deadline, and get its output however it ends.
//!
//! A program run with its output read through pipes can hang its caller in
//! two ways, and this crate closes both:
//!
//! - A program that outlives its deadline is killed with everything it
//!   started ([`os::process::Tree::kill_tree`]), not only the program: a
//!   shell's `sleep`, a git hook, a helper daemon would otherwise live on
//!   holding the pipes.
//! - A pipe reaches its end only when every process holding its write end
//!   has closed it, so a descendant that outlives a program that exited
//!   normally holds a reader of the pipe forever. The wait for the readers
//!   is therefore bounded: after the program has exited or been killed, they
//!   get [`Within::drain_grace`] more, and are then abandoned (they end when
//!   the descendant closes the pipe) with [`Outcome::drain_incomplete`] set
//!   and what they had read so far. This is Go's `Cmd.WaitDelay`.
//!
//! Stdout and stderr are kept apart, each up to [`Within::max_bytes`]
//! ([`Outcome::truncated`] says some was discarded); there are no files.
//!
//! The environment is exactly the [`Spec`]'s: the caller says every
//! variable, so nothing of the caller's own leaks in.
//!
//! Limits, from [`os::process`]: a descendant that leaves the process
//! group (`setsid`, `setpgid`) or the job object escapes the kill; and the
//! program runs in a process group of its own, so a Ctrl-C at the terminal
//! does not reach it. A caller that wants it to stop on a signal hears the
//! signal itself ([`os::Os::signals`]) and requests the [`Stop`] it ran the
//! program with ([`run_within_stopped_by`]); this crate returns only when
//! its program is over.
//!
//! # Stopped
//!
//! [`run_within_stopped_by`] is [`run_within_started`] that also ends when a
//! [`Stop`] is requested (a caller that heard a signal asking it to stop):
//! the tree is asked to stop (SIGTERM to the group on unix; on Windows,
//! which has no ask, the kill is at once), the program gets
//! [`Within::stop_grace`] to exit, then whatever is left of the tree is
//! killed, and the outcome is [`stopped`](Outcome::stopped) with the
//! program's status.
//!
//! # Started
//!
//! [`run_within_started`] is [`run_within`] that calls a closure with the
//! program's pid once the spawn has succeeded (never when it failed), so a
//! caller can record "started" truthfully.
//!
//! # Terminal mode
//!
//! [`run_terminal`] runs an interactive program. Its invariants:
//!
//! - The program inherits the caller's stdin, stdout and stderr; nothing is
//!   captured, so [`Within::max_bytes`], [`Within::drain_grace`] and the
//!   [`Spec`]'s input do not apply (the input is ignored).
//! - The program is NOT put in a process group of its own: it stays in the
//!   caller's, so when that is the terminal's foreground group its reads
//!   from the terminal do not stop it (SIGTTIN), and a Ctrl-C at the
//!   terminal reaches it and its descendants directly.
//! - There is no deadline. The wait ends when the program exits, or when
//!   the caller's [`Stop`] has been requested, which kills the DIRECT child
//!   (SIGKILL on unix, `TerminateProcess` on Windows) and reports it
//!   [`stopped`](TerminalOutcome::stopped). The caller turns the platform's
//!   signals ([`os::Os::signals`]) into [`Stop::request`] on its own
//!   runtime.
//! - The tree is NOT killed: with no group of its own there is no handle on
//!   the descendants, and signalling the caller's own group would signal the
//!   caller. A descendant outlives a stopped program unless the terminal
//!   delivers it a signal too. A caller that must end the descendants runs
//!   the program in a mode with a tree ([`run_within`]).
//!
//! Tested on Linux only; macOS and Windows use only `std::process`, but are
//! not exercised. No test attaches a real tty: the inheritance and
//! group invariants are proved, not the terminal's own job control.
//!
//! # Piped mode
//!
//! [`run_piped`] starts a LONG-LIVED program the caller converses with (a
//! line protocol over stdin and stdout, say). Its invariants:
//!
//! - Its stdin, stdout and stderr are pipes, handed to the caller as
//!   [`Pipes`]; nothing is captured or capped, so [`Within`] does not apply
//!   and the [`Spec`]'s input is ignored. Dropping the stdin is the
//!   program's end of input. The caller reads both outputs, or drops the
//!   one it does not want: a full pipe no one reads blocks the program.
//! - There is no deadline. The program runs until it exits, or until the
//!   caller ends its tree through the [`Piped`] handle:
//!   [`kill_tree`](Piped::kill_tree) (the deadline kill's, the whole tree at
//!   once) or [`terminate_tree`](Piped::terminate_tree) (the stop's: asked,
//!   a grace, then killed).
//! - It is a tree of its own, as in [`run_within`]: the same limits (a
//!   descendant that leaves the group escapes; a Ctrl-C at the terminal does
//!   not reach it), and a kill after the program exited still ends what it
//!   left running. Dropping the handle ends nothing.
//! - The environment is exactly the [`Spec`]'s, and `started` is called once
//!   with the pid after the spawn, never when it failed, as in
//!   [`run_within_started`].
//! - The handle is `Clone + Send + Sync`: one thread waits
//!   ([`Piped::wait`], which asks between pauses and never blocks in the
//!   system's wait) while another ends the tree.
//!
//! It is synchronous, std pipes and the caller's threads, like the rest of
//! this crate: no async runtime is linked, so a consumer on any runtime (or
//! none) can use it. The pipes are std's `ChildStdin`, `ChildStdout` and
//! `ChildStderr`; a consumer that wants them async converts them on its own
//! runtime (on unix each is an `OwnedFd`, which tokio's
//! `net::unix::pipe::{Sender, Receiver}` take; anywhere, a blocking thread
//! per pipe bridged to an async channel), and a blocking [`Piped::wait`] goes
//! on a blocking thread (`spawn_blocking`) of its own.
//!
//! A program that exited normally and left a descendant running is not
//! killed: the descendant may be meant to stay.
//!
//! The `test-support` feature adds only test helpers
//! ([`install_script`], [`process_gone`], `diagnostic_samples`); it changes nothing a
//! production build does. In particular a spawn that fails with ETXTBSY
//! ("text file busy") fails: the cause is a test fixture exec'ing a script
//! it has just written (see [`install_script`]), and the fixture is where it
//! is fixed.

mod capture;
mod error;
mod piped;
#[cfg(any(test, feature = "test-support"))]
mod procfs;
#[cfg(any(test, feature = "test-support"))]
mod script;
mod terminal;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use os::process::{spawn_tree, Tree};

#[cfg(any(test, feature = "test-support"))]
pub use error::diagnostic_samples;
pub use error::{RunError, EN_US};
pub use piped::{run_piped, Piped, Pipes};
#[cfg(any(test, feature = "test-support"))]
pub use procfs::process_gone;
#[cfg(any(test, feature = "test-support"))]
pub use script::install_script;
pub use terminal::{run_terminal, Stop, TerminalOutcome};

/// How often the program is asked whether it has exited, when nothing else
/// wakes the wait.
const POLL: Duration = Duration::from_millis(5);

/// What to run: the program, its arguments, its whole environment, where, and
/// what it is fed. Its `Debug` names the variables and gives the input's
/// length only, never a variable's value or the input itself (ruling 84): an
/// environment holds credentials, and a `{:?}` in a log or an error must not
/// carry them.
#[derive(Clone)]
pub struct Spec {
    program: OsString,
    args: Vec<OsString>,
    env: Vec<(OsString, OsString)>,
    cwd: Option<PathBuf>,
    stdin: Option<Vec<u8>>,
}

impl fmt::Debug for Spec {
    /// The program, its arguments and directory as they are; the variables'
    /// names (lossily when not UTF-8); the input as its length.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<_> = self
            .env
            .iter()
            .map(|(name, _)| name.to_string_lossy())
            .collect();
        f.debug_struct("Spec")
            .field("program", &self.program)
            .field("args", &self.args)
            .field("env", &names)
            .field("cwd", &self.cwd)
            .field(
                "stdin",
                &self.stdin.as_ref().map(|bytes| Length(bytes.len())),
            )
            .finish()
    }
}

/// An input's length, rendered `<N bytes>` in a [`Spec`]'s `Debug`.
struct Length(usize);

impl fmt::Debug for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} bytes>", self.0)
    }
}

impl Spec {
    /// `program` (a path, or a name looked up on the `PATH` of the
    /// environment of the caller, not of the program), with no arguments, an
    /// EMPTY environment, the caller's working directory and no input.
    pub fn new(program: impl Into<OsString>) -> Spec {
        Spec {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
            stdin: None,
        }
    }

    /// Add arguments.
    pub fn args<I, S>(mut self, args: I) -> Spec
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|a| a.as_ref().to_os_string()));
        self
    }

    /// Add environment variables. The program sees these and no others; a
    /// later value for a name replaces an earlier one.
    pub fn envs<I, K, V>(mut self, vars: I) -> Spec
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (key, value) in vars {
            let key = key.as_ref().to_os_string();
            self.env.retain(|(k, _)| *k != key);
            self.env.push((key, value.as_ref().to_os_string()));
        }
        self
    }

    /// The directory to run in.
    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Spec {
        self.cwd = Some(dir.into());
        self
    }

    /// Bytes to feed the program's stdin, which is then closed. Without
    /// them its stdin is closed from the start (the null device).
    pub fn stdin(mut self, bytes: impl Into<Vec<u8>>) -> Spec {
        self.stdin = Some(bytes.into());
        self
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .env_clear()
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = &self.cwd {
            command.current_dir(dir);
        }
        command
    }

    fn name(&self) -> String {
        self.program.to_string_lossy().into_owned()
    }
}

/// How long a run has, and how much it may say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Within {
    /// How long the program has to exit before its tree is killed.
    pub deadline: Duration,
    /// How long, once the program has exited or been killed, the readers of
    /// its output get to reach the end of the pipes before they are
    /// abandoned.
    pub drain_grace: Duration,
    /// The most of each stream that is kept; the rest is discarded.
    pub max_bytes: usize,
    /// How long, once a stop is requested ([`run_within_stopped_by`]), the
    /// program has to exit after the ask before its tree is killed.
    pub stop_grace: Duration,
}

impl Within {
    /// The default grace after the program is over.
    pub const DRAIN_GRACE: Duration = Duration::from_secs(2);
    /// The default cap on each stream: 64 MiB.
    pub const MAX_BYTES: usize = 64 * 1024 * 1024;
    /// The default grace after a stop is asked.
    pub const STOP_GRACE: Duration = Duration::from_secs(5);

    /// `deadline` for the program, [`DRAIN_GRACE`](Self::DRAIN_GRACE) for
    /// its output after, [`MAX_BYTES`](Self::MAX_BYTES) of each stream,
    /// [`STOP_GRACE`](Self::STOP_GRACE) after a stop is asked.
    pub fn new(deadline: Duration) -> Within {
        Within {
            deadline,
            drain_grace: Self::DRAIN_GRACE,
            max_bytes: Self::MAX_BYTES,
            stop_grace: Self::STOP_GRACE,
        }
    }

    /// The same, giving the program `grace` to exit once a stop is asked.
    pub fn stop_grace(mut self, grace: Duration) -> Within {
        self.stop_grace = grace;
        self
    }

    /// The same, with `grace` to read the output after the program is over.
    pub fn drain_grace(mut self, grace: Duration) -> Within {
        self.drain_grace = grace;
        self
    }

    /// The same, keeping at most `max` bytes of each stream.
    pub fn max_bytes(mut self, max: usize) -> Within {
        self.max_bytes = max;
        self
    }
}

/// How a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// How the program exited; `None` when it was killed at its deadline.
    pub status: Option<ExitStatus>,
    /// What it wrote to stdout, up to the cap.
    pub stdout: Vec<u8>,
    /// What it wrote to stderr, up to the cap.
    pub stderr: Vec<u8>,
    /// The deadline passed and the tree was killed.
    pub timed_out: bool,
    /// A stop was requested while the program ran, and the tree was asked
    /// to stop, then killed ([`run_within_stopped_by`]). A program that
    /// exited by itself is not `stopped`.
    pub stopped: bool,
    /// Some output beyond the cap was discarded.
    pub truncated: bool,
    /// A pipe was still open when the drain grace ended: a process that
    /// outlived the program still holds it, and any more it writes is lost.
    pub drain_incomplete: bool,
}

/// Run `spec` and return how it ended: its exit status or the kill at the
/// deadline, and its output. A program that exits non-zero is an outcome,
/// not an error.
pub fn run_within(spec: &Spec, within: &Within) -> Result<Outcome, RunError> {
    run_within_started(spec, within, |_| {})
}

/// [`run_within`], telling the caller the program's process id once it has
/// really been started: `started` is called exactly once, with the pid, after
/// the spawn succeeded and before anything else is done with the program. A
/// program that cannot be started is the error, and `started` is not called.
pub fn run_within_started(
    spec: &Spec,
    within: &Within,
    started: impl FnOnce(u32),
) -> Result<Outcome, RunError> {
    run_within_stopped_by(spec, within, &Stop::new(), started)
}

/// [`run_within_started`], also ending when `stop` is requested: the tree is
/// asked to stop, given [`Within::stop_grace`], then killed, and the outcome
/// is [`stopped`](Outcome::stopped). A `stop` requested before the call still
/// lets the program start (and `started` be called); it is asked to stop at
/// the first look.
pub fn run_within_stopped_by(
    spec: &Spec,
    within: &Within,
    stop: &Stop,
    started: impl FnOnce(u32),
) -> Result<Outcome, RunError> {
    let mut tree = spawn_tree(spec.command()).map_err(|e| RunError::spawn(spec.name(), e))?;
    started(tree.id());
    let (done_tx, done_rx) = mpsc::channel();
    let stdout = capture::start(tree.take_stdout(), within.max_bytes, done_tx.clone());
    let stderr = capture::start(tree.take_stderr(), within.max_bytes, done_tx);
    feed(&mut tree, spec);
    let mut ended = Ended::default();
    let until = Until {
        deadline: within.deadline,
        stop,
        stop_grace: within.stop_grace,
    };
    let over = wait(&mut tree, spec, &until, &done_rx, &mut ended)?;
    let drain_incomplete = !drain(&done_rx, &mut ended, within.drain_grace);
    let (stdout, cut_out) = stdout.take();
    let (stderr, cut_err) = stderr.take();
    let (status, timed_out, stopped) = match over {
        Over::Exited(status) => (Some(status), false, false),
        Over::TimedOut => (None, true, false),
        Over::Stopped(status) => (status, false, true),
    };
    Ok(Outcome {
        timed_out,
        stopped,
        status,
        stdout,
        stderr,
        truncated: cut_out || cut_err,
        drain_incomplete,
    })
}

/// How many of the two readers have reached the end of their pipe.
#[derive(Default)]
struct Ended(usize);

impl Ended {
    const BOTH: usize = 2;

    fn both(&self) -> bool {
        self.0 >= Self::BOTH
    }
}

/// Feed the input on a thread of its own, so a program that never reads it
/// cannot hold the run past its deadline; the thread ends when the program
/// does, with a broken pipe.
fn feed(tree: &mut Tree, spec: &Spec) {
    if let (Some(bytes), Some(mut pipe)) = (spec.stdin.clone(), tree.take_stdin()) {
        std::thread::spawn(move || {
            let _ = pipe.write_all(&bytes);
        });
    }
}

/// What [`wait`] asks of a started program: [`Tree`], or a stand-in in a test
/// that must make the system's calls fail.
trait Running {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>>;
    fn kill_tree(&mut self) -> std::io::Result<()>;
    fn terminate_tree(&mut self, grace: Duration) -> std::io::Result<()>;
}

impl Running for Tree {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        Tree::try_wait(self)
    }

    fn kill_tree(&mut self) -> std::io::Result<()> {
        Tree::kill_tree(self)
    }

    fn terminate_tree(&mut self, grace: Duration) -> std::io::Result<()> {
        Tree::terminate_tree(self, grace)
    }
}

/// When a [`wait`] gives up on the program: at its deadline, or at a stop.
struct Until<'a> {
    deadline: Duration,
    stop: &'a Stop,
    stop_grace: Duration,
}

/// How a [`wait`] ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Over {
    /// The program exited by itself.
    Exited(ExitStatus),
    /// The deadline passed and the tree was killed.
    TimedOut,
    /// A stop was requested: the tree was asked to stop, then killed; the
    /// program's status, when the system still gives it.
    Stopped(Option<ExitStatus>),
}

/// Wait for the program to exit, its deadline to pass (its tree killed) or a
/// stop to be requested (its tree asked to stop, then killed). Nothing here
/// blocks in a wait that a kill would have to get past: the program is asked,
/// and the pause between asks ends early when a reader finishes. The exit is
/// asked first, so a program that is over is never reported stopped.
fn wait(
    tree: &mut impl Running,
    spec: &Spec,
    until: &Until<'_>,
    done: &Receiver<()>,
    ended: &mut Ended,
) -> Result<Over, RunError> {
    let deadline = Instant::now() + until.deadline;
    let killed = |source| RunError::Kill {
        program: spec.name(),
        source,
    };
    loop {
        match tree.try_wait() {
            Ok(Some(status)) => return Ok(Over::Exited(status)),
            Ok(None) => {}
            Err(source) => {
                let _ = tree.kill_tree();
                return Err(RunError::Wait {
                    program: spec.name(),
                    source,
                });
            }
        }
        if until.stop.is_requested() {
            tree.terminate_tree(until.stop_grace).map_err(killed)?;
            return Ok(Over::Stopped(tree.try_wait().ok().flatten()));
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return tree.kill_tree().map(|()| Over::TimedOut).map_err(killed);
        }
        if done.recv_timeout(left.min(POLL)).is_ok() {
            ended.0 += 1;
        }
    }
}

/// Wait up to `grace` for the readers to reach the end of their pipes;
/// whether they both did.
fn drain(done: &Receiver<()>, ended: &mut Ended, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    while !ended.both() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || done.recv_timeout(left).is_err() {
            return false;
        }
        ended.0 += 1;
    }
    true
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
