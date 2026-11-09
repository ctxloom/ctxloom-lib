//! A terminal run inherits the caller's stdio and group and ends at exit or
//! stop. No test attaches a real tty (a test process has none to give), so
//! what is proved is what the run itself does: the child is handed the
//! caller's descriptors, and is not moved to a group of its own. The stop is
//! forced (requested before the call, or from the `started` callback, when the
//! program is known to be running), never waited for.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use file_system::{Fs, Rooted};
use os::{Flavor, Mode};

use super::*;
use crate::install_script;

const PROMPT: Duration = Duration::from_secs(10);

fn runs_scripts() -> bool {
    os::host().flavor() == Flavor::Unix
}

struct Machine {
    fs: Rooted,
    dir: PathBuf,
}

impl Machine {
    fn new() -> Machine {
        let fs = Rooted::temp().unwrap();
        let dir = os::fixture("/bin");
        fs.create_dir_all(&dir, Mode::DIR).unwrap();
        Machine { fs, dir }
    }

    fn spec(&self, body: &str) -> Spec {
        let path =
            install_script(&self.fs, &self.dir, "prog", &format!("#!/bin/sh\n{body}\n")).unwrap();
        Spec::new("/bin/sh")
            .args([path])
            .envs([("PATH", "/usr/bin:/bin")])
    }

    fn note(&self, name: &str) -> PathBuf {
        self.fs.real_path(&self.dir).unwrap().join(name)
    }

    fn read(&self, name: &str) -> String {
        self.fs
            .read_to_string(&self.dir.join(name))
            .unwrap_or_default()
    }
}

#[test]
fn the_exit_status_is_reported_and_the_run_is_not_stopped() {
    if !runs_scripts() {
        return;
    }
    let out = run_terminal(&Machine::new().spec("exit 3"), &Stop::new(), |_| {}).unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(!out.stopped);
}

#[test]
fn started_is_called_once_with_the_childs_own_pid() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let spec = machine.spec(&format!("echo $$ > {}", machine.note("pid").display()));
    let mut seen = Vec::new();
    run_terminal(&spec, &Stop::new(), |pid| seen.push(pid)).unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(machine.read("pid").trim(), seen[0].to_string());
}

#[test]
fn a_program_that_cannot_start_is_the_error_and_started_is_not_called() {
    let mut called = false;
    let error = run_terminal(&Spec::new("/no/such/program"), &Stop::new(), |_| {
        called = true
    })
    .unwrap_err();
    assert!(matches!(error, RunError::NotFound { .. }), "{error}");
    assert!(!called);
}

#[test]
fn the_child_stays_in_the_callers_process_group() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    // The child's parent is the test process, spawned directly.
    let spec = machine.spec(&format!(
        "echo $(ps -o pgid= -p $$) $(ps -o pgid= -p $PPID) > {}",
        machine.note("groups").display()
    ));
    run_terminal(&spec, &Stop::new(), |_| {}).unwrap();
    let groups = machine.read("groups");
    let groups: Vec<&str> = groups.split_whitespace().collect();
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert_eq!(groups[0], groups[1]);
    // And the contrast that gives the test its meaning: the other modes put
    // the child in a group of its own.
    let own = machine.spec(&format!(
        "echo $(ps -o pgid= -p $$) $(ps -o pgid= -p $PPID) > {}",
        machine.note("own").display()
    ));
    crate::run_within(&own, &crate::Within::new(PROMPT)).unwrap();
    let own = machine.read("own");
    let own: Vec<&str> = own.split_whitespace().collect();
    assert_ne!(own[0], own[1]);
}

#[test]
fn a_stop_requested_before_the_run_kills_the_child_after_it_started() {
    if !runs_scripts() {
        return;
    }
    let stop = Stop::new();
    stop.request();
    let mut started = 0;
    let began = Instant::now();
    let out = run_terminal(&Machine::new().spec("exec sleep 30"), &stop, |_| {
        started += 1
    })
    .unwrap();
    assert!(out.stopped);
    assert_eq!(out.status.code(), None, "killed by a signal");
    assert_eq!(started, 1);
    assert!(began.elapsed() < PROMPT);
}

#[test]
fn a_stop_requested_once_the_child_is_known_to_run_ends_it() {
    if !runs_scripts() {
        return;
    }
    let stop = Stop::new();
    let from_callback = stop.clone();
    // The request is made from `started`, when the child is known to run.
    let out = run_terminal(&Machine::new().spec("exec sleep 30"), &stop, move |_| {
        from_callback.request()
    })
    .unwrap();
    assert!(out.stopped);
}

