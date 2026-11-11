//! How a command ends: its class and its code, by the exit-code convention
//! (`docs/exit-codes.md` at this repository's root, rendered as
//! [`convention`](crate::convention)).
//!
//! The codes come in blocks: 0-9 successes, 10-19 failures, 20-29 declines,
//! 128+n a signal. Only the success block collapses: by long Unix convention
//! a non-zero status is an error (`set -e`, `&&`, make and CI all read it so),
//! so a success exits 0 unless the caller opted in with
//! `--detailed-exitcode`, and only then are the distinguishing successes 1-9
//! returned. A failure or a decline is already non-zero and reads correctly
//! as an error, so it is always returned as it is.

use std::process::ExitCode;

/// A signal's code is this plus its number (the shell's convention).
const SIGNAL_BASE: u8 = 128;

/// The class of an [`Exit`]: which block its code is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    /// 0-9: the command did what was asked.
    Success,
    /// 10-19: the command tried, and something broke.
    Failed,
    /// 20-29: the command did not try: the request or the state forbids it.
    Declined,
    /// 128+n: the command was stopped by signal n.
    Signalled,
}

/// How a command ends. Built only by the constructors and constants, so its
/// code is always in its block. Its order is the severity order (the order
/// of the detailed codes: a signal over a decline over a failure over a
/// success, and within a block the higher code): see [`Exit::worst`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Exit(u8);

impl Exit {
    /// 0: the command succeeded.
    pub const OK: Exit = Exit(0);
    /// 1: a dry run or preview succeeded, and applying it would change
    /// something (returned only under `--detailed-exitcode`).
    pub const WOULD_CHANGE: Exit = Exit(1);
    /// 10: the command tried and something broke.
    pub const FAILED: Exit = Exit(10);
    /// 20: the request or the state forbids it (a usage error too).
    pub const REFUSED: Exit = Exit(20);
    /// 21: the input was examined and cannot produce a working result.
    pub const FATAL_FINDINGS: Exit = Exit(21);

    /// The success `code`, 0 to 9. Panics on any other.
    pub const fn success(code: u8) -> Exit {
        assert!(code <= 9, "a success is a code from 0 to 9");
        Exit(code)
    }

    /// The failure `code`, 10 to 19. Panics on any other.
    pub const fn failed(code: u8) -> Exit {
        assert!(
            10 <= code && code <= 19,
            "a failure is a code from 10 to 19"
        );
        Exit(code)
    }

    /// The decline `code`, 20 to 29. Panics on any other.
    pub const fn declined(code: u8) -> Exit {
        assert!(
            20 <= code && code <= 29,
            "a decline is a code from 20 to 29"
        );
        Exit(code)
    }

    /// Stopped by signal `number`, 1 to 127: the code 128 + `number`.
    /// Panics on any other.
    pub const fn signal(number: u8) -> Exit {
        assert!(
            1 <= number && number <= 127,
            "a signal is numbered from 1 to 127"
        );
        Exit(SIGNAL_BASE + number)
    }

    /// The exit of an argument parser's error: a usage error is refused
    /// (20, never the parser's own 2); help and version are ok.
    pub fn usage(error: &clap::Error) -> Exit {
        if error.use_stderr() {
            Exit::REFUSED
        } else {
            Exit::OK
        }
    }

    /// The block this exit is in.
    pub const fn class(self) -> Class {
        match self.0 {
            0..=9 => Class::Success,
            10..=19 => Class::Failed,
            20..=29 => Class::Declined,
            _ => Class::Signalled,
        }
    }

    /// The signal that stopped the command, if a signal did.
    pub const fn signal_number(self) -> Option<u8> {
        match self.class() {
            Class::Signalled => Some(self.0 - SIGNAL_BASE),
            _ => None,
        }
    }

    /// The process's exit status. This is the one place the collapse rule
    /// lives: without `detailed` (the `--detailed-exitcode` flag) a success
    /// is 0 whatever its code; every other exit is its code.
    pub const fn code(self, detailed: bool) -> u8 {
        match self.class() {
            Class::Success if !detailed => 0,
            _ => self.0,
        }
    }

    /// [`code`](Exit::code) as the process's [`ExitCode`].
    pub fn process(self, detailed: bool) -> ExitCode {
        ExitCode::from(self.code(detailed))
    }

    /// The more severe of `self` and `other`, the one a command that has
    /// both to report exits with. A signal outranks everything; a decline
    /// outranks a failure (it is a verdict on the request, which a retry
    /// unchanged would meet again, where a failure may be transient); any
    /// error outranks a success; within a block the higher code wins.
    pub fn worst(self, other: Exit) -> Exit {
        self.max(other)
    }
}

/// An error that says its own class: what a [`Failure`](crate::Failure)
/// made of it ([`Failure::of`](crate::Failure::of)) exits with.
pub trait ExitClass {
    /// The exit this error ends a command with.
    fn exit(&self) -> Exit;
}

#[cfg(test)]
#[path = "exit.test.rs"]
mod tests;
