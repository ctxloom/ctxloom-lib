//! A run is over when its program is, however it ends; none of it by
//! sleeping in a test: a deadline is a tiny one, a program that must still
//! be running is `sleep 30`, and "promptly" is an assertion that a run did
//! not take as long as that.

use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use file_system::{Fs, Perms, Rooted};
use os::{Flavor, Mode};

use super::*;

/// Far longer than anything here takes, far shorter than a `sleep 30`.
const PROMPT: Duration = Duration::from_secs(10);
const TINY: Duration = Duration::from_millis(30);
const LONG: Duration = Duration::from_secs(60);

fn runs_scripts() -> bool {
    os::host().flavor() == Flavor::Unix
}

/// A directory of the test's own, and a script in it.
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
        Spec::new(path)
    }

    fn real_dir(&self) -> PathBuf {
        self.fs.real_path(&self.dir).unwrap()
    }
}

fn run(body: &str, within: &Within) -> Outcome {
    run_within(&Machine::new().spec(body), within).unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn code(status: Option<ExitStatus>) -> Option<i32> {
    status.and_then(|s| s.code())
}

#[test]
fn output_and_exit_status_come_back_with_the_streams_apart() {
    if !runs_scripts() {
        return;
    }
    let out = run("echo out; echo err >&2; exit 3", &Within::new(LONG));
    assert_eq!(code(out.status), Some(3));
    assert_eq!(text(&out.stdout), "out\n");
    assert_eq!(text(&out.stderr), "err\n");
    assert!(!out.timed_out && !out.truncated && !out.drain_incomplete);
    let out = run("exit 0", &Within::new(LONG));
    assert_eq!(code(out.status), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
}

#[test]
fn output_larger_than_a_pipe_is_all_kept() {
    if !runs_scripts() {
        return;
    }
    let out = run(
        "head -c 300000 /dev/zero | tr '\\0' x; head -c 200000 /dev/zero | tr '\\0' y >&2",
        &Within::new(LONG),
    );
    assert_eq!(out.stdout.len(), 300_000);
    assert_eq!(out.stderr.len(), 200_000);
    assert!(!out.truncated);
}

#[test]
fn the_cap_is_per_stream_and_says_it_cut() {
    if !runs_scripts() {
        return;
    }
    let within = Within::new(LONG).max_bytes(10);
    let out = run("printf 0123456789; printf abcdefghij >&2", &within);
    assert_eq!(
        (text(&out.stdout), text(&out.stderr)),
        ("0123456789".into(), "abcdefghij".into())
    );
    assert!(!out.truncated, "exactly the cap is not over it");
    let out = run("printf 0123456789A; printf abc >&2", &within);
    assert_eq!(text(&out.stdout), "0123456789");
    assert_eq!(text(&out.stderr), "abc");
    assert!(out.truncated);
    let out = run("printf abc; printf 0123456789A >&2", &within);
    assert_eq!(text(&out.stdout), "abc");
    assert_eq!(text(&out.stderr), "0123456789");
    assert!(out.truncated, "either stream's cut counts");
}

#[test]
fn a_flood_over_the_cap_does_not_block_the_program() {
    if !runs_scripts() {
        return;
    }
    let within = Within::new(LONG).max_bytes(100);
    let out = run("head -c 1000000 /dev/zero | tr '\\0' x", &within);
    assert_eq!(out.stdout.len(), 100);
    assert!(out.truncated && !out.timed_out);
    assert_eq!(code(out.status), Some(0));
}

#[test]
fn the_environment_is_exactly_the_specs() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let spec = machine
        .spec("echo \"a=${A-unset} b=${B-unset} home=${HOME-unset}\"")
        .envs([("A", "1"), ("B", "x")])
        .envs([("B", "2")]);
    let out = run_within(&spec, &Within::new(LONG)).unwrap();
    assert_eq!(text(&out.stdout), "a=1 b=2 home=unset\n");
}

#[test]
fn arguments_and_the_working_directory_are_the_specs() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let spec = machine
        .spec("echo \"$# $1 $2\"; pwd -P")
        .args(["one"])
        .args(["two words"])
        .current_dir(machine.real_dir());
    let out = run_within(&spec, &Within::new(LONG)).unwrap();
    let real = machine.real_dir();
    assert_eq!(
        text(&out.stdout),
        format!("2 one two words\n{}\n", real.display())
    );
}

#[test]
fn input_is_fed_and_closed() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let big = vec![b'z'; 300_000];
    let out = run_within(&machine.spec("cat").stdin(big.clone()), &Within::new(LONG)).unwrap();
    assert_eq!(out.stdout, big);
    let out = run_within(
        &machine.spec("cat").stdin(b"fed".to_vec()),
        &Within::new(LONG),
    )
    .unwrap();
    assert_eq!(text(&out.stdout), "fed");
}

#[test]
fn without_input_stdin_is_closed_at_once() {
    if !runs_scripts() {
        return;
    }
    let out = run("cat; echo after", &Within::new(LONG));
    assert_eq!(text(&out.stdout), "after\n");
}

