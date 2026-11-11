//! The exit pattern: the blocks and their bounds, the collapse of the
//! distinguishing successes without `--detailed-exitcode`, the severity
//! order, and the usage error's class.

use std::panic::catch_unwind;

use super::*;

#[test]
fn test_the_defined_codes_are_zero_one_ten_twenty_and_twenty_one() {
    let detailed = [
        Exit::OK,
        Exit::WOULD_CHANGE,
        Exit::FAILED,
        Exit::REFUSED,
        Exit::FATAL_FINDINGS,
    ]
    .map(|exit| exit.code(true));
    assert_eq!(detailed, [0, 1, 10, 20, 21]);
}

#[test]
fn test_each_defined_code_is_of_its_class() {
    assert_eq!(Exit::OK.class(), Class::Success);
    assert_eq!(Exit::WOULD_CHANGE.class(), Class::Success);
    assert_eq!(Exit::FAILED.class(), Class::Failed);
    assert_eq!(Exit::REFUSED.class(), Class::Declined);
    assert_eq!(Exit::FATAL_FINDINGS.class(), Class::Declined);
}

#[test]
fn test_a_distinguishing_success_collapses_to_zero_without_the_flag() {
    for code in 0..=9 {
        let exit = Exit::success(code);
        assert_eq!(exit.code(false), 0, "success {code} without the flag");
        assert_eq!(exit.code(true), code, "success {code} with the flag");
        assert_eq!(exit.class(), Class::Success);
    }
}

#[test]
fn test_failures_declines_and_signals_never_collapse() {
    for code in 10..=19 {
        let exit = Exit::failed(code);
        assert_eq!((exit.code(false), exit.code(true)), (code, code));
        assert_eq!(exit.class(), Class::Failed);
    }
    for code in 20..=29 {
        let exit = Exit::declined(code);
        assert_eq!((exit.code(false), exit.code(true)), (code, code));
        assert_eq!(exit.class(), Class::Declined);
    }
    for n in 1..=127 {
        let exit = Exit::signal(n);
        assert_eq!((exit.code(false), exit.code(true)), (128 + n, 128 + n));
        assert_eq!(exit.class(), Class::Signalled);
        assert_eq!(exit.signal_number(), Some(n));
    }
}

#[test]
fn test_only_a_signal_has_a_signal_number() {
    for exit in [Exit::OK, Exit::WOULD_CHANGE, Exit::FAILED, Exit::REFUSED] {
        assert_eq!(exit.signal_number(), None, "{exit:?}");
    }
    assert_eq!(Exit::signal(15).signal_number(), Some(15));
}

#[test]
fn test_a_success_is_a_code_from_zero_to_nine() {
    assert!(catch_unwind(|| Exit::success(9)).is_ok());
    assert!(
        catch_unwind(|| Exit::success(10)).is_err(),
        "10 is a failure"
    );
}

#[test]
fn test_a_failure_is_a_code_from_ten_to_nineteen() {
    assert!(catch_unwind(|| Exit::failed(9)).is_err(), "9 is a success");
    assert!(catch_unwind(|| Exit::failed(10)).is_ok());
    assert!(catch_unwind(|| Exit::failed(19)).is_ok());
    assert!(
        catch_unwind(|| Exit::failed(20)).is_err(),
        "20 is a decline"
    );
}

#[test]
fn test_a_decline_is_a_code_from_twenty_to_twenty_nine() {
    assert!(
        catch_unwind(|| Exit::declined(19)).is_err(),
        "19 is a failure"
    );
    assert!(catch_unwind(|| Exit::declined(20)).is_ok());
    assert!(catch_unwind(|| Exit::declined(29)).is_ok());
    assert!(
        catch_unwind(|| Exit::declined(30)).is_err(),
        "30 is no block's"
    );
}

#[test]
fn test_a_signal_is_numbered_from_one_to_one_hundred_twenty_seven() {
    assert!(
        catch_unwind(|| Exit::signal(0)).is_err(),
        "there is no signal 0"
    );
    assert!(catch_unwind(|| Exit::signal(1)).is_ok());
    assert!(catch_unwind(|| Exit::signal(127)).is_ok());
    assert!(
        catch_unwind(|| Exit::signal(128)).is_err(),
        "128 + 128 is no exit code"
    );
}

