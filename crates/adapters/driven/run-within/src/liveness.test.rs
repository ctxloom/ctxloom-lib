use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use os::Flavor;

use super::{process_gone, stat_is_gone};

fn runs_ps() -> bool {
    os::host().flavor() == Flavor::Unix
}

#[test]
fn a_zombie_is_gone() {
    // procps and BSD ps both give a zombie `Z`, with or without modifiers.
    assert!(stat_is_gone("Z"));
    assert!(stat_is_gone("Z+"));
    assert!(stat_is_gone("Zs\n"));
}

#[test]
fn a_dead_process_not_yet_released_is_gone() {
    // procps's `X`: dead, caught before the kernel let it go.
    assert!(stat_is_gone("X"));
    assert!(stat_is_gone("x\n"));
}

#[test]
fn a_live_process_is_not_gone_in_any_live_state() {
    // Linux's and macOS's live states, as `ps -o stat=` pads or flags them.
    for stat in [
        "R", "S", "D", "T", "t", "I", "U", "Ss", "R+", "S<l", "  S  \n",
    ] {
        assert!(!stat_is_gone(stat), "{stat:?}");
    }
}

#[test]
fn no_stat_at_all_is_gone() {
    // ps names no process it cannot find: an empty line, or blanks.
    assert!(stat_is_gone(""));
    assert!(stat_is_gone("\n"));
    assert!(stat_is_gone("   "));
}

#[test]
fn a_pid_with_no_process_is_gone() {
    if !runs_ps() {
        return;
    }
    assert!(process_gone("4294967294"));
    assert!(process_gone(" 4294967294\n"));
}

#[test]
fn this_process_is_not_gone() {
    if !runs_ps() {
        return;
    }
    assert!(!process_gone(&std::process::id().to_string()));
    assert!(!process_gone(&format!(" {}\n", std::process::id())));
}

#[test]
fn a_child_killed_and_not_yet_reaped_is_gone() {
    if !runs_ps() {
        return;
    }
    let mut child = Command::new("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id().to_string();
    let alive = !process_gone(&pid);
    child.kill().unwrap();
    // Not waited for: it is a zombie until the wait below, and reads gone.
    let until = Instant::now() + Duration::from_secs(10);
    let mut gone = process_gone(&pid);
    while !gone && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
        gone = process_gone(&pid);
    }
    let reaped = child.wait();
    // Asked after the kill and the wait, so a failure leaves no `sleep`.
    assert!(alive, "a sleeping {pid} read as gone");
    assert!(gone, "a zombie {pid} read as alive");
    assert!(reaped.is_ok());
}
