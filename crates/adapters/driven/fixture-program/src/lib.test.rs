//! The race is forced, not waited for: a child is given the descriptor a
//! file was written through (where a thread's fork holds it between fork and
//! exec), so the file is busy for exactly as long as the test says.

use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use file_system::{Fs, Open, Perms, Rooted};
use os::{Flavor, Mode};

use super::{beside, install_program, shim_ready, SHIM};

fn runs_scripts() -> bool {
    os::host().flavor() == Flavor::Unix
}

fn machine() -> (Rooted, std::path::PathBuf) {
    let fs = Rooted::temp().unwrap();
    let dir = os::fixture("/bin");
    fs.create_dir_all(&dir, Mode::DIR).unwrap();
    (fs, dir)
}

fn host() -> file_system::Os {
    super::host_fs().unwrap()
}

/// A child that holds `file` of `fs` open for writing until its stdin closes.
fn holder(fs: &dyn Fs, file: &Path) -> Child {
    let writer = fs.open(file, Open::Create(Mode::FILE)).unwrap();
    // The parent's writer is moved into the child's stdout; once this
    // returns only the child holds the file for writing.
    Command::new("/bin/sh")
        .args(["-c", "read line"])
        .stdin(Stdio::piped())
        .stdout(writer)
        .spawn()
        .unwrap()
}

fn stdout_of(program: &Path) -> String {
    let out = Command::new(program).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_program_runs_its_script_with_the_arguments_and_the_scripts_own_directory() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let body =
        "#!/bin/sh\nprintf '%s|%s|%s\\n' \"$#\" \"$1\" \"$(basename \"$(dirname \"$0\")\")\"\n";
    let program = install_program(&fs, &dir.join("demo"), body).unwrap();
    let out = Command::new(&program).args(["a b", "c"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "2|a b|bin\n");
}

#[test]
fn the_program_is_the_hosts_path_a_link_to_the_shim_with_a_body_that_is_not_executable() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let program = install_program(&fs, &dir.join("demo"), "exit 0\n").unwrap();
    assert_eq!(program, fs.real_path(&dir.join("demo")).unwrap());
    let host = host();
    assert_eq!(host.read_link(&program).unwrap(), Path::new(SHIM));
    assert!(host.symlink_metadata(&program).unwrap().kind == file_system::Kind::Symlink);
    let body = fs.metadata(&beside(&dir.join("demo"))).unwrap();
    assert!(!body.executable, "the body is read by sh, never exec'd");
    assert_eq!(
        fs.read_to_string(&beside(&dir.join("demo"))).unwrap(),
        "exit 0\n"
    );
}

#[test]
fn the_shim_is_committed_executable() {
    if !runs_scripts() {
        return;
    }
    assert!(host().metadata(Path::new(SHIM)).unwrap().executable);
}

#[test]
fn a_shim_without_the_execute_bit_is_refused_by_name() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let plain = dir.join("plain");
    fs.write_atomic(&plain, b"x", Perms::Exact(Mode::FILE))
        .unwrap();
    let real = fs.real_path(&plain).unwrap();
    let error = shim_ready(&host(), &real).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(error.to_string().contains("plain"), "{error}");
    fs.set_permissions(&plain, Mode::from_bits(0o700)).unwrap();
    shim_ready(&host(), &real).unwrap();
}

#[test]
fn installing_again_replaces_the_script() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let at = dir.join("demo");
    let first = install_program(&fs, &at, "echo one\n").unwrap();
    assert_eq!(stdout_of(&first), "one\n");
    let second = install_program(&fs, &at, "echo two\n").unwrap();
    assert_eq!(first, second);
    assert_eq!(stdout_of(&second), "two\n");
}

#[test]
fn a_file_already_at_the_path_is_replaced_by_the_program() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let at = dir.join("demo");
    fs.write_atomic(&at, b"not a program", Perms::Exact(Mode::FILE))
        .unwrap();
    let program = install_program(&fs, &at, "echo ran\n").unwrap();
    assert_eq!(stdout_of(&program), "ran\n");
}

#[test]
fn it_installs_through_the_hosts_own_filesystem_too() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let real_dir = fs.real_path(&dir).unwrap();
    let host = host();
    let program = install_program(&host, &real_dir.join("onhost"), "echo ran\n").unwrap();
    assert_eq!(program, real_dir.join("onhost"));
    assert_eq!(stdout_of(&program), "ran\n");
}

#[test]
fn a_missing_directory_is_the_filesystems_error() {
    if !runs_scripts() {
        return;
    }
    let (fs, _) = machine();
    let error = install_program(&fs, &os::fixture("/nope/demo"), "x").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

/// The control: without this crate the fixture fails exactly as the race
/// makes it fail, so the next test proves something.
#[test]
fn a_script_a_child_still_holds_for_writing_cannot_be_exec_d() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let at = dir.join("racy");
    fs.write_atomic(
        &at,
        b"#!/bin/sh\necho ran\n",
        Perms::Exact(Mode::from_bits(0o755)),
    )
    .unwrap();
    let mut held = holder(&fs, &at);
    let error = Command::new(fs.real_path(&at).unwrap())
        .output()
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::ExecutableFileBusy, "{error}");
    drop(held.stdin.take());
    held.wait().unwrap();
}

#[test]
fn a_program_runs_while_a_child_holds_its_script_for_writing() {
    if !runs_scripts() {
        return;
    }
    let (fs, dir) = machine();
    let at = dir.join("racy");
    let program = install_program(&fs, &at, "echo ran\n").unwrap();
    let mut held = holder(&fs, &beside(&at));
    assert_eq!(stdout_of(&program), "ran\n");
    drop(held.stdin.take());
    held.wait().unwrap();
    assert_eq!(stdout_of(&program), "ran\n", "and when the holder is gone");
}

#[test]
fn a_program_runs_while_a_child_holds_the_program_path_for_writing_through_its_old_script() {
    if !runs_scripts() {
        return;
    }
    // Reinstalling over a program whose script a child holds: the new script
    // is a new file, the shim untouched, and the program still runs.
    let (fs, dir) = machine();
    let at = dir.join("racy");
    install_program(&fs, &at, "echo old\n").unwrap();
    let mut held = holder(&fs, &beside(&at));
    let program = install_program(&fs, &at, "echo new\n").unwrap();
    assert_eq!(stdout_of(&program), "new\n");
    drop(held.stdin.take());
    held.wait().unwrap();
}
