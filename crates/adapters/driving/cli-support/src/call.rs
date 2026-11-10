//! The call a command is run with and the failure it ends in.

use std::io::{self, Write};

use angzarr_diagnostic::Diagnostic;
use angzarr_presentation::{Catalog, Format};
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

/// Why a command did not complete: its own diagnostic, the exit code
/// ([`EXIT_FAILED`], [`EXIT_REFUSED`] or [`EXIT_FATAL`]), and the refusals
/// it gathered, in order, when it refuses for several reasons at once (none
/// by default).
#[derive(Debug)]
pub struct Failure {
    pub cause: Box<dyn Diagnostic>,
    pub exit: u8,
    /// Each reason the command was refused, in the order said; each is
    /// reported as a failure alone would be ([`say_failure`](crate::say_failure)).
    pub refusals: Vec<Box<dyn Diagnostic>>,
}

impl Failure {
    /// `cause`, ending the command with `exit`.
    pub fn new(cause: impl Diagnostic + 'static, exit: u8) -> Failure {
        Failure::boxed(Box::new(cause), exit)
    }

    /// `cause`, already boxed, ending the command with `exit`.
    pub fn boxed(cause: Box<dyn Diagnostic>, exit: u8) -> Failure {
        Failure {
            cause,
            exit,
            refusals: Vec::new(),
        }
    }

    /// This failure with `refusal` after the refusals it has.
    pub fn with_refusal(self, refusal: impl Diagnostic + 'static) -> Failure {
        self.with_refusals([Box::new(refusal) as Box<dyn Diagnostic>])
    }

    /// This failure with `refusals`, in order, after the refusals it has.
    pub fn with_refusals(
        mut self,
        refusals: impl IntoIterator<Item = Box<dyn Diagnostic>>,
    ) -> Failure {
        self.refusals.extend(refusals);
        self
    }

    /// Say this failure on `err` in `format`: its cause, then its refusals
    /// ([`say_failure`](crate::say_failure)). The exit code is the caller's
    /// to return.
    pub fn say(
        &self,
        program: &str,
        format: Format,
        catalog: &Catalog,
        err: &mut dyn Write,
    ) -> io::Result<()> {
        crate::say_failure(program, format, catalog, &*self.cause, &self.refusals, err)
    }
}
