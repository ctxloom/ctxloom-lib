use std::io;

use angzarr_diagnostic::Arg;

use super::{diagnostic_samples, RunError, EN_US};

#[test]
fn every_code_is_under_the_crates_name_with_its_message() {
    let (name, text) = EN_US;
    assert_eq!(name, "run-within");
    assert_eq!(
        angzarr_diagnostic::catalog::check(name, text, &diagnostic_samples()),
        []
    );
}

#[test]
fn the_codes_args_and_accessors_are_stable() {
    let cases = [
        ("run-within-not-found", io::ErrorKind::NotFound),
        ("run-within-unrunnable", io::ErrorKind::PermissionDenied),
        ("run-within-wait-failed", io::ErrorKind::Other),
        ("run-within-kill-failed", io::ErrorKind::Other),
    ];
    let samples = diagnostic_samples();
    assert_eq!(samples.len(), cases.len());
    for (sample, (code, _)) in samples.iter().zip(cases) {
        assert_eq!(sample.code(), code);
        let args = sample.args();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].0, "program");
        assert!(matches!(&args[0].1, Arg::Str(s) if s == "demo"));
        assert_eq!(args[1].0, "reason");
        assert!(matches!(&args[1].1, Arg::Str(s) if s == "sample"));
    }
}

#[test]
fn accessors_reach_every_variant() {
    let make = |n: usize| match n {
        0 => RunError::NotFound {
            program: "p0".into(),
            source: io::Error::from(io::ErrorKind::NotFound),
        },
        1 => RunError::Unrunnable {
            program: "p1".into(),
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        },
        2 => RunError::Wait {
            program: "p2".into(),
            source: io::Error::from(io::ErrorKind::Interrupted),
        },
        _ => RunError::Kill {
            program: "p3".into(),
            source: io::Error::from(io::ErrorKind::TimedOut),
        },
    };
    let kinds = [
        io::ErrorKind::NotFound,
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::Interrupted,
        io::ErrorKind::TimedOut,
    ];
    for (n, kind) in kinds.into_iter().enumerate() {
        let error = make(n);
        assert_eq!(error.program(), format!("p{n}"));
        assert_eq!(error.io().kind(), kind);
        let source = std::error::Error::source(&error).unwrap();
        assert_eq!(source.to_string(), io::Error::from(kind).to_string());
    }
}

#[test]
fn a_spawn_failure_is_not_found_only_when_it_found_nothing() {
    let missing = RunError::spawn("a".into(), io::Error::from(io::ErrorKind::NotFound));
    assert!(matches!(missing, RunError::NotFound { .. }));
    let denied = RunError::spawn("a".into(), io::Error::from(io::ErrorKind::PermissionDenied));
    assert!(matches!(denied, RunError::Unrunnable { .. }));
}

#[test]
fn display_is_the_message_in_english() {
    let error = RunError::spawn(
        "demo".into(),
        io::Error::new(io::ErrorKind::NotFound, "gone"),
    );
    let text = error.to_string();
    assert!(text.contains("demo") && text.contains("gone"), "{text}");
}
