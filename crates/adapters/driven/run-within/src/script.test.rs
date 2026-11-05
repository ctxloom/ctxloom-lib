use file_system::{Fs, Rooted};
use os::Mode;

use super::install_script;

#[test]
fn the_script_is_written_whole_executable_and_found_on_the_host() {
    let fs = Rooted::temp().unwrap();
    let dir = os::fixture("/bin");
    fs.create_dir_all(&dir, Mode::DIR).unwrap();
    let path = install_script(&fs, &dir, "demo", "#!/bin/sh\nexit 0\n").unwrap();
    let seen = dir.join("demo");
    assert_eq!(fs.read_to_string(&seen).unwrap(), "#!/bin/sh\nexit 0\n");
    assert!(fs.metadata(&seen).unwrap().executable);
    assert!(path.ends_with("demo"));
    assert_eq!(fs.real_path(&seen).unwrap(), path);
}

#[test]
fn a_missing_directory_is_the_filesystems_error() {
    let fs = Rooted::temp().unwrap();
    let error = install_script(&fs, &os::fixture("/nope"), "demo", "x").unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}