#[test]
fn test_a_decline_outranks_a_failure() {
    assert_eq!(Exit::FAILED.worst(Exit::REFUSED), Exit::REFUSED);
    assert_eq!(Exit::REFUSED.worst(Exit::FAILED), Exit::REFUSED);
    assert_eq!(Exit::failed(19).worst(Exit::declined(20)), Exit::REFUSED);
}

#[test]
fn test_fatal_findings_outrank_a_refusal() {
    assert_eq!(
        Exit::REFUSED.worst(Exit::FATAL_FINDINGS),
        Exit::FATAL_FINDINGS
    );
    assert_eq!(
        Exit::FATAL_FINDINGS.worst(Exit::REFUSED),
        Exit::FATAL_FINDINGS
    );
}

#[test]
fn test_a_signal_outranks_everything_and_any_error_outranks_a_success() {
    let signal = Exit::signal(1);
    for other in [Exit::OK, Exit::success(9), Exit::FAILED, Exit::declined(29)] {
        assert_eq!(signal.worst(other), signal, "{other:?}");
        assert_eq!(other.worst(signal), signal, "{other:?}");
    }
    assert_eq!(Exit::success(9).worst(Exit::FAILED), Exit::FAILED);
    assert_eq!(Exit::WOULD_CHANGE.worst(Exit::OK), Exit::WOULD_CHANGE);
    assert_eq!(Exit::OK.worst(Exit::WOULD_CHANGE), Exit::WOULD_CHANGE);
    assert_eq!(Exit::signal(2).worst(Exit::signal(15)), Exit::signal(15));
}

#[test]
fn test_the_severity_order_is_the_order_of_the_detailed_codes() {
    let mut exits = [
        Exit::signal(2),
        Exit::FATAL_FINDINGS,
        Exit::OK,
        Exit::FAILED,
        Exit::REFUSED,
        Exit::WOULD_CHANGE,
    ];
    exits.sort();
    assert_eq!(exits.map(|exit| exit.code(true)), [0, 1, 10, 20, 21, 130]);
}

#[test]
fn test_a_usage_error_is_refused_and_help_or_version_is_ok() {
    assert_eq!(Exit::usage(true), Exit::REFUSED);
    assert_eq!(Exit::usage(false), Exit::OK);
}

#[test]
fn test_the_process_exit_code_is_the_code_under_the_flag() {
    use std::process::ExitCode;
    assert_eq!(Exit::WOULD_CHANGE.process(false), ExitCode::from(0));
    assert_eq!(Exit::WOULD_CHANGE.process(true), ExitCode::from(1));
    assert_eq!(Exit::REFUSED.process(false), ExitCode::from(20));
}

/// An error of a crate, which says its own class.
struct Taken;

impl ExitClass for Taken {
    fn exit(&self) -> Exit {
        Exit::REFUSED
    }
}

#[test]
fn test_an_error_says_its_own_class() {
    assert_eq!(Taken.exit(), Exit::REFUSED);
}

// A child program's own codes are not the convention's: its status is said in
// the classes, never passed through (its 1 would read as a success).
#[test]
fn test_a_child_status_is_said_in_the_classes() {
    let of = |code, signal| Exit::from_child_status(code, signal).code(true);
    assert_eq!(of(Some(0), None), 0, "a child that exited 0 is ok");
    for code in [1, 2, 9, 10, 20, 21, 30, 127, 128, 256, -1] {
        assert_eq!(of(Some(code), None), 10, "a child's {code} failed");
    }
    assert_eq!(of(Some(129), None), 129, "a shell's 128+1 is signal 1");
    assert_eq!(of(Some(143), None), 143, "a shell's 128+15 is signal 15");
    assert_eq!(of(Some(255), None), 255, "a shell's 128+127 is signal 127");
    assert_eq!(of(None, Some(15)), 143, "a child the signal ended");
    assert_eq!(of(None, Some(1)), 129);
    assert_eq!(of(None, Some(127)), 255);
    assert_eq!(of(Some(0), Some(9)), 137, "the signal is what ended it");
    for signal in [0, 128, -9] {
        assert_eq!(of(None, Some(signal)), 10, "no such signal: {signal}");
    }
    assert_eq!(of(None, None), 10, "a status that says nothing failed");
}
