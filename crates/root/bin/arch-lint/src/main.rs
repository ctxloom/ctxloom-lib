use std::process::ExitCode;

use arch_lint::{run, Args};
use clap::Parser;
use file_system::Os;
use process_env::EnvSnapshot;

// The composition root: the process's arguments, environment and filesystem
// in, the outcome out. Its logic is `arch_lint::run`, tested there.
#[cfg_attr(test, mutants::skip)]
fn main() -> ExitCode {
    let args = Args::parse();
    let env = EnvSnapshot::from_process();
    let fs = match env.cwd().and_then(|cwd| Os::new(os::host(), &cwd)) {
        Ok(fs) => fs,
        Err(e) => {
            eprintln!("cannot find the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let outcome = run(&args, &fs);
    print!("{}", outcome.text);
    ExitCode::from(u8::try_from(outcome.code).unwrap_or(2))
}
