//! A piped program is driven live, has no deadline, and ends only when it
//! exits or its tree is ended; none of it by sleeping in a test: a program
//! that must still be running is blocked reading a stdin the test holds open,
//! so no interleaving of the test's threads can let it end early.

use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use file_system::{Fs, Rooted};
use os::{Flavor, Mode};

use super::*;
use crate::{install_script, process_gone, run_within, Within};

/// Far longer than anything here takes, far shorter than a `sleep 30`.
const PROMPT: Duration = Duration::from_secs(10);
const LONG: Duration = Duration::from_secs(60);
const TINY: Duration = Duration::from_millis(30);

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
        // Run by the interpreter, never exec'd: see `install_script`.
        Spec::new("/bin/sh").args([path])
    }
}

fn piped(body: &str) -> (Piped, Pipes) {
    run_piped(&Machine::new().spec(body), |_| {}).unwrap()
}

/// The next line of `reader`, as text.
fn line(reader: &mut impl BufRead) -> String {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    line
}

fn say(stdin: &mut impl Write, text: &str) {
    stdin.write_all(text.as_bytes()).unwrap();
    stdin.flush().unwrap();
}

/// A real exit status `code`, from a program that exits with it.
fn status(code: i32) -> ExitStatus {
    let out = run_within(
        &Machine::new().spec(&format!("exit {code}")),
        &Within::new(LONG),
    );
    out.unwrap().status.unwrap()
}

#[test]
fn a_line_written_is_answered_while_the_program_runs_and_its_eof_ends_it() {
    if !runs_scripts() {
        return;
    }
    let (child, pipes) =
        piped("while read -r line; do echo \"out:$line\"; echo \"err:$line\" >&2; done; exit 5");
    let Pipes {
        mut stdin,
        stdout,
        stderr,
    } = pipes;
    let (mut stdout, mut stderr) = (BufReader::new(stdout), BufReader::new(stderr));
    say(&mut stdin, "one\n");
    assert_eq!(line(&mut stdout), "out:one\n");
    assert_eq!(line(&mut stderr), "err:one\n");
    say(&mut stdin, "two\n");
    assert_eq!(
        line(&mut stdout),
        "out:two\n",
        "the same program, still live"
    );
    assert_eq!(line(&mut stderr), "err:two\n");
    assert_eq!(child.try_wait().unwrap(), None, "blocked on the open stdin");
    drop(stdin);
    assert_eq!(child.wait().unwrap().code(), Some(5));
    assert_eq!(line(&mut stdout), "", "its stdout is at its end");
    assert_eq!(child.try_wait().unwrap().and_then(|s| s.code()), Some(5));
}

#[test]
fn started_is_called_once_with_the_pid_the_handle_gives() {
    if !runs_scripts() {
        return;
    }
    let machine = Machine::new();
    let mut seen = Vec::new();
    let (child, pipes) =
        run_piped(&machine.spec("echo $$; read -r x"), |pid| seen.push(pid)).unwrap();
    assert_eq!(seen, [child.id()]);
    let mut stdout = BufReader::new(pipes.stdout);
    assert_eq!(line(&mut stdout).trim(), child.id().to_string());
    drop(pipes.stdin);
    assert_eq!(child.wait().unwrap().code(), Some(0));
    assert_eq!(seen.len(), 1);
}

#[test]
fn a_program_that_is_not_there_is_not_found_and_not_started() {
    let mut called = false;
    let error = run_piped(&Spec::new("/no/such/program"), |_| called = true).unwrap_err();
    assert!(matches!(error, RunError::NotFound { .. }), "{error}");
    assert_eq!(error.program(), "/no/such/program");
    assert!(!called);
}

