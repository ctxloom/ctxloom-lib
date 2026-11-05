//! The call a command is run with and the failure it ends in.

use angzarr_diagnostic::Diagnostic;
use angzarr_presentation::Format;
use clap::ArgMatches;

/// Exit code: the command failed.
pub const EXIT_FAILED: u8 = 1;
/// Exit code: the command was refused before any state was read.
pub const EXIT_REFUSED: u8 = 2;
/// Exit code: fatal findings (an answer may still have been printed).
pub const EXIT_FATAL: u8 = 3;

/// One command, as an adapter is asked to run it. `G` is what the
/// application adds to every call (its global flags): `()` when it adds
/// none.
pub struct Call<'a, G = ()> {
    /// The matches of the whole command line: the adapter reads its noun's
    /// arguments from them.
    pub matches: &'a ArgMatches,
    /// The `--format` the line gave, if it gave one.
    pub explicit_format: Option<Format>,
    /// What the terminal gives when none was asked for: text on a terminal,
    /// json off one.
    pub default_format: Format,
    /// Whether stdout is a terminal.
    pub stdout_is_terminal: bool,
    /// The application's own global flags.
    pub globals: G,
}

impl<'a> Call<'a, ()> {
    /// A call with no globals and stdout not a terminal.
    pub fn new(
        matches: &'a ArgMatches,
        explicit_format: Option<Format>,
        default_format: Format,
    ) -> Self {
        Call::with_globals(matches, explicit_format, default_format, ())
    }
}

impl<'a, G> Call<'a, G> {
    /// A call carrying `globals`, with stdout not a terminal.
    pub fn with_globals(
        matches: &'a ArgMatches,
        explicit_format: Option<Format>,
        default_format: Format,
        globals: G,
    ) -> Self {
        Call {
            matches,
            explicit_format,
            default_format,
            stdout_is_terminal: false,
            globals,
        }
    }

    /// The format to answer in: the one asked for, else the terminal's.
    pub fn format(&self) -> Format {
        self.explicit_format.unwrap_or(self.default_format)
    }
}

/// Why a command did not complete: its own diagnostic, and the exit code
/// ([`EXIT_FAILED`], [`EXIT_REFUSED`] or [`EXIT_FATAL`]).
#[derive(Debug)]
pub struct Failure {
    pub cause: Box<dyn Diagnostic>,
    pub exit: u8,
}

impl Failure {
    /// `cause`, ending the command with `exit`.
    pub fn new(cause: impl Diagnostic + 'static, exit: u8) -> Failure {
        Failure {
            cause: Box::new(cause),
            exit,
        }
    }
}
