use super::*;

const TABLE: &str = r#"
dev_checked = ["adapters/driven", "adapters/driving"]

[classes]
presentation = ["presentation", "*-presentation"]

[roles."core/wire"]
allow = ["core/wire"]

[roles."core/domain"]
allow = ["core/domain", "core/wire"]
rule = "a domain never depends on a read model"

[roles."core/read-model"]
allow = ["core/read-model", "core/domain", "core/wire"]

[roles."core/application"]
allow = ["core/*"]

[roles."adapters/driven"]
allow = ["core/ports", "core/wire"]

[roles."adapters/driving"]
allow = ["core/application", "@presentation"]

[roles."core/ports"]
allow = ["core/wire"]

[roles."root/bin"]
allow = ["*"]
"#;

fn pkg(name: &str, dir: &str, deps: &[(&str, Kind)]) -> Package {
    Package {
        name: name.to_string(),
        dir: dir.to_string(),
        deps: deps.iter().map(|(n, k)| (n.to_string(), *k)).collect(),
    }
}

fn run(table: &str, packages: Vec<Package>) -> Report {
    check(&Graph { packages }, &Rules::parse(table).unwrap())
}

fn lines(report: &Report) -> Vec<String> {
    report.findings.iter().map(ToString::to_string).collect()
}

fn base() -> Vec<Package> {
    vec![
        pkg("wire", "crates/core/wire/wire", &[]),
        pkg(
            "domain",
            "crates/core/domain/domain",
            &[("wire", Kind::Normal)],
        ),
        pkg(
            "rm",
            "crates/core/read-model/rm",
            &[("domain", Kind::Normal)],
        ),
        pkg(
            "ports",
            "crates/core/ports/ports",
            &[("wire", Kind::Normal)],
        ),
        pkg(
            "app",
            "crates/core/application/app",
            &[("rm", Kind::Normal)],
        ),
        pkg(
            "store",
            "crates/adapters/driven/store",
            &[("ports", Kind::Normal)],
        ),
        pkg(
            "cli",
            "crates/adapters/driving/cli",
            &[("app", Kind::Normal)],
        ),
        pkg("presentation", "crates/adapters/driving/presentation", &[]),
        pkg(
            "bin",
            "crates/root/bin/bin",
            &[("store", Kind::Normal), ("cli", Kind::Normal)],
        ),
    ]
}

fn with(mut packages: Vec<Package>, name: &str, dep: &str, kind: Kind) -> Vec<Package> {
    packages
        .iter_mut()
        .find(|p| p.name == name)
        .unwrap()
        .deps
        .insert((dep.to_string(), kind));
    packages
}

#[test]
fn a_clean_workspace_has_no_findings_and_counts_its_crates_and_edges() {
    let report = run(TABLE, base());
    assert_eq!(lines(&report), Vec::<String>::new());
    assert_eq!(report.crates, 9);
    assert_eq!(report.edges, 8);
}

#[test]
fn a_core_crate_depending_outside_the_core_is_a_violation() {
    let report = run(TABLE, with(base(), "domain", "store", Kind::Normal));
    assert_eq!(
        lines(&report),
        [
            "violation: domain (core/domain) -> store (adapters/driven) [normal]: \
          core/domain may depend on core/domain, core/wire; a domain never depends on a read model"
        ]
    );
    assert_eq!(report.violations(), 1);
    assert_eq!(report.warnings(), 0);
}

#[test]
fn a_domain_depending_on_a_read_model_is_a_violation() {
    let report = run(TABLE, with(base(), "domain", "rm", Kind::Normal));
    assert_eq!(report.violations(), 1);
    assert!(
        lines(&report)[0].starts_with("violation: domain (core/domain) -> rm (core/read-model)")
    );
}

#[test]
fn a_build_dependency_is_checked_like_a_normal_one() {
    let report = run(TABLE, with(base(), "store", "app", Kind::Build));
    assert_eq!(
        lines(&report),
        [
            "violation: store (adapters/driven) -> app (core/application) [build]: \
          adapters/driven may depend on core/ports, core/wire"
        ]
    );
}

#[test]
fn an_adapter_depending_on_an_adapter_is_a_violation_but_not_onto_presentation() {
    let ok = run(TABLE, with(base(), "cli", "presentation", Kind::Normal));
    assert_eq!(lines(&ok), Vec::<String>::new());
    let bad = run(TABLE, with(base(), "cli", "store", Kind::Normal));
    assert_eq!(bad.violations(), 1);
    // A driven adapter may not reach the presentation crates.
    let bad = run(TABLE, with(base(), "store", "presentation", Kind::Normal));
    assert_eq!(bad.violations(), 1);
}

#[test]
fn nothing_but_the_root_depends_on_the_root() {
    let report = run(TABLE, with(base(), "app", "bin", Kind::Normal));
    assert_eq!(report.violations(), 1);
    assert!(lines(&report)[0].contains("-> bin (root/bin)"));
}

#[test]
fn a_dev_edge_of_a_checked_role_is_a_warning() {
    let report = run(TABLE, with(base(), "cli", "ports", Kind::Dev));
    assert_eq!(
        lines(&report),
        [
            "warning: cli (adapters/driving) -> ports (core/ports) [dev]: \
          adapters/driving may depend on core/application, @presentation"
        ]
    );
    assert_eq!(report.violations(), 0);
    assert_eq!(report.warnings(), 1);
}

#[test]
fn a_dev_edge_of_an_unchecked_role_is_not_examined() {
    let report = run(TABLE, with(base(), "domain", "store", Kind::Dev));
    assert_eq!(lines(&report), Vec::<String>::new());
    // It is still an edge between members.
    assert_eq!(report.edges, 9);
}