#[test]
fn the_environment_is_exactly_the_specs_and_its_input_is_not_fed() {
    if !runs_scripts() {
        return;
    }
    let spec = Machine::new()
        .spec("echo \"a=${A-unset} home=${HOME-unset}\"; read -r x; echo \"got:$x\"")
        .envs([("A", "1")])
        .stdin("fed\n");
    let (child, mut pipes) = run_piped(&spec, |_| {}).unwrap();
    let mut stdout = BufReader::new(pipes.stdout);
    assert_eq!(line(&mut stdout), "a=1 home=unset\n");
    say(&mut pipes.stdin, "ours\n");
    assert_eq!(
        line(&mut stdout),
        "got:ours\n",
        "the caller's, never the spec's"
    );
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn a_kill_from_another_thread_ends_a_wait_and_the_whole_tree() {
    if !runs_scripts() {
        return;
    }
    let (child, pipes) = piped("sleep 30 & echo $!; read -r x");
    let descendant = line(&mut BufReader::new(pipes.stdout));
    let began = Instant::now();
    let waiter = {
        let child = child.clone();
        std::thread::spawn(move || child.wait())
    };
    child.kill_tree().unwrap();
    let status = waiter.join().unwrap().unwrap();
    assert_eq!(status.code(), None, "a kill ended it");
    assert!(began.elapsed() < PROMPT);
    assert!(
        process_gone(&descendant),
        "the descendant {descendant} lives"
    );
    // The stdin is still held: the kill, not an end of input, ended it.
    drop(pipes.stdin);
    assert_eq!(child.try_wait().unwrap(), Some(status));
}

#[test]
fn a_kill_after_the_program_exited_still_ends_what_it_left_running() {
    if !runs_scripts() {
        return;
    }
    let (child, pipes) = piped("sleep 30 & echo $!; exit 0");
    let descendant = line(&mut BufReader::new(pipes.stdout));
    assert_eq!(child.wait().unwrap().code(), Some(0));
    assert!(!process_gone(&descendant), "nothing ended it yet");
    child.kill_tree().unwrap();
    assert!(
        process_gone(&descendant),
        "the descendant {descendant} lives"
    );
    assert_eq!(
        child.wait().unwrap().code(),
        Some(0),
        "the same status ever after"
    );
}

#[test]
fn a_terminate_asks_the_tree_first_and_the_program_may_finish() {
    if !runs_scripts() {
        return;
    }
    let (child, pipes) = piped("trap 'echo asked; exit 7' TERM; echo ready; sleep 30 & wait");
    let mut stdout = BufReader::new(pipes.stdout);
    assert_eq!(line(&mut stdout), "ready\n", "its trap is set");
    let began = Instant::now();
    child.terminate_tree(LONG).unwrap();
    assert!(began.elapsed() < PROMPT);
    assert_eq!(child.wait().unwrap().code(), Some(7), "the ask, not a kill");
    assert_eq!(line(&mut stdout), "asked\n");
}

#[test]
fn a_handle_is_shared_across_threads_and_its_debug_names_only_program_and_pid() {
    fn shared<T: Send + Sync + Clone + 'static>() {}
    shared::<Piped>();
    let child = Piped::over(Fake::new(vec![]), "demo-prog", 4242);
    let shown = format!("{child:?}");
    assert!(
        shown.contains("demo-prog") && shown.contains("4242"),
        "{shown}"
    );
    assert_eq!(child.clone().id(), 4242);
}

// --- the system's calls failing, which a real program does not make them do.

/// What a [`Fake`] was asked, shared with the test that gave it away.
#[derive(Default)]
struct Log {
    asks: usize,
    killed: bool,
    terminated: Vec<Duration>,
}

/// A started program whose calls are scripted.
struct Fake {
    waits: Vec<io::Result<Option<ExitStatus>>>,
    kill: Option<io::Error>,
    log: Arc<Mutex<Log>>,
}

impl Fake {
    fn new(waits: Vec<io::Result<Option<ExitStatus>>>) -> Fake {
        Fake {
            waits,
            kill: None,
            log: Arc::default(),
        }
    }

    fn failing_kill(mut self, error: &str) -> Fake {
        self.kill = Some(io::Error::other(error.to_string()));
        self
    }

    /// The handle over this fake, and its log.
    fn start(self) -> (Piped, Arc<Mutex<Log>>) {
        let log = Arc::clone(&self.log);
        (Piped::over(self, "demo", 7), log)
    }
}

impl Running for Fake {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.log.lock().unwrap().asks += 1;
        if self.waits.is_empty() {
            Ok(None)
        } else {
            self.waits.remove(0)
        }
    }

    fn kill_tree(&mut self) -> io::Result<()> {
        self.log.lock().unwrap().killed = true;
        self.kill.take().map_or(Ok(()), Err)
    }

    fn terminate_tree(&mut self, grace: Duration) -> io::Result<()> {
        self.log.lock().unwrap().terminated.push(grace);
        self.kill.take().map_or(Ok(()), Err)
    }
}

#[test]
fn a_wait_asks_again_until_the_program_is_over() {
    if !runs_scripts() {
        return;
    }
    let done = status(3);
    let (child, log) = Fake::new(vec![Ok(None), Ok(None), Ok(Some(done))]).start();
    assert_eq!(child.wait().unwrap(), done);
    let log = log.lock().unwrap();
    assert_eq!(log.asks, 3);
    assert!(!log.killed);
}

#[test]
fn a_wait_that_fails_kills_the_tree_and_says_so() {
    for wait in [true, false] {
        let (child, log) = Fake::new(vec![Err(io::Error::other("lost"))]).start();
        let error = if wait {
            child.wait().unwrap_err()
        } else {
            child.try_wait().unwrap_err()
        };
        assert!(matches!(error, RunError::Wait { .. }), "{error}");
        assert_eq!(error.io().to_string(), "lost");
        assert_eq!(error.program(), "demo");
        assert!(log.lock().unwrap().killed);
    }
}

#[test]
fn a_program_still_running_is_not_killed_by_a_look() {
    let (child, log) = Fake::new(vec![Ok(None)]).start();
    assert_eq!(child.try_wait().unwrap(), None);
    let log = log.lock().unwrap();
    assert_eq!(log.asks, 1);
    assert!(!log.killed);
}

#[test]
fn a_kill_that_fails_is_a_kill_error() {
    let (child, log) = Fake::new(vec![]).failing_kill("denied").start();
    let error = child.kill_tree().unwrap_err();
    assert!(matches!(error, RunError::Kill { .. }), "{error}");
    assert_eq!(error.io().to_string(), "denied");
    assert_eq!(error.program(), "demo");
    assert!(log.lock().unwrap().killed);
    assert!(child.kill_tree().is_ok(), "the next kill worked");
}

#[test]
fn a_terminate_gives_its_grace_and_its_failure_is_a_kill_error() {
    let (child, log) = Fake::new(vec![]).start();
    child.terminate_tree(TINY).unwrap();
    assert_eq!(log.lock().unwrap().terminated, [TINY]);
    assert!(!log.lock().unwrap().killed, "asked, not killed outright");
    let (child, _) = Fake::new(vec![]).failing_kill("denied").start();
    let error = child.terminate_tree(TINY).unwrap_err();
    assert!(matches!(error, RunError::Kill { .. }), "{error}");
    assert_eq!(error.program(), "demo");
}
