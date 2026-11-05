//! Why a program could not be run to an outcome.

use std::fmt;
use std::io;

use angzarr_diagnostic::{code, Arg, Diagnostic};

/// This crate's en-US messages, its name first.
pub const EN_US: (&str, &str) = (
    env!("CARGO_PKG_NAME"),
    include_str!(concat!(env!("OUT_DIR"), "/en-US.ftl")),
);

/// Why a run produced no outcome. Each variant keeps the operating system's
/// own error, for a caller that words it differently
/// ([`io`](RunError::io)).
#[derive(Debug)]
pub enum RunError {
    /// The program, or the directory to run it in, does not exist.
    NotFound { program: String, source: io::Error },
    /// The program exists but could not be started (not executable, no
    /// permission, a resource limit, ...).
    Unrunnable { program: String, source: io::Error },
    /// Its exit could not be asked after; the tree was killed.
    Wait { program: String, source: io::Error },
    /// It outlived its deadline and the kill failed, so it may still run.
    Kill { program: String, source: io::Error },
}

impl RunError {
    /// What a failed spawn was: `NotFound` or `Unrunnable`, by the error's
    /// kind.
    pub(crate) fn spawn(program: String, source: io::Error) -> RunError {
        if source.kind() == io::ErrorKind::NotFound {
            RunError::NotFound { program, source }
        } else {
            RunError::Unrunnable { program, source }
        }
    }

    /// The error's code as a `&'static str`.
    pub fn static_code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => code!("not-found"),
            Self::Unrunnable { .. } => code!("unrunnable"),
            Self::Wait { .. } => code!("wait-failed"),
            Self::Kill { .. } => code!("kill-failed"),
        }
    }

    /// The program, as it was named.
    pub fn program(&self) -> &str {
        match self {
            Self::NotFound { program, .. }
            | Self::Unrunnable { program, .. }
            | Self::Wait { program, .. }
            | Self::Kill { program, .. } => program,
        }
    }

    /// The operating system's error.
    pub fn io(&self) -> &io::Error {
        match self {
            Self::NotFound { source, .. }
            | Self::Unrunnable { source, .. }
            | Self::Wait { source, .. }
            | Self::Kill { source, .. } => source,
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.io())
    }
}

impl Diagnostic for RunError {
    fn code(&self) -> &str {
        self.static_code()
    }

    fn args(&self) -> Vec<(&str, Arg)> {
        vec![
            ("program", self.program().into()),
            ("reason", self.io().to_string().into()),
        ]
    }
}

/// One value of every error, for the catalog test.
#[cfg(any(test, feature = "test-support"))]
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
    let io = |kind| io::Error::new(kind, "sample");
    let program = || "demo".to_string();
    vec![
        Box::new(RunError::NotFound {
            program: program(),
            source: io(io::ErrorKind::NotFound),
        }),
        Box::new(RunError::Unrunnable {
            program: program(),
            source: io(io::ErrorKind::PermissionDenied),
        }),
        Box::new(RunError::Wait {
            program: program(),
            source: io(io::ErrorKind::Other),
        }),
        Box::new(RunError::Kill {
            program: program(),
            source: io(io::ErrorKind::Other),
        }),
    ]
}

#[cfg(test)]
#[path = "error.test.rs"]
mod tests;
