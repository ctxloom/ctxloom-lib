use super::*;

fn violation() -> Finding {
    Finding {
        level: Level::Violation,
        problem: Problem::NoRule {
            package: "m".into(),
            role: "core/mystery".into(),
        },
    }
}

fn warning() -> Finding {
    Finding {
        level: Level::Warning,
        problem: Problem::NoRule {
            package: "w".into(),
            role: "core/other".into(),
        },
    }
}

fn report(findings: Vec<Finding>) -> Report {
    Report {
        findings,
        crates: 3,
        edges: 5,
    }
}

#[test]
fn a_clean_report_is_one_ok_line() {
    let r = report(vec![]);
    assert!(!r.failed(false));
    assert!(!r.failed(true));
    assert_eq!(r.render(false), "ok: 3 crates, 5 edges\n");
}

#[test]
fn a_violation_fails_with_a_count_line_then_the_findings() {
    let r = report(vec![violation()]);
    assert!(r.failed(false));
    assert_eq!(
        r.render(false),
        "1 violation\nviolation: m (core/mystery) has no rule in the rules table\n"
    );
}

#[test]
fn violations_and_warnings_are_counted_and_pluralised() {
    let r = report(vec![violation(), violation(), warning(), warning()]);
    assert!(r.render(false).starts_with("2 violations, 2 warnings\n"));
    let r = report(vec![violation(), warning()]);
    assert!(r.render(false).starts_with("1 violation, 1 warning\n"));
}

#[test]
fn a_warning_alone_passes_and_is_listed_after_the_ok_line() {
    let r = report(vec![warning()]);
    assert!(!r.failed(false));
    assert_eq!(
        r.render(false),
        "ok: 3 crates, 5 edges (1 warning)\nwarning: w (core/other) has no rule in the rules table\n"
    );
}

#[test]
fn a_warning_fails_when_warnings_are_denied() {
    let r = report(vec![warning(), warning()]);
    assert!(r.failed(true));
    assert_eq!(
        r.render(true),
        "0 violations, 2 warnings\n\
         warning: w (core/other) has no rule in the rules table\n\
         warning: w (core/other) has no rule in the rules table\n"
    );
}
