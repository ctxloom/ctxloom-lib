//! What every driving CLI adapter shares, so none writes it again: the
//! [`Call`] a command is run with, the [`Failure`] it ends in, the notices
//! printer, the failure reporter, and the helpers that read arguments and
//! write answers.
//!
//! It is a presentation crate: domain-free, and linking no use case. What an
//! application adds to every call (its global flags) rides in the call's
//! `globals`, a type of the application's own.

mod args;
mod call;
mod notices;
mod report;

pub use args::{emit, emit_or, flag, many, one};
pub use call::{Call, Failure, EXIT_FAILED, EXIT_FATAL, EXIT_REFUSED};
pub use notices::{say_notices, say_warning, FixOf};
pub use report::{report_failure, say_failure, ReportText, Reported};

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
