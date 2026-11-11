//! The exit a command ends with: `exit-code`'s [`Exit`], with what the
//! command line adds to it (a parser's error as an exit).

pub use exit_code::{convention, Class, Exit, ExitClass};

/// The exit of an argument parser's error: a usage error is refused (20,
/// never clap's own 2); a request for help or the version is ok (0).
pub fn usage_exit(error: &clap::Error) -> Exit {
    Exit::usage(error.use_stderr())
}

#[cfg(test)]
#[path = "exit.test.rs"]
mod tests;
