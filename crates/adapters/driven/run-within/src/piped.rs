//! Run a long-lived program on pipes the caller drives, with no deadline.

use std::fmt;
use std::process::{ChildStderr, ChildStdin, ChildStdout, ExitStatus};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{RunError, Running, Spec};

/// The program's three pipes, the caller's to drive.
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
/// thread. Clones share the one program.
#[derive(Clone)]
pub struct Piped {
    running: Arc<Mutex<Box<dyn Running + Send>>>,
    program: String,
    pid: u32,
}

impl fmt::Debug for Piped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Piped").finish_non_exhaustive()
    }
}

/// Start `spec` on pipes, with no deadline.
pub fn run_piped(spec: &Spec, started: impl FnOnce(u32)) -> Result<(Piped, Pipes), RunError> {
    let _ = (spec, started);
    todo!()
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

    /// The program's process id.
    pub fn id(&self) -> u32 {
        let _ = (&self.running, &self.program, self.pid);
        todo!()
    }

    /// The program's exit status if it has exited.
    pub fn try_wait(&self) -> Result<Option<ExitStatus>, RunError> {
        todo!()
    }

    /// Wait for the program to exit.
    pub fn wait(&self) -> Result<ExitStatus, RunError> {
        todo!()
    }

    /// Kill the whole tree at once.
    pub fn kill_tree(&self) -> Result<(), RunError> {
        todo!()
    }

    /// Ask the tree to stop, give the program `grace`, then kill what is left.
    pub fn terminate_tree(&self, grace: Duration) -> Result<(), RunError> {
        let _ = grace;
        todo!()
    }
}

#[cfg(test)]
#[path = "piped.test.rs"]
mod tests;
