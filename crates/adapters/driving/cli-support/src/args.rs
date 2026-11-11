//! Reading arguments and writing answers.

use std::io::{self, Write};

use angzarr_presentation::{write_json, Format};
use clap::{Arg, ArgAction, ArgMatches};
use serde::Serialize;

use crate::Failure;

/// Write `value` as json, or `text` as words, by `format`.
pub fn emit<T: Serialize>(
    format: Format,
    out: &mut dyn Write,
    value: &T,
    text: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    match format {
        Format::Json => write_json(out, value),
        Format::Text => text(out),
    }
}

/// [`emit`], with a write that fails made into the [`Failure`] `fault` says.
pub fn emit_or<T: Serialize>(
    format: Format,
    out: &mut dyn Write,
    value: &T,
    text: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    fault: impl FnOnce(io::Error) -> Failure,
) -> Result<(), Failure> {
    emit(format, out, value, text).map_err(fault)
}

/// Whether the flag `name` was given (false for a name the command lacks).
pub fn flag(matches: &ArgMatches, name: &str) -> bool {
    matches
        .try_get_one::<bool>(name)
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false)
}

/// The one value of `name`, if it has one.
pub fn one(matches: &ArgMatches, name: &str) -> Option<String> {
    matches.try_get_one::<String>(name).ok().flatten().cloned()
}

/// Every value of `name`.
pub fn many(matches: &ArgMatches, name: &str) -> Vec<String> {
    matches
        .try_get_many::<String>(name)
        .ok()
        .flatten()
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

/// The name of the global flag that asks for the distinguishing successes
/// (1-9) instead of 0 ([`Exit::code`](crate::Exit::code)).
pub const DETAILED_EXITCODE: &str = "detailed-exitcode";

/// The global `--detailed-exitcode` flag, for a program's top command.
pub fn detailed_exitcode_arg() -> Arg {
    Arg::new(DETAILED_EXITCODE)
        .long(DETAILED_EXITCODE)
        .action(ArgAction::SetTrue)
        .global(true)
        .help("Exit 1-9 for a success that says more (1: would change something); without it every success exits 0")
}

/// Whether the line asked for `--detailed-exitcode`, read at any depth.
pub fn detailed_exitcode(_matches: &ArgMatches) -> bool {
    false
}
