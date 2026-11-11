//! What every driving CLI adapter shares, so none writes it again: the
//! [`Call`] a command is run with, the [`Failure`] it ends in, the notices
//! printer, the failure reporter, the [`Exit`] a command ends with (the
//! exit-code [`convention`]), and the helpers that read arguments and write
//! answers.
//!
//! It is a presentation crate: domain-free, and linking no use case. What an
//! application adds to every call (its global flags) rides in the call's
//! `globals`, a type of the application's own.

mod args;
mod call;
mod exit;
mod notices;
mod report;

pub use args::{
    detailed_exitcode, detailed_exitcode_arg, emit, emit_or, flag, many, one, DETAILED_EXITCODE,
};
pub use call::{Call, Failure};
pub use exit::{convention, usage_exit, Class, Exit, ExitClass};
pub use notices::{say_notices, say_warning, FixOf};
pub use report::{report_failure, say_failure, ReportText, Reported};

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