#[test]
fn a_program_that_never_reads_its_input_does_not_hold_the_run() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let spec = machine.spec("exec sleep 30").stdin(vec![b'q'; 1_000_000]);
    let started = Instant::now();
    let out = run_within(&spec, &Within::new(TINY)).unwrap();
    assert!(out.timed_out);
    assert!(started.elapsed() < PROMPT);
    let spec = machine.spec("exit 0").stdin(vec![b'q'; 1_000_000]);
    let out = run_within(&spec, &Within::new(LONG)).unwrap();
    assert_eq!(code(out.status), Some(0));
}

#[test]
fn a_program_past_its_deadline_is_killed_and_the_run_returns_promptly() {
    if !runs_scripts() {
        return;
    }
    let started = Instant::now();
    let out = run("exec sleep 30", &Within::new(TINY));
    assert!(started.elapsed() < PROMPT, "{:?}", started.elapsed());
    assert!(out.timed_out);
    assert_eq!(out.status, None);
    assert!(!out.drain_incomplete);
}

#[test]
fn the_whole_tree_dies_at_the_deadline_and_so_do_the_pipes() {
    if !runs_scripts() {
        return;
    }
    // A background sleep holds both pipes, as a helper a program started
    // would. The pipes end only if it was killed with its parent. The
    // deadline is long enough for the script to have started it.
    let started = Instant::now();
    let out = run(
        "sleep 30 &\necho $!\nwait",
        &Within::new(Duration::from_millis(500)).drain_grace(PROMPT),
    );
    assert!(started.elapsed() < PROMPT, "{:?}", started.elapsed());
    assert!(out.timed_out);
    assert!(
        text(&out.stdout).trim().parse::<u32>().is_ok(),
        "the descendant started: {:?}",
        text(&out.stdout)
    );
    assert!(
        !out.drain_incomplete,
        "a surviving descendant holds the pipes"
    );
}

#[test]
fn a_descendant_that_outlives_a_normal_exit_does_not_hang_the_run() {
    if !runs_scripts() {
        return;
    }
    let started = Instant::now();
    let out = run("sleep 5 &\necho done", &Within::new(LONG).drain_grace(TINY));
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(code(out.status), Some(0));
    assert!(!out.timed_out);
    assert_eq!(text(&out.stdout), "done\n", "what was read is kept");
    assert!(out.drain_incomplete);
}

#[test]
fn a_descendant_that_outlives_a_kill_by_escaping_the_tree_does_not_hang_the_run() {
    if !runs_scripts() {
        return;
    }
    // setsid is the documented way out of the tree.
    let machine = Machine::new();
    let spec = machine.spec("setsid sleep 5 &\nexec sleep 30");
    let started = Instant::now();
    let out = run_within(
        &spec,
        &Within::new(Duration::from_millis(300)).drain_grace(TINY),
    );
    let out = out.unwrap();
    assert!(started.elapsed() < Duration::from_secs(4));
    assert!(out.timed_out);
}

#[test]
fn an_unbounded_pause_is_not_one_the_deadline_waits_out() {
    if !runs_scripts() {
        return;
    }
    let started = Instant::now();
    let out = run("exec sleep 30", &Within::new(Duration::ZERO));
    assert!(out.timed_out);
    assert!(started.elapsed() < PROMPT);
}

#[test]
fn a_program_that_is_not_there_is_not_found() {
    let error = run_within(&Spec::new("/no/such/program"), &Within::new(LONG)).unwrap_err();
    assert!(matches!(error, RunError::NotFound { .. }), "{error}");
    assert_eq!(error.program(), "/no/such/program");
    assert_eq!(error.io().kind(), io::ErrorKind::NotFound);
}

#[test]
fn a_missing_working_directory_is_not_found_too() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let spec = machine.spec("exit 0").current_dir("/no/such/dir");
    let error = run_within(&spec, &Within::new(LONG)).unwrap_err();
    assert!(matches!(error, RunError::NotFound { .. }), "{error}");
}

#[test]
fn a_program_that_is_not_executable_is_unrunnable_not_missing() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let path = machine.dir.join("plain");
    machine
        .fs
        .write_atomic(&path, b"#!/bin/sh\n", Perms::Exact(Mode::FILE))
        .unwrap();
    let spec = Spec::new(machine.fs.real_path(&path).unwrap());
    let error = run_within(&spec, &Within::new(LONG)).unwrap_err();
    assert!(matches!(error, RunError::Unrunnable { .. }), "{error}");
    assert_eq!(error.io().kind(), io::ErrorKind::PermissionDenied);
    // A directory is found and cannot be run.
    let error = run_within(&Spec::new(machine.real_dir()), &Within::new(LONG)).unwrap_err();
    assert!(matches!(error, RunError::Unrunnable { .. }), "{error}");
}

#[test]
fn the_defaults_and_their_overrides() {
    let within = Within::new(Duration::from_secs(7));
    assert_eq!(within.deadline, Duration::from_secs(7));
    assert_eq!(within.drain_grace, Duration::from_secs(2));
    assert_eq!(within.max_bytes, 64 * 1024 * 1024);
    let within = within.drain_grace(TINY).max_bytes(5);
    assert_eq!((within.drain_grace, within.max_bytes), (TINY, 5));
    assert_eq!(within.deadline, Duration::from_secs(7));
}

