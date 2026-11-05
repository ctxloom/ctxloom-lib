use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

use os::Flavor;

use super::EnvSnapshot;

fn unix(vars: &[(&str, &str)]) -> EnvSnapshot {
    EnvSnapshot::of(vars.iter().copied()).with_flavor(Flavor::Unix)
}

fn windows(vars: &[(&str, &str)]) -> EnvSnapshot {
    EnvSnapshot::of(vars.iter().copied()).with_flavor(Flavor::Windows)
}

#[test]
fn test_a_variable_is_found_by_name_with_its_value_as_given() {
    let env = unix(&[("A", "1"), ("B", "")]);
    assert_eq!(env.var("A"), Some(OsStr::new("1")));
    assert_eq!(env.var("B"), Some(OsStr::new("")));
    assert_eq!(env.var("C"), None);
    assert_eq!(env.var("a"), None);
}

#[test]
fn test_the_first_of_two_variables_of_one_name_is_the_one_found() {
    assert_eq!(
        unix(&[("A", "first"), ("A", "second")]).var("A"),
        Some(OsStr::new("first"))
    );
}

#[test]
fn test_windows_compares_names_without_case() {
    let env = windows(&[("Path", "x")]);
    assert_eq!(env.var("PATH"), Some(OsStr::new("x")));
    assert_eq!(env.var("path"), Some(OsStr::new("x")));
    assert_eq!(env.var("PATHS"), None);
    assert_eq!(unix(&[("Path", "x")]).var("PATH"), None);
}

#[test]
fn test_a_name_that_is_not_utf_8_is_never_the_one_found() {
    let env = EnvSnapshot::of([(os::not_unicode("A"), OsString::from("v"))]);
    assert_eq!(env.var("A"), None);
    assert_eq!(env.vars().count(), 1);
}

#[test]
fn test_non_empty_refuses_the_unset_and_the_empty() {
    let env = unix(&[("A", "1"), ("B", "")]);
    assert_eq!(env.non_empty("A"), Some(OsStr::new("1")));
    assert_eq!(env.non_empty("B"), None);
    assert_eq!(env.non_empty("C"), None);
}

#[test]
fn test_utf8_is_text_or_nothing() {
    let env = EnvSnapshot::of([
        (OsString::from("A"), OsString::from("1")),
        (OsString::from("E"), OsString::new()),
        (OsString::from("N"), os::not_unicode("v")),
    ]);
    assert_eq!(env.utf8("A"), Some("1"));
    assert_eq!(env.utf8("E"), Some(""));
    assert_eq!(env.utf8("N"), None);
    assert_eq!(env.utf8("U"), None);
    assert!(env.var("N").is_some(), "the value is kept, not dropped");
}

#[test]
fn test_the_home_is_the_flavors_variable_unless_empty() {
    assert_eq!(
        unix(&[("HOME", "/h"), ("USERPROFILE", "C:\\u")]).home(),
        Some(OsStr::new("/h"))
    );
    assert_eq!(
        windows(&[("HOME", "/h"), ("USERPROFILE", "C:\\u")]).home(),
        Some(OsStr::new("C:\\u"))
    );
    assert_eq!(unix(&[("HOME", "")]).home(), None);
    assert_eq!(unix(&[]).home(), None);
}

#[test]
fn test_the_search_path_is_split_as_the_flavor_splits_it() {
    assert_eq!(
        unix(&[("PATH", "/a::/b")]).search_path(),
        vec![PathBuf::from("/a"), PathBuf::from("/b")]
    );
    assert_eq!(
        windows(&[("PATH", "C:\\a;\"C:\\b;c\";;D:\\d")]).search_path(),
        vec![
            PathBuf::from("C:\\a"),
            PathBuf::from("C:\\b;c"),
            PathBuf::from("D:\\d")
        ]
    );
    assert_eq!(unix(&[]).search_path(), Vec::<PathBuf>::new());
}

