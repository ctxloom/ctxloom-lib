use file_system::Fs;

use super::{process_gone, state_is_gone};

#[test]
fn a_dead_process_not_yet_released_is_gone() {
    // The line a loaded machine showed for a killed `sleep`.
    assert!(state_is_gone("1139644 (sleep) X 0 -1 -1 0 -1 4194560"));
    assert!(state_is_gone("1139644 (sleep) x 0 -1 -1 0 -1 4194560"));
}

#[test]
fn a_zombie_is_gone() {
    assert!(state_is_gone("42 (sleep) Z 1 42 42 0 -1 4194564"));
}

#[test]
fn a_live_process_is_not_gone_in_any_live_state() {
    for state in ["R", "S", "D", "T", "t", "I"] {
        assert!(
            !state_is_gone(&format!("42 (sleep) {state} 1 42 42 0 -1 0")),
            "{state}"
        );
    }
}

#[test]
fn a_command_that_looks_like_a_state_does_not_decide_it() {
    // The state is after the last ") ", so a name holding one is not read.
    assert!(!state_is_gone("42 (a) Z (b) S 1 42 42 0 -1 0"));
    assert!(state_is_gone("42 (a) S (b) Z 1 42 42 0 -1 0"));
}

#[test]
fn a_line_that_is_not_a_stat_line_is_gone() {
    assert!(state_is_gone(""));
    assert!(state_is_gone("garbage"));
    assert!(state_is_gone("42 (sleep) "));
}

#[test]
fn a_pid_with_no_entry_is_gone() {
    assert!(process_gone("4294967294"));
}

#[test]
fn this_process_is_not_gone() {
    let host =
        file_system::Os::new(os::host(), std::path::Path::new(os::host().flavor().top())).unwrap();
    if !host.exists(std::path::Path::new("/proc/self")).unwrap() {
        return;
    }
    assert!(!process_gone(&std::process::id().to_string()));
}
