//! A parser's error as an exit, a failure of an error that says its class,
//! and the global `--detailed-exitcode` flag.

use clap::{error::ErrorKind, Command};

use super::*;

#[test]
fn test_a_usage_error_is_refused_and_help_or_version_is_ok() {
    let usage = Command::new("demo").error(ErrorKind::UnknownArgument, "no such flag");
    assert_eq!(usage_exit(&usage), Exit::REFUSED);
    let help = Command::new("demo").error(ErrorKind::DisplayHelp, "help");
    assert_eq!(usage_exit(&help), Exit::OK);
    let version = Command::new("demo").error(ErrorKind::DisplayVersion, "1");
    assert_eq!(usage_exit(&version), Exit::OK);
}

/// An error of a crate, which says its own class.
#[derive(Debug)]
struct Taken;

impl std::fmt::Display for Taken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl angzarr_diagnostic::Diagnostic for Taken {
    fn code(&self) -> &str {
        "taken"
    }

    fn args(&self) -> Vec<(&str, angzarr_diagnostic::Arg)> {
        Vec::new()
    }
}

impl ExitClass for Taken {
    fn exit(&self) -> Exit {
        Exit::REFUSED
    }
}

#[test]
fn test_a_failure_of_an_error_that_says_its_class_ends_with_that_class() {
    let failure = crate::Failure::of(Taken);
    assert_eq!(failure.exit, Exit::REFUSED);
    assert_eq!(failure.cause.code(), "taken");
    assert!(failure.refusals.is_empty());
}

#[test]
fn test_the_detailed_exitcode_flag_is_global_and_off_unless_given() {
    let command = Command::new("demo")
        .arg(crate::detailed_exitcode_arg())
        .subcommand(Command::new("sub"));
    let given = command
        .clone()
        .try_get_matches_from(["demo", "sub", "--detailed-exitcode"])
        .expect("parsed");
    let (_, sub) = given.subcommand().expect("the sub");
    assert!(
        crate::detailed_exitcode(sub),
        "read at the depth it was given"
    );
    assert!(crate::detailed_exitcode(&given), "and at the top");
    let plain = command
        .try_get_matches_from(["demo", "sub"])
        .expect("parsed");
    assert!(!crate::detailed_exitcode(&plain));
    let none = Command::new("other").get_matches_from(["other"]);
    assert!(!crate::detailed_exitcode(&none), "a line without the flag");
}
