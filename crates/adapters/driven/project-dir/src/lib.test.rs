//! Characterization of the root walk and the project path, moved from a
//! consumer's app-dir and store-path tests, with the apps' names made up
//! (`.demo`).

use std::path::{Path, PathBuf};

use angzarr_diagnostic::Diagnostic;
use file_system::{Perms, Rooted};
use os::Mode;

use super::*;

fn rooted() -> Rooted {
    Rooted::temp().expect("a temporary root")
}

fn dir(fs: &Rooted, path: &str) {
    fs.create_dir_all(Path::new(path), Mode::from_bits(0o755))
        .unwrap();
}

fn file(fs: &Rooted, path: &str) {
    dir(fs, Path::new(path).parent().unwrap().to_str().unwrap());
    fs.write_atomic(Path::new(path), b"x", Perms::Exact(Mode::from_bits(0o644)))
        .unwrap();
}

/// The walk the demo app's way: the nearest `.demo` directory, stopping at
/// `stop`.
fn demo_root(fs: &Rooted, cwd: &str, stop: &str) -> Option<PathBuf> {
    nearest_ancestor(
        Path::new(cwd),
        |d| d == Path::new(stop),
        |d| holds_dir(fs, d, ".demo"),
    )
}

#[test]
fn test_the_nearest_ancestor_with_the_marker_wins_and_the_start_counts() {
    let fs = rooted();
    dir(&fs, "/work/proj/.demo");
    dir(&fs, "/work/proj/sub/.demo");
    dir(&fs, "/work/proj/sub/deeper/x");
    assert_eq!(
        demo_root(&fs, "/work/proj/sub/deeper/x", "/tmp"),
        Some(PathBuf::from("/work/proj/sub"))
    );
    assert_eq!(
        demo_root(&fs, "/work/proj/sub", "/tmp"),
        Some(PathBuf::from("/work/proj/sub"))
    );
    assert_eq!(
        demo_root(&fs, "/work/proj", "/tmp"),
        Some(PathBuf::from("/work/proj"))
    );
}

#[test]
fn test_no_marker_anywhere_finds_none() {
    let fs = rooted();
    dir(&fs, "/work/bare");
    assert_eq!(demo_root(&fs, "/work/bare", "/tmp"), None);
}

#[test]
fn test_the_walk_stops_at_the_stop_directory_itself_and_below_it_only() {
    let fs = rooted();
    dir(&fs, "/tmp/.demo");
    dir(&fs, "/tmp/p/deep");
    dir(&fs, "/tmp/work/bare");
    // Below the stop, the marker above it is out of reach.
    assert_eq!(demo_root(&fs, "/tmp/work/bare", "/tmp"), None);
    assert_eq!(demo_root(&fs, "/tmp/p/deep", "/tmp"), None);
    // The stop directory itself is never a root, though it holds the marker.
    assert_eq!(demo_root(&fs, "/tmp", "/tmp"), None);
    // Not stopping, the same marker is found.
    assert_eq!(
        demo_root(&fs, "/tmp/p/deep", "/nowhere"),
        Some(PathBuf::from("/tmp"))
    );
}

#[test]
fn test_a_stop_is_asked_before_the_rule_and_a_never_stopping_walk_reaches_the_top() {
    let fs = rooted();
    dir(&fs, "/.demo");
    dir(&fs, "/a/b");
    let found = nearest_ancestor(Path::new("/a/b"), |_| false, |d| holds_dir(&fs, d, ".demo"));
    assert_eq!(found, Some(PathBuf::from("/")));
    // A stop that is true at once finds nothing, though the start is a root.
    dir(&fs, "/a/b/.demo");
    assert_eq!(
        nearest_ancestor(Path::new("/a/b"), |_| true, |_| true),
        None
    );
}

#[test]
fn test_the_rule_is_asked_of_each_directory_nearest_first_and_no_more() {
    let asked = std::cell::RefCell::new(Vec::new());
    let found = nearest_ancestor(
        Path::new("/a/b/c"),
        |_| false,
        |d| {
            asked.borrow_mut().push(d.to_path_buf());
            d == Path::new("/a")
        },
    );
    assert_eq!(found, Some(PathBuf::from("/a")));
    assert_eq!(
        *asked.borrow(),
        [
            PathBuf::from("/a/b/c"),
            PathBuf::from("/a/b"),
            PathBuf::from("/a")
        ]
    );
}

#[test]
fn test_holds_dir_counts_a_directory_and_not_a_file_or_nothing() {
    let fs = rooted();
    dir(&fs, "/w/d/.demo");
    file(&fs, "/w/f/.demo");
    assert!(holds_dir(&fs, Path::new("/w/d"), ".demo"));
    assert!(!holds_dir(&fs, Path::new("/w/f"), ".demo"));
    assert!(!holds_dir(&fs, Path::new("/w/none"), ".demo"));
}

#[test]
fn test_holds_entry_counts_a_directory_or_a_file_and_not_nothing() {
    let fs = rooted();
    dir(&fs, "/w/d/.git");
    file(&fs, "/w/f/.git");
    dir(&fs, "/w/none");
    assert!(holds_entry(&fs, Path::new("/w/d"), ".git"));
    assert!(holds_entry(&fs, Path::new("/w/f"), ".git"));
    assert!(!holds_entry(&fs, Path::new("/w/none"), ".git"));
}

#[test]
fn test_a_project_s_directory_is_the_home_the_app_dir_projects_the_id() {
    assert_eq!(
        project_dir(Path::new("/home/u"), ".demo", "jolly-mute-magma").unwrap(),
        PathBuf::from("/home/u/.demo/projects/jolly-mute-magma")
    );
    // A relative home stays relative.
    assert_eq!(
        project_dir(Path::new("rel"), ".demo", "p").unwrap(),
        PathBuf::from("rel/.demo/projects/p")
    );
}

#[test]
fn test_an_unsafe_id_is_refused_with_its_code() {
    let refused = |id: &str| -> String {
        project_dir(Path::new("/h"), ".demo", id)
            .expect_err(id)
            .code()
            .to_string()
    };
    assert_eq!(refused(""), "project-dir-id-empty");
    assert_eq!(refused("a\0b"), "project-dir-id-nul");
    assert_eq!(refused("/etc"), "project-dir-id-absolute");
    assert_eq!(refused("a/b"), "project-dir-id-separator");
    assert_eq!(refused("a\\b"), "project-dir-id-separator");
    assert_eq!(refused("../x"), "project-dir-id-separator");
    assert_eq!(refused("x/.."), "project-dir-id-separator");
    assert_eq!(refused(".."), "project-dir-id-relative");
    assert_eq!(refused("."), "project-dir-id-relative");
}

#[test]
fn test_a_plain_name_is_an_id_even_with_dots_in_it() {
    for id in [
        "p",
        "a..b",
        ".hidden",
        "...",
        "a.b",
        "jolly-mute-magma",
        "a b",
    ] {
        assert_eq!(check_id(id), Ok(()), "{id}");
    }
}

#[test]
fn test_a_refused_id_is_shown_as_given() {
    let error = check_id("a/b").unwrap_err();
    assert_eq!(error, ProjectDirError::IdSeparator { id: "a/b".into() });
}