#[test]
fn an_edge_to_a_crate_outside_the_workspace_is_not_an_edge() {
    let report = run(TABLE, with(base(), "domain", "serde", Kind::Normal));
    assert_eq!(lines(&report), Vec::<String>::new());
    assert_eq!(report.edges, 8);
}

#[test]
fn a_package_off_the_layout_is_a_violation_and_its_edges_are_not_judged() {
    let mut packages = base();
    packages.push(pkg("odd", "crates/oddly/odd", &[("store", Kind::Normal)]));
    packages = with(packages, "domain", "odd", Kind::Normal);
    let report = run(TABLE, packages);
    assert_eq!(
        lines(&report),
        ["violation: odd is at crates/oddly/odd, not crates/<ring>/<role>/<crate>"]
    );
}

#[test]
fn a_role_without_a_rule_is_a_violation_and_its_edges_are_not_judged() {
    let mut packages = base();
    packages.push(pkg(
        "mystery",
        "crates/core/mystery/mystery",
        &[("store", Kind::Normal)],
    ));
    packages = with(packages, "domain", "mystery", Kind::Normal);
    let report = run(TABLE, packages);
    assert_eq!(
        lines(&report),
        ["violation: mystery (core/mystery) has no rule in the rules table"]
    );
}

#[test]
fn an_exception_silences_its_edge_and_a_dev_one_its_warning() {
    let table = format!(
        "{TABLE}\n[[exception]]\nfrom = \"cli\"\nto = \"ports\"\nkind = \"dev\"\nreason = \"debt\"\n\n\
         [[exception]]\nfrom = \"domain\"\nto = \"store\"\nreason = \"debt\"\n"
    );
    let packages = with(
        with(base(), "cli", "ports", Kind::Dev),
        "domain",
        "store",
        Kind::Normal,
    );
    let report = run(&table, packages);
    assert_eq!(lines(&report), Vec::<String>::new());
}

#[test]
fn an_exception_of_another_kind_does_not_silence_and_is_stale() {
    let table = format!(
        "{TABLE}\n[[exception]]\nfrom = \"cli\"\nto = \"ports\"\nkind = \"dev\"\nreason = \"debt\"\n"
    );
    let report = run(&table, with(base(), "cli", "ports", Kind::Build));
    assert_eq!(
        lines(&report),
        [
            "violation: cli (adapters/driving) -> ports (core/ports) [build]: \
             adapters/driving may depend on core/application, @presentation",
            "violation: exception cli -> ports [dev] matches no finding",
        ]
    );
}

#[test]
fn an_exception_matching_nothing_is_stale() {
    let table =
        format!("{TABLE}\n[[exception]]\nfrom = \"ghost\"\nto = \"wire\"\nreason = \"debt\"\n");
    let report = run(&table, base());
    assert_eq!(
        lines(&report),
        ["violation: exception ghost -> wire matches no finding"]
    );
}

#[test]
fn an_exception_for_an_allowed_edge_is_stale() {
    let table =
        format!("{TABLE}\n[[exception]]\nfrom = \"domain\"\nto = \"wire\"\nreason = \"debt\"\n");
    let report = run(&table, base());
    assert_eq!(report.violations(), 1);
}

#[test]
fn findings_are_ordered_violations_first_then_by_package_and_target() {
    let mut packages = base();
    packages.push(pkg("odd", "crates/oddly/odd", &[]));
    packages.push(pkg("mystery", "crates/core/mystery/mystery", &[]));
    let packages = with(
        with(
            with(
                with(packages, "domain", "store", Kind::Normal),
                "domain",
                "bin",
                Kind::Normal,
            ),
            "cli",
            "ports",
            Kind::Dev,
        ),
        "app",
        "bin",
        Kind::Normal,
    );
    let table =
        format!("{TABLE}\n[[exception]]\nfrom = \"ghost\"\nto = \"wire\"\nreason = \"debt\"\n");
    let got: Vec<String> = lines(&run(&table, packages))
        .iter()
        .map(|l| {
            l.split(':').next().unwrap().to_string() + ":" + l.split_whitespace().nth(1).unwrap()
        })
        .collect();
    assert_eq!(
        got,
        [
            "violation:odd",
            "violation:mystery",
            "violation:app",
            "violation:domain",
            "violation:domain",
            "violation:exception",
            "warning:cli",
        ]
    );
}

#[test]
fn an_edge_with_no_allowed_roles_says_nothing_is_allowed() {
    let table = "[roles.\"core/wire\"]\nallow = []\n\n[roles.\"core/domain\"]\nallow = []\n";
    let packages = vec![
        pkg("wire", "crates/core/wire/wire", &[]),
        pkg(
            "domain",
            "crates/core/domain/domain",
            &[("wire", Kind::Normal)],
        ),
    ];
    let report = run(table, packages);
    assert_eq!(
        lines(&report),
        ["violation: domain (core/domain) -> wire (core/wire) [normal]: core/domain may depend on nothing"]
    );
}

#[test]
fn the_layout_root_comes_from_the_rules() {
    let table = "layout = \"libs\"\n[roles.\"core/wire\"]\nallow = []\n";
    let report = run(table, vec![pkg("wire", "libs/core/wire/wire", &[])]);
    assert_eq!(lines(&report), Vec::<String>::new());
    let report = run(table, vec![pkg("wire", "crates/core/wire/wire", &[])]);
    assert_eq!(
        lines(&report),
        ["violation: wire is at crates/core/wire/wire, not libs/<ring>/<role>/<crate>"]
    );
}
