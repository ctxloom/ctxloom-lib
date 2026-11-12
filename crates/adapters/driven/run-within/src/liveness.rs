//! Test support: whether a process the run killed is gone.

use std::time::Duration;

use crate::{run_within, Spec, Within};

/// Where `ps` is: `/bin` on macOS and on a merged-`/usr` Linux, `/usr/bin`
/// on the rest.
const PS_PATH: &str = "/bin:/usr/bin";

/// Far longer than a `ps` of one pid takes.
const PS_DEADLINE: Duration = Duration::from_secs(10);

/// Whether a process is gone, by its pid as text (a script's `echo $!`):
/// `ps -o stat= -p <pid>` names no process, or names a zombie (`Z`: no one
/// has reaped it, as an orphan waits for init) or a dead one (`X`/`x`: not
/// yet released, which a kill's reader can catch). A zombie is gone because
/// it runs nothing and holds nothing a test waits on: what is asked is
/// whether the kill landed, not whether someone reaped it.
///
/// It asks `ps`, the POSIX one on Linux and macOS alike, through this
/// crate's own [`run_within`]. Where `ps` cannot be run (Windows) it panics:
/// an answer it cannot give is not "gone", which would pass the test that
/// asked.
pub fn process_gone(pid: &str) -> bool {
    let spec = Spec::new("ps")
        .args(["-o", "stat=", "-p", pid.trim()])
        .envs([("PATH", PS_PATH), ("LC_ALL", "C")]);
    let outcome = match run_within(&spec, &Within::new(PS_DEADLINE)) {
        Ok(outcome) => outcome,
        Err(error) => panic!("process_gone could not run ps: {error}"),
    };
    assert!(
        outcome.status.is_some(),
        "process_gone: ps -p {pid} did not answer within {PS_DEADLINE:?}"
    );
    // A pid ps does not find is no line and a non-zero exit; the line alone
    // decides.
    stat_is_gone(&String::from_utf8_lossy(&outcome.stdout))
}

/// Whether `ps -o stat=`'s line `stat` is a process that is gone: no state at
/// all, or a state whose letter (the first; modifiers such as `s` or `+`
/// follow it) is `Z`, `X` or `x`.
pub(crate) fn stat_is_gone(stat: &str) -> bool {
    matches!(stat.trim().chars().next(), Some('Z' | 'X' | 'x') | None)
}

#[cfg(test)]
#[path = "liveness.test.rs"]
mod tests;
