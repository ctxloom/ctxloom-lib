use std::path::Path;

use super::*;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
        .join("Cargo.toml")
}

fn package<'a>(graph: &'a Graph, name: &str) -> &'a Package {
    graph.packages.iter().find(|p| p.name == name).unwrap()
}

#[test]
fn loads_every_member_with_its_directory_under_the_workspace_root() {
    let graph = load(&fixture("clean")).unwrap();
    assert_eq!(graph.packages.len(), 11);
    assert_eq!(
        package(&graph, "fx-domain").dir,
        "crates/core/domain/fx-domain"
    );
    assert_eq!(package(&graph, "fx-bin").dir, "crates/root/bin/fx-bin");
}

#[test]
fn loads_the_edges_between_members_by_kind() {
    let graph = load(&fixture("clean")).unwrap();
    let deps: Vec<(&str, Kind)> = package(&graph, "fx-driven-a")
        .deps
        .iter()
        .map(|(n, k)| (n.as_str(), *k))
        .collect();
    assert_eq!(
        deps,
        [
            ("fx-ports", Kind::Normal),
            ("fx-ports", Kind::Dev),
            ("fx-wire", Kind::Normal),
            ("fx-wire", Kind::Build),
        ]
    );
    assert_eq!(package(&graph, "fx-bin").deps.len(), 7);
    assert!(package(&graph, "fx-wire").deps.is_empty());
}

#[test]
fn a_package_off_the_layout_keeps_its_real_directory() {
    let graph = load(&fixture("violations")).unwrap();
    assert_eq!(
        package(&graph, "fx-misplaced").dir,
        "crates/oddly/fx-misplaced"
    );
    assert_eq!(graph.packages.len(), 14);
}

#[test]
fn a_manifest_that_is_not_there_is_an_error() {
    let err = load(&fixture("no-such-workspace")).unwrap_err();
    assert!(err.to_string().contains("no-such-workspace"), "{err}");
}

#[test]
fn only_path_dependencies_on_members_are_edges() {
    // fx-domain also depends on a registry crate and on a path crate outside
    // the workspace.
    let graph = load(&fixture("outside")).unwrap();
    let deps: Vec<&str> = package(&graph, "fx-domain")
        .deps
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(deps, ["fx-wire"]);
}