// --- the system's calls failing, which a real program does not make them do.

/// A started program whose calls are scripted.
struct Fake {
    waits: Vec<io::Result<Option<ExitStatus>>>,
    asks: usize,
    kill: Option<io::Error>,
    killed: bool,
}

impl Fake {
    fn new(waits: Vec<io::Result<Option<ExitStatus>>>, kill: Option<io::Error>) -> Fake {
        Fake {
            waits,
            asks: 0,
            kill,
            killed: false,
        }
    }
}

impl Running for Fake {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.asks += 1;
        if self.waits.is_empty() {
            Ok(None)
        } else {
            self.waits.remove(0)
        }
    }

    fn kill_tree(&mut self) -> io::Result<()> {
        self.killed = true;
        self.kill.take().map_or(Ok(()), Err)
    }
}

fn waited(fake: &mut Fake, deadline: Duration) -> Result<Option<ExitStatus>, RunError> {
    let (tx, rx) = mpsc::channel();
    let _keep = tx;
    wait(
        fake,
        &Spec::new("demo"),
        deadline,
        &rx,
        &mut Ended::default(),
    )
}

#[test]
fn a_wait_that_fails_kills_the_tree_and_says_so() {
    let mut fake = Fake::new(vec![Err(io::Error::other("lost"))], None);
    let error = waited(&mut fake, LONG).unwrap_err();
    assert!(matches!(error, RunError::Wait { .. }), "{error}");
    assert_eq!(error.io().to_string(), "lost");
    assert!(fake.killed);
}

#[test]
fn a_kill_that_fails_at_the_deadline_is_an_error_not_a_timeout() {
    let mut fake = Fake::new(vec![Ok(None)], Some(io::Error::other("denied")));
    let error = waited(&mut fake, Duration::ZERO).unwrap_err();
    assert!(matches!(error, RunError::Kill { .. }), "{error}");
    assert_eq!(error.program(), "demo");
}

#[test]
fn a_program_still_running_is_asked_again_until_it_is_over() {
    if !runs_scripts() {
        return;
    }
    let status = run("exit 0", &Within::new(LONG)).status.unwrap();
    let mut fake = Fake::new(vec![Ok(None), Ok(None), Ok(Some(status))], None);
    assert_eq!(waited(&mut fake, LONG).unwrap(), Some(status));
    assert_eq!(fake.asks, 3);
    assert!(!fake.killed);
}

#[test]
fn a_program_still_running_at_the_deadline_is_killed_without_a_status() {
    let mut fake = Fake::new(vec![], None);
    assert_eq!(waited(&mut fake, TINY).unwrap(), None);
    assert!(fake.killed);
    assert!(fake.asks >= 1);
}

// --- ETXTBSY

#[test]
fn only_a_busy_text_file_is_tried_again() {
    let busy = || io::Error::from(io::ErrorKind::ExecutableFileBusy);
    let mut calls = 0;
    let got = retrying_busy(10, || {
        calls += 1;
        if calls < 4 {
            Err(busy())
        } else {
            Ok(calls)
        }
    });
    assert_eq!(got.unwrap(), 4);

    let mut calls = 0;
    let got: io::Result<()> = retrying_busy(10, || {
        calls += 1;
        Err(io::ErrorKind::NotFound.into())
    });
    assert_eq!(got.unwrap_err().kind(), io::ErrorKind::NotFound);
    assert_eq!(calls, 1, "another error is not retried");
}

#[test]
fn a_file_that_stays_busy_is_tried_a_last_time_and_its_error_returned() {
    let mut calls = 0;
    let got: io::Result<()> = retrying_busy(3, || {
        calls += 1;
        Err(io::ErrorKind::ExecutableFileBusy.into())
    });
    assert_eq!(got.unwrap_err().kind(), io::ErrorKind::ExecutableFileBusy);
    assert_eq!(calls, 4);
    let mut calls = 0;
    let got = retrying_busy(3, || {
        calls += 1;
        if calls < 4 {
            Err(io::ErrorKind::ExecutableFileBusy.into())
        } else {
            Ok(())
        }
    });
    assert!(got.is_ok(), "the last try counts");
}

#[test]
fn draining_waits_for_the_readers_only_up_to_the_grace() {
    let (tx, rx) = mpsc::channel();
    let mut ended = Ended::default();
    assert!(
        !drain(&rx, &mut ended, TINY),
        "none ended, one grace passed"
    );
    tx.send(()).unwrap();
    assert!(!drain(&rx, &mut ended, TINY), "one of two");
    assert_eq!(ended.0, 1);
    tx.send(()).unwrap();
    assert!(drain(&rx, &mut ended, TINY));
    assert!(ended.both());
    assert!(
        drain(&rx, &mut ended, Duration::ZERO),
        "nothing left to wait for"
    );
    let mut none = Ended::default();
    assert!(!drain(&rx, &mut none, Duration::ZERO));
}
