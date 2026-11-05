use super::*;

const MINIMAL: &str = r#"
[roles."core/domain"]
allow = ["core/wire", "core/domain"]
rule = "a domain never depends on a read model"

[roles."core/wire"]
allow = []
"#;

#[test]
fn glob_matches_literally_without_a_star() {
    assert!(glob("core/wire", "core/wire"));
    assert!(!glob("core/wire", "core/wires"));
    assert!(!glob("core/wire", "core/wir"));
    assert!(glob("", ""));
    assert!(!glob("", "x"));
    assert!(!glob("x", ""));
}

#[test]
fn glob_star_matches_any_run_including_slashes_and_nothing() {
    assert!(glob("*", ""));
    assert!(glob("*", "anything/at/all"));
    assert!(glob("core/*", "core/domain"));
    assert!(glob("core/*", "core/"));
    assert!(!glob("core/*", "core"));
    assert!(!glob("core/*", "adapters/core/domain"));
    assert!(glob("*-presentation", "demo-presentation"));
    assert!(glob("*-presentation", "-presentation"));
    assert!(!glob("*-presentation", "presentation"));
    assert!(!glob("*-presentation", "demo-presentations"));
    assert!(glob("a*c", "abc"));
    assert!(glob("a*c", "ac"));
    assert!(!glob("a*c", "ab"));
    assert!(glob("a*b*c", "aXbYc"));
    assert!(glob("a*b*c", "abbc"));
    assert!(!glob("a*b*c", "aXcYb"));
    assert!(glob("**", "x"));
}

#[test]
fn glob_backtracks_over_a_repeated_literal() {
    assert!(glob("*ab", "aab"));
    assert!(glob("*ab", "abab"));
    assert!(!glob("*ab", "aba"));
    assert!(glob("a*a", "aaa"));
}

#[test]
fn parses_a_minimal_table_with_defaults() {
    let rules = Rules::parse(MINIMAL).unwrap();
    assert_eq!(rules.layout(), "crates");
    assert!(rules.has_role("core/domain"));
    assert!(!rules.has_role("core/mystery"));
    assert_eq!(
        rules.allowed_roles("core/domain"),
        ["core/wire", "core/domain"]
    );
    assert_eq!(
        rules.rule_text("core/domain"),
        Some("a domain never depends on a read model")
    );
    assert_eq!(rules.rule_text("core/wire"), None);
    assert!(!rules.checks_dev("core/domain"));
    assert!(rules.exceptions().is_empty());
}

#[test]
fn an_unknown_role_has_nothing_allowed() {
    let rules = Rules::parse(MINIMAL).unwrap();
    assert!(rules.allowed_roles("nowhere/at-all").is_empty());
    assert_eq!(rules.rule_text("nowhere/at-all"), None);
    assert!(!rules.allows("nowhere/at-all", "x", "core/wire"));
}

#[test]
fn allows_by_role_by_glob_and_by_class() {
    let text = r#"
[classes]
presentation = ["presentation", "*-presentation"]

[roles."adapters/driving"]
allow = ["core/application", "@presentation"]

[roles."core/application"]
allow = ["core/*"]

[roles."root/bin"]
allow = ["*"]
"#;
    let rules = Rules::parse(text).unwrap();
    assert!(rules.allows("adapters/driving", "a", "core/application"));
    assert!(rules.allows("adapters/driving", "presentation", "adapters/driving"));
    assert!(rules.allows("adapters/driving", "demo-presentation", "adapters/driving"));
    assert!(!rules.allows("adapters/driving", "demo-cli", "adapters/driving"));
    assert!(!rules.allows("adapters/driving", "a", "core/ports"));
    assert!(rules.allows("core/application", "a", "core/ports"));
    assert!(!rules.allows("core/application", "a", "adapters/driven"));
    assert!(rules.allows("root/bin", "a", "adapters/driven"));
    // A class rule is by the target's name, whatever its role.
    assert!(!rules.allows("core/application", "demo-presentation", "adapters/driving"));
}

#[test]
fn parses_dev_checked_and_exceptions() {
    let text = r#"
dev_checked = ["core/domain"]
layout = "libs"

[roles."core/domain"]
allow = []

[[exception]]
from = "a"
to = "b"
kind = "dev"
reason = "known debt"

[[exception]]
from = "c"
to = "d"
reason = "any kind"
"#;
    let rules = Rules::parse(text).unwrap();
    assert_eq!(rules.layout(), "libs");
    assert!(rules.checks_dev("core/domain"));
    let ex = rules.exceptions();
    assert_eq!(ex.len(), 2);
    assert_eq!((ex[0].from.as_str(), ex[0].to.as_str()), ("a", "b"));
    assert_eq!(ex[0].kind, Some(Kind::Dev));
    assert_eq!(ex[1].kind, None);
    assert_eq!(ex[1].reason, "any kind");
}

#[test]
fn a_syntax_error_is_refused() {
    let err = Rules::parse("[roles").unwrap_err();
    assert!(matches!(err, RulesError::Syntax(_)), "{err}");
}

#[test]
fn an_unknown_field_is_refused() {
    let err = Rules::parse(&format!("{MINIMAL}\nlayuot = \"x\"\n")).unwrap_err();
    assert!(matches!(err, RulesError::Syntax(_)), "{err}");
}

#[test]
fn an_exception_without_a_reason_is_refused() {
    for reason in ["", "   "] {
        let text =
            format!("{MINIMAL}\n[[exception]]\nfrom = \"a\"\nto = \"b\"\nreason = \"{reason}\"\n");
        let err = Rules::parse(&text).unwrap_err();
        assert_eq!(err.to_string(), "the exception a -> b has no reason");
    }
}

#[test]
fn an_allow_naming_an_unknown_class_is_refused() {
    let text = "[roles.\"core/domain\"]\nallow = [\"@nothing\"]\n";
    let err = Rules::parse(text).unwrap_err();
    assert_eq!(
        err.to_string(),
        "the role core/domain allows the class @nothing, which is not under [classes]"
    );
}

#[test]
fn dev_checked_naming_an_unknown_role_is_refused() {
    let text = format!("dev_checked = [\"core/mystery\"]\n{MINIMAL}");
    let err = Rules::parse(&text).unwrap_err();
    assert_eq!(
        err.to_string(),
        "dev_checked names the role core/mystery, which has no rule"
    );
}
