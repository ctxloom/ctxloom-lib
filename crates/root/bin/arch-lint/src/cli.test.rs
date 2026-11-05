use std::path::{Path, PathBuf};

use file_system::Os;

use super::*;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn fs() -> Os {
    Os::new(os::host(), &fixtures()).unwrap()
}

fn args(workspace: &str, rules: &str) -> Args {
    Args {
        manifest_path: fixtures().join(workspace).join("Cargo.toml"),
        rules: fixtures().join(rules),
        deny_warnings: false,
    }
}

const VIOLATIONS: &str = "\
11 violations, 2 warnings
violation: fx-misplaced is at crates/oddly/fx-misplaced, not crates/<ring>/<role>/<crate>
violation: fx-mystery (core/mystery) has no rule in the rules table
violation: fx-app (core/application) -> fx-tool (root/bin) [build]: core/application may depend on core/*; nothing but the root depends on the root
violation: fx-domain (core/domain) -> fx-driven-a (adapters/driven) [normal]: core/domain may depend on core/domain, core/wire; a domain never depends on a read model
violation: fx-domain (core/domain) -> fx-readmodel (core/read-model) [normal]: core/domain may depend on core/domain, core/wire; a domain never depends on a read model
violation: fx-driven-a (adapters/driven) -> fx-driven-b (adapters/driven) [normal]: adapters/driven may depend on core/ports, core/wire; a driven adapter sees only ports
violation: fx-driven-b (adapters/driven) -> fx-app (core/application) [normal]: adapters/driven may depend on core/ports, core/wire; a driven adapter sees only ports
violation: fx-driving-a (adapters/driving) -> fx-driven-a (adapters/driven) [normal]: adapters/driving may depend on core/application, @presentation; a driving adapter sees only the application layer
violation: fx-driving-a (adapters/driving) -> fx-ports (core/ports) [normal]: adapters/driving may depend on core/application, @presentation; a driving adapter sees only the application layer
violation: fx-readmodel (core/read-model) -> fx-app (core/application) [normal]: core/read-model may depend on core/read-model, core/domain, core/wire; a read model never depends on the application layer
violation: exception fx-ghost -> fx-wire matches no finding
warning: fx-driven-b (adapters/driven) -> fx-driving-a (adapters/driving) [dev]: adapters/driven may depend on core/ports, core/wire; a driven adapter sees only ports
warning: fx-driving-b (adapters/driving) -> fx-ports (core/ports) [dev]: adapters/driving may depend on core/application, @presentation; a driving adapter sees only the application layer
";

#[test]
fn the_violations_workspace_reports_exactly_one_finding_per_rule_broken() {
    let out = run(&args("violations", "violations-rules.toml"), &fs());
    assert_eq!(out.text, VIOLATIONS);
    assert_eq!(out.code, 1);
}

#[test]
fn the_clean_workspace_passes_with_one_line() {
    let out = run(&args("clean", "clean-rules.toml"), &fs());
    assert_eq!(out.text, "ok: 11 crates, 26 edges\n");
    assert_eq!(out.code, 0);
}

#[test]
fn warnings_alone_pass_unless_denied() {
    let mut a = args("warned", "clean-rules.toml");
    let out = run(&a, &fs());
    assert_eq!(
        out.text,
        "ok: 3 crates, 2 edges (1 warning)\n\
         warning: fx-store (adapters/driven) -> fx-app (core/application) [dev]: \
         adapters/driven may depend on core/ports, core/wire; a driven adapter sees only ports\n"
    );
    assert_eq!(out.code, 0);
    a.deny_warnings = true;
    let out = run(&a, &fs());
    assert!(
        out.text.starts_with("0 violations, 1 warning\n"),
        "{}",
        out.text
    );
    assert_eq!(out.code, 1);
}

#[test]
fn rules_that_cannot_be_read_are_a_tool_failure() {
    let out = run(&args("clean", "no-such-rules.toml"), &fs());
    assert_eq!(out.code, 2);
    assert!(
        out.text.starts_with("cannot read the rules "),
        "{}",
        out.text
    );
    assert!(out.text.contains("no-such-rules.toml"), "{}", out.text);
}

#[test]
fn rules_that_do_not_parse_are_a_tool_failure() {
    let out = run(&args("clean", "bad-rules.toml"), &fs());
    assert_eq!(out.code, 2);
    assert!(out.text.contains("bad-rules.toml: "), "{}", out.text);
}

#[test]
fn a_workspace_that_cannot_be_read_is_a_tool_failure() {
    let out = run(&args("no-such-workspace", "clean-rules.toml"), &fs());
    assert_eq!(out.code, 2);
    assert!(
        out.text.starts_with("cannot read the workspace "),
        "{}",
        out.text
    );
}

#[test]
fn the_arguments_default_to_the_working_directorys_files() {
    use clap::Parser;
    let a = Args::parse_from(["arch-lint"]);
    assert_eq!(a.manifest_path, Path::new("Cargo.toml"));
    assert_eq!(a.rules, Path::new("arch-rules.toml"));
    assert!(!a.deny_warnings);
    let a = Args::parse_from([
        "arch-lint",
        "--manifest-path",
        "m",
        "--rules",
        "r",
        "--deny-warnings",
    ]);
    assert_eq!(a.manifest_path, Path::new("m"));
    assert_eq!(a.rules, Path::new("r"));
    assert!(a.deny_warnings);
}
