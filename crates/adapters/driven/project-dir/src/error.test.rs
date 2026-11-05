use angzarr_diagnostic::{Arg, Diagnostic};

use super::{diagnostic_samples, ProjectDirError, EN_US};

/// The catalog is exported under the crate's package name, and every code
/// is under that name, with its message using exactly its args.
#[test]
fn every_code_is_under_the_crates_name_with_its_message() {
    let (name, text) = EN_US;
    assert_eq!(name, "project-dir");
    assert_eq!(
        angzarr_diagnostic::catalog::check(name, text, &diagnostic_samples()),
        []
    );
}

#[test]
fn every_variant_has_its_own_code() {
    let mut codes: Vec<String> = diagnostic_samples()
        .iter()
        .map(|d| d.code().to_string())
        .collect();
    let all = codes.len();
    codes.sort();
    codes.dedup();
    assert_eq!(codes.len(), all);
    assert_eq!(all, 5);
}

#[test]
fn the_codes_and_args_are_stable() {
    let id = |s: &str| s.to_string();
    let cases = [
        (ProjectDirError::IdEmpty, "project-dir-id-empty", None),
        (
            ProjectDirError::IdNul { id: id("a\0") },
            "project-dir-id-nul",
            Some("a\0"),
        ),
        (
            ProjectDirError::IdAbsolute { id: id("/a") },
            "project-dir-id-absolute",
            Some("/a"),
        ),
        (
            ProjectDirError::IdSeparator { id: id("a/b") },
            "project-dir-id-separator",
            Some("a/b"),
        ),
        (
            ProjectDirError::IdRelative { id: id("..") },
            "project-dir-id-relative",
            Some(".."),
        ),
    ];
    for (error, code, arg) in cases {
        assert_eq!(error.code(), code);
        assert_eq!(error.static_code(), code);
        assert_eq!(error.id(), arg);
        let args = error.args();
        match arg {
            None => assert!(args.is_empty()),
            Some(want) => {
                assert_eq!(args.len(), 1);
                assert_eq!(args[0].0, "id");
                assert!(matches!(&args[0].1, Arg::Str(s) if s == want), "{args:?}");
            }
        }
    }
}
