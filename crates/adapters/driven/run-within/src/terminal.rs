//! Run an interactive program on the caller's terminal.

use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::{RunError, Spec, Within};

/// How often the program is asked whether it has exited, and the stop asked
/// after.
const POLL: Duration = Duration::from_millis(5);

/// A request, from anywhere, that a [`run_terminal`] or a
/// [`run_within_stopped_by`](crate::run_within_stopped_by) end its program. Clones
/// share the request: it is made once and stays made.
#[derive(Clone, Debug, Default)]
pub struct Stop(Arc<AtomicBool>);

impl Stop {
    /// A stop not yet requested.
    pub fn new() -> Stop {
        Stop::default()
    }

    /// Ask for the stop. Idempotent.
    pub fn request(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether the stop has been requested.
    pub fn is_requested(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// How a terminal run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalOutcome {
    /// How the program ended: its own exit, or the kill a stop made.
    pub status: ExitStatus,
    /// The program was killed because the [`Stop`] was requested while it
    /// still ran. A program that exited by itself is not `stopped`, even if
    /// the stop was requested at the same moment.
    pub stopped: bool,
}

/// How a terminal run is ended once its [`Stop`] is requested: the shape of
/// [`Within::stop_grace`], with nothing else of [`Within`], which a terminal
/// run has no use for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Terminal {
    /// How long, once a stop is requested, the program has to exit after the
    /// ask before it is killed.
    pub stop_grace: Duration,
}

impl Terminal {
    /// The default grace after a stop is asked: [`Within::STOP_GRACE`].
    pub const STOP_GRACE: Duration = Within::STOP_GRACE;

    /// [`STOP_GRACE`](Self::STOP_GRACE) after a stop is asked.
    pub fn new() -> Terminal {
        Terminal {
            stop_grace: Self::STOP_GRACE,
        }
    }

    /// The same, giving the program `grace` to exit once a stop is asked.
    pub fn stop_grace(mut self, grace: Duration) -> Terminal {
        self.stop_grace = grace;
        self
    }
}

impl Default for Terminal {
    fn default() -> Terminal {
        Terminal::new()
    }
}

/// [`run_terminal_with`] at [`Terminal::new`]: a stop gives the program
/// [`Terminal::STOP_GRACE`].
pub fn run_terminal(
    spec: &Spec,
    stop: &Stop,
    started: impl FnOnce(u32),
) -> Result<TerminalOutcome, RunError> {
    run_terminal_with(spec, &Terminal::new(), stop, started)
}

/// Run `spec` with the caller's stdin, stdout and stderr, in the caller's
/// process group, until it exits or `stop` is requested: then its DIRECT
/// child is asked to stop (SIGTERM on unix; Windows has no ask), given
/// [`Terminal::stop_grace`] to exit, killed if it has not, and reaped.
/// `started` is called once with the pid after the spawn succeeded, never
/// when it failed. The crate docs state what this guarantees and does not.
///
/// A `stop` requested before the call still lets the program start (and
/// `started` be called); it is then asked to stop at the first look.
pub fn run_terminal_with(
    spec: &Spec,
    terminal: &Terminal,
    stop: &Stop,
    started: impl FnOnce(u32),
) -> Result<TerminalOutcome, RunError> {
    let mut child = command(spec)
        .spawn()
        .map_err(|e| RunError::spawn(spec.name(), e))?;
    started(child.id());
    wait(&mut child, spec, stop, terminal.stop_grace)
}

fn command(spec: &Spec) -> Command {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .env_clear()
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(dir) = &spec.cwd {
        command.current_dir(dir);
    }
    command
}

/// What [`wait`] asks of the program: [`Child`], or a stand-in in a test that
/// must make the system's calls fail.
trait Running {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>>;
    /// Ask the program to stop; whether an ask was sent (`false` where the
    /// platform has none, or the ask failed: the kill follows at once).
    fn request_stop(&mut self) -> bool;
    fn kill(&mut self) -> std::io::Result<()>;
    fn wait(&mut self) -> std::io::Result<ExitStatus>;
}

impl Running for Child {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        Child::try_wait(self)
    }

    fn request_stop(&mut self) -> bool {
        false
    }

    fn kill(&mut self) -> std::io::Result<()> {
        Child::kill(self)
    }

    fn wait(&mut self) -> std::io::Result<ExitStatus> {
        Child::wait(self)
    }
}

/// Ask until the program is over or the stop is requested; the exit is
/// asked first, so a program that is over is never reported stopped.
fn wait(
    child: &mut impl Running,
    spec: &Spec,
    stop: &Stop,
    _grace: Duration,
) -> Result<TerminalOutcome, RunError> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(TerminalOutcome {
                    status,
                    stopped: false,
                })
            }
            Ok(None) => {}
            Err(source) => {
                let _ = child.kill();
                return Err(RunError::Wait {
                    program: spec.name(),
                    source,
                });
            }
        }
        if stop.is_requested() {
            // RED: the ask is made, its grace not yet waited.
            let _asked = child.request_stop();
            child.kill().map_err(|source| RunError::Kill {
                program: spec.name(),
                source,
            })?;
            let status = child.wait().map_err(|source| RunError::Wait {
                program: spec.name(),
                source,
            })?;
            return Ok(TerminalOutcome {
                status,
                stopped: true,
            });
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
#[path = "terminal.test.rs"]
mod tests;