#[test]
fn test_a_search_path_that_is_not_utf_8_keeps_every_directory() {
    let mut path = OsString::from("/a:");
    path.push(os::not_unicode("/b"));
    let env = EnvSnapshot::of([(OsString::from("PATH"), path)]).with_flavor(Flavor::Unix);
    assert_eq!(env.search_path().len(), 2);
}

#[test]
fn test_vars_are_every_pair_in_order_and_vars_utf8_skips_what_is_not_text() {
    let env = EnvSnapshot::of([
        (OsString::from("A"), OsString::from("1")),
        (OsString::from("B"), os::not_unicode("v")),
        (os::not_unicode("C"), OsString::from("3")),
        (OsString::from("D"), OsString::from("4")),
    ]);
    let names: Vec<&OsStr> = env.vars().map(|(name, _)| name).collect();
    assert_eq!(names.len(), 4);
    assert_eq!(names[0], OsStr::new("A"));
    assert_eq!(
        env.vars_utf8(),
        vec![
            ("A".to_string(), "1".to_string()),
            ("D".to_string(), "4".to_string())
        ]
    );
}

#[test]
fn test_the_working_and_temporary_directories_are_the_ones_given() {
    let env = unix(&[]).with_cwd("/work").with_temp_dir("/scratch");
    assert_eq!(env.cwd().unwrap(), Path::new("/work"));
    assert_eq!(env.temp_dir(), Path::new("/scratch"));
    assert_eq!(unix(&[]).cwd().unwrap(), Path::new("."));
    assert_eq!(unix(&[]).temp_dir(), Path::new("tmp"));
}

#[test]
fn test_a_working_directory_that_could_not_be_read_keeps_its_error() {
    let env = unix(&[("A", "1")]).without_cwd(io::Error::from_raw_os_error(2));
    let first = env.cwd().unwrap_err();
    assert_eq!(first.raw_os_error(), Some(2));
    assert_eq!(first.kind(), io::ErrorKind::NotFound);
    assert_eq!(
        env.cwd().unwrap_err().to_string(),
        io::Error::from_raw_os_error(2).to_string()
    );
    assert_eq!(env.utf8("A"), Some("1"), "the rest is kept");

    let custom = unix(&[]).without_cwd(io::Error::new(io::ErrorKind::InvalidInput, "odd"));
    let err = custom.cwd().unwrap_err();
    assert_eq!(
        (err.kind(), err.to_string()),
        (io::ErrorKind::InvalidInput, "odd".to_string())
    );
}

#[test]
fn test_the_flavor_is_the_one_given() {
    assert_eq!(unix(&[]).flavor(), Flavor::Unix);
    assert_eq!(windows(&[]).flavor(), Flavor::Windows);
    assert_eq!(EnvSnapshot::of([("A", "b")]).flavor(), os::host().flavor());
}

/// The one test that reads the process: the real variables, working
/// directory and temporary directory, the host's flavor.
#[test]
#[allow(clippy::disallowed_methods)]
fn test_from_process_holds_what_the_process_has() {
    const NAME: &str = "PROCESS_ENV_TEST_VARIABLE";
    std::env::set_var(NAME, "kept");
    std::env::set_var("PROCESS_ENV_TEST_EMPTY", "");
    let env = EnvSnapshot::from_process();
    std::env::remove_var(NAME);
    std::env::remove_var("PROCESS_ENV_TEST_EMPTY");
    assert_eq!(env.utf8(NAME), Some("kept"));
    assert_eq!(env.utf8("PROCESS_ENV_TEST_EMPTY"), Some(""));
    assert_eq!(env.cwd().unwrap(), std::env::current_dir().unwrap());
    assert_eq!(env.temp_dir(), std::env::temp_dir());
    assert_eq!(env.flavor(), os::host().flavor());
    assert!(env.vars().count() >= 2);
    // A snapshot is a snapshot: the variable is gone from the process now.
    assert_eq!(env.utf8(NAME), Some("kept"));
    assert_eq!(EnvSnapshot::from_process().var(NAME), None);
}
