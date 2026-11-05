use super::*;

#[test]
fn a_crate_directory_is_ring_role_crate_under_the_layout() {
    assert_eq!(
        role_of("crates", "crates/core/domain/demo"),
        Some("core/domain".to_string())
    );
    assert_eq!(
        role_of("crates", "crates/root/bin/demo-tool"),
        Some("root/bin".to_string())
    );
}

#[test]
fn any_other_depth_is_not_placed() {
    assert_eq!(role_of("crates", "crates/oddly/demo"), None);
    assert_eq!(role_of("crates", "crates/core/domain"), None);
    assert_eq!(role_of("crates", "crates/core/domain/demo/deeper"), None);
    assert_eq!(role_of("crates", "demo"), None);
    assert_eq!(role_of("crates", ""), None);
}

#[test]
fn the_layout_root_must_be_the_configured_one() {
    assert_eq!(role_of("crates", "libs/core/domain/demo"), None);
    assert_eq!(
        role_of("libs", "libs/core/domain/demo"),
        Some("core/domain".to_string())
    );
    assert_eq!(role_of("libs", "crates/core/domain/demo"), None);
}

#[test]
fn an_empty_component_is_not_placed() {
    assert_eq!(role_of("crates", "crates//domain/demo"), None);
    assert_eq!(role_of("crates", "crates/core//demo"), None);
    assert_eq!(role_of("crates", "crates/core/domain/"), None);
}

#[test]
fn a_kind_is_spelled_as_the_rules_spell_it() {
    assert_eq!(Kind::Normal.to_string(), "normal");
    assert_eq!(Kind::Build.to_string(), "build");
    assert_eq!(Kind::Dev.to_string(), "dev");
}