#[test]
fn a_child_that_exits_by_itself_is_not_stopped_by_a_stop_after() {
    if !runs_scripts() {
        return;
    }
    let stop = Stop::new();
    let out = run_terminal(&Machine::new().spec("exit 0"), &stop, |_| {}).unwrap();
    stop.request();
    assert!(!out.stopped);
    assert!(stop.is_requested());
}

#[test]
fn clones_of_a_stop_share_the_request() {
    let stop = Stop::new();
    let other = stop.clone();
    assert!(!other.is_requested());
    stop.request();
    assert!(other.is_requested());
}

// --- the system's calls failing

struct Fake {
    asks: Vec<io::Result<Option<ExitStatus>>>,
    kill: Option<io::Error>,
    wait: Option<io::Error>,
    killed: bool,
}

impl Running for Fake {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.asks.is_empty() {
            Ok(None)
        } else {
            self.asks.remove(0)
        }
    }

    fn kill(&mut self) -> io::Result<()> {
        self.killed = true;
        self.kill.take().map_or(Ok(()), Err)
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        match self.wait.take() {
            Some(e) => Err(e),
            None => Ok(exit_status()),
        }
    }
}

fn exit_status() -> ExitStatus {
    std::process::Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .status()
        .unwrap()
}

fn fake(asks: Vec<io::Result<Option<ExitStatus>>>) -> Fake {
    Fake {
        asks,
        kill: None,
        wait: None,
        killed: false,
    }
}

#[test]
fn a_failed_ask_kills_the_child_and_says_so() {
    if !runs_scripts() {
        return;
    }
    let mut f = fake(vec![Err(io::Error::other("lost"))]);
    let error = wait(&mut f, &Spec::new("demo"), &Stop::new()).unwrap_err();
    assert!(matches!(error, RunError::Wait { .. }), "{error}");
    assert!(f.killed);
}

#[test]
fn a_failed_kill_at_the_stop_is_an_error_not_a_stop() {
    if !runs_scripts() {
        return;
    }
    let mut f = fake(vec![]);
    f.kill = Some(io::Error::other("denied"));
    let stop = Stop::new();
    stop.request();
    let error = wait(&mut f, &Spec::new("demo"), &stop).unwrap_err();
    assert!(matches!(error, RunError::Kill { .. }), "{error}");
}

#[test]
fn a_failed_reap_after_the_kill_is_a_wait_error() {
    if !runs_scripts() {
        return;
    }
    let mut f = fake(vec![]);
    f.wait = Some(io::Error::other("lost"));
    let stop = Stop::new();
    stop.request();
    let error = wait(&mut f, &Spec::new("demo"), &stop).unwrap_err();
    assert!(matches!(error, RunError::Wait { .. }), "{error}");
}

#[test]
fn a_child_still_running_is_asked_again_until_it_is_over() {
    if !runs_scripts() {
        return;
    }
    let status = exit_status();
    let mut f = fake(vec![Ok(None), Ok(None), Ok(Some(status))]);
    let out = wait(&mut f, &Spec::new("demo"), &Stop::new()).unwrap();
    assert_eq!(out.status, status);
    assert!(!f.killed);
}

// --- inherited stdio, proved in a process whose stdio the test owns

/// The driver: ignored, so only the parent test runs it, by re-running this
/// binary with `--ignored`; it runs a terminal child whose stdin, stdout and
/// stderr are the pipes the parent gave.
#[test]
#[ignore = "run by the parent test, which owns its stdio"]
fn stdio_driver() {
    let machine = Machine::new();
    let spec = machine.spec("echo out; echo err >&2; read line; echo got:$line");
    let out = run_terminal(&spec, &Stop::new(), |_| {}).unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn the_child_reads_the_callers_stdin_and_writes_the_callers_stdout_and_stderr() {
    if !runs_scripts() {
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let mut driver = std::process::Command::new(exe)
        .args([
            "--ignored",
            "--exact",
            "terminal::tests::stdio_driver",
            "--nocapture",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    driver.stdin.take().unwrap().write_all(b"hello\n").unwrap();
    let out = driver.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stdout}{stderr}");
    assert!(stdout.contains("out\n"), "{stdout}");
    assert!(stdout.contains("got:hello\n"), "{stdout}");
    assert!(stderr.contains("err\n"), "{stderr}");
}
