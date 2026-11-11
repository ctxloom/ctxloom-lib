//! Run a long-lived program on pipes the caller drives, with no deadline.

use std::fmt;
use std::process::{ChildStderr, ChildStdin, ChildStdout, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use os::process::spawn_tree;

use crate::{RunError, Running, Spec, POLL};

/// The program's three pipes, the caller's to drive. Dropping `stdin` is
/// the program's end of input. The caller reads BOTH outputs (or drops
/// one): a program that fills a pipe no one reads blocks writing it.
#[derive(Debug)]
pub struct Pipes {
    /// What the caller writes, the program reads.
    pub stdin: ChildStdin,
    /// What the program writes to its stdout.
    pub stdout: ChildStdout,
    /// What the program writes to its stderr.
    pub stderr: ChildStderr,
}

/// A program started by [`run_piped`]: its tree, waited on and ended from any
/// thread. Clones share the one program, so one thread can wait while
/// another kills. Dropping every clone does not end the program (as
/// dropping an [`os::process::Tree`] does not): a caller ends it, by its
/// input's end, [`terminate_tree`](Self::terminate_tree) or
/// [`kill_tree`](Self::kill_tree), or leaves it on purpose. Its `Debug`
/// names the program and its pid.
#[derive(Clone)]
pub struct Piped {
    running: Arc<Mutex<Box<dyn Running + Send>>>,
    program: String,
    pid: u32,
}

impl fmt::Debug for Piped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Piped")
            .field("program", &self.program)
            .field("pid", &self.pid)
            .finish_non_exhaustive()
    }
}

/// Start `spec` with its stdin, stdout and stderr piped to the caller and no
/// deadline, as a tree of its own (a process group on unix, a job object on
/// Windows), and answer its handle and its pipes. `started` is called once
/// with the pid after the spawn succeeded, never when it failed. The
/// environment is exactly the [`Spec`]'s; its input
/// ([`Spec::stdin`]) is not fed: the caller writes the pipe. The crate docs
/// state what this guarantees and does not.
pub fn run_piped(spec: &Spec, started: impl FnOnce(u32)) -> Result<(Piped, Pipes), RunError> {
    let mut command = spec.command();
    command.stdin(Stdio::piped());
    let mut tree = spawn_tree(command).map_err(|e| RunError::spawn(spec.name(), e))?;
    let pid = tree.id();
    started(pid);
    let (Some(stdin), Some(stdout), Some(stderr)) =
        (tree.take_stdin(), tree.take_stdout(), tree.take_stderr())
    else {
        unreachable!("the command was given three pipes, taken once each")
    };
    let child = Piped {
        running: Arc::new(Mutex::new(Box::new(tree))),
        program: spec.name(),
        pid,
    };
    Ok((
        child,
        Pipes {
            stdin,
            stdout,
            stderr,
        },
    ))
}

impl Piped {
    #[cfg(test)]
    pub(crate) fn over(running: impl Running + Send + 'static, program: &str, pid: u32) -> Piped {
        Piped {
            running: Arc::new(Mutex::new(Box::new(running))),
            program: program.to_string(),
            pid,
        }
    }

    /// The program's process id (its tree's leader).
    pub fn id(&self) -> u32 {
        self.pid
    }

    /// The program's exit status if it has exited (the same status ever
    /// after), without waiting. A look that fails kills the tree and is the
    /// [`RunError::Wait`].
    pub fn try_wait(&self) -> Result<Option<ExitStatus>, RunError> {
        let mut running = self.lock();
        running.try_wait().map_err(|source| {
            let _ = running.kill_tree();
            RunError::Wait {
                program: self.program.clone(),
                source,
            }
        })
    }

    /// Wait for the program to exit and answer its status: its own exit, or
    /// the kill a [`kill_tree`](Self::kill_tree) or
    /// [`terminate_tree`](Self::terminate_tree) from any thread made. It
    /// asks between pauses rather than blocking in the system's wait, so the
    /// handle is free for an end meanwhile. It does not wait for a
    /// descendant that outlives the program.
    pub fn wait(&self) -> Result<ExitStatus, RunError> {
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            std::thread::sleep(POLL);
        }
    }

    /// Kill the whole tree at once (SIGKILL to the group on unix,
    /// `TerminateJobObject` on Windows), as the deadline kill of
    /// [`run_within`](crate::run_within) does, and reap the program.
    /// Idempotent; after the program has exited it still kills a descendant
    /// that outlived it. A kill that failed is the [`RunError::Kill`].
    pub fn kill_tree(&self) -> Result<(), RunError> {
        let killed = self.lock().kill_tree();
        killed.map_err(|source| self.kill_error(source))
    }

    /// Ask the tree to stop (SIGTERM to the group on unix; on Windows, which
    /// has no ask, the kill is at once), give the program up to `grace` to
    /// exit, then kill whatever is left of the tree. The handle is held for
    /// the grace: a [`try_wait`](Self::try_wait) or
    /// [`wait`](Self::wait) elsewhere answers after it. A failure is the
    /// [`RunError::Kill`].
    pub fn terminate_tree(&self, grace: Duration) -> Result<(), RunError> {
        let ended = self.lock().terminate_tree(grace);
        ended.map_err(|source| self.kill_error(source))
    }

    fn kill_error(&self, source: std::io::Error) -> RunError {
        RunError::Kill {
            program: self.program.clone(),
            source,
        }
    }

    /// The tree, whoever else holds the handle. A thread that panicked
    /// holding it left nothing half-done that matters here.
    fn lock(&self) -> MutexGuard<'_, Box<dyn Running + Send>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "piped.test.rs"]
mod tests;
