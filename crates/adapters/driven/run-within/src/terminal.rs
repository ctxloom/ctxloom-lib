//! Run an interactive program on the caller's terminal.

use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::{RunError, Spec};

/// How often the program is asked whether it has exited, and the stop asked
/// after.
const POLL: Duration = Duration::from_millis(5);

/// A request, from anywhere, that a [`run_terminal`] end its program. Clones
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

/// Run `spec` with the caller's stdin, stdout and stderr, in the caller's
/// process group, until it exits or `stop` is requested (then its direct
/// child is killed). `started` is called once with the pid after the spawn
/// succeeded, never when it failed. The crate docs state what this
/// guarantees and does not.
///
/// A `stop` requested before the call still lets the program start (and
/// `started` be called); it is then killed at the first look.
pub fn run_terminal(
    spec: &Spec,
    stop: &Stop,
    started: impl FnOnce(u32),
) -> Result<TerminalOutcome, RunError> {
    let mut child = command(spec)
        .spawn()
        .map_err(|e| RunError::spawn(spec.name(), e))?;
    started(child.id());
    wait(&mut child, spec, stop)
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
    fn kill(&mut self) -> std::io::Result<()>;
    fn wait(&mut self) -> std::io::Result<ExitStatus>;
}

impl Running for Child {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        Child::try_wait(self)
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
fn wait(child: &mut impl Running, spec: &Spec, stop: &Stop) -> Result<TerminalOutcome, RunError> {
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
