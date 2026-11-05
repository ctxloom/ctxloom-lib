use super::{diagnostic_samples, EN_US};

/// The catalog is exported under the crate's package name, and every code
/// is under that name, with its message using exactly its args.
#[test]
fn every_code_is_under_the_crates_name_with_its_message() {
    let (name, text) = EN_US;
    assert_eq!(name, "duration-text");
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
    assert_eq!(all, 11);
}
