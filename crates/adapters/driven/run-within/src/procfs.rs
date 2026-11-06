//! Test support: whether a process the run killed is gone.

use std::path::Path;

use file_system::{Fs, Os};

/// Whether a process is gone, by its pid as text (a script's `echo $!`):
/// its `/proc/<pid>/stat` cannot be read, or its state is `Z` (a zombie no
/// one has reaped: an orphan waits for init) or `X`/`x` (dead, and not yet
/// released from `/proc`, which a kill's reader can catch).
///
/// Where `/proc` is absent every pid reads as gone: a test that wants the
/// check gates on `/proc/self` itself.
pub fn process_gone(pid: &str) -> bool {
    let top = os::host().flavor().top();
    let Ok(host) = Os::new(os::host(), Path::new(top)) else {
        return true;
    };
    let stat = Path::new("/proc").join(pid.trim()).join("stat");
    // A process can vanish between the look and the read: that is gone too.
    host.read_to_string(&stat)
        .map_or(true, |line| state_is_gone(&line))
}

/// Whether the `/proc/<pid>/stat` line `stat` is a process that is gone. The
/// state is the letter after the LAST `") "`: the command before it is
/// parenthesised, may hold spaces and parentheses, and can itself look like
/// a state.
pub(crate) fn state_is_gone(stat: &str) -> bool {
    let Some((_, after)) = stat.rsplit_once(") ") else {
        // Not a stat line (empty, or cut short): there is nothing alive in it.
        return true;
    };
    matches!(after.chars().next(), Some('Z' | 'X' | 'x') | None)
}

#[cfg(test)]
#[path = "procfs.test.rs"]
mod tests;
