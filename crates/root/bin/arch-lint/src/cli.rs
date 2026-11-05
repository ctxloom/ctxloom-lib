//! The command line: read the rules, read the workspace, say what is found.

use std::path::PathBuf;

use clap::Parser;
use file_system::Fs;

use crate::check::check;
use crate::load::load;
use crate::rules::Rules;

/// The command line.
#[derive(Debug, Parser)]
#[command(
    name = "arch-lint",
    about = "Hold a workspace's dependency edges to a table of which role may depend on which"
)]
pub struct Args {
    /// The workspace's Cargo.toml.
    #[arg(long, default_value = "Cargo.toml")]
    pub manifest_path: PathBuf,
    /// The rules table.
    #[arg(long, default_value = "arch-rules.toml")]
    pub rules: PathBuf,
    /// Fail on warnings (dev-dependency findings) too.
    #[arg(long)]
    pub deny_warnings: bool,
}

/// What to print, and the exit code: 0 passed, 1 a finding failed the gate,
/// 2 the rules or the workspace could not be read.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub text: String,
    pub code: i32,
}

/// Run the lint as `args` say, reading the rules through `fs`.
pub fn run(args: &Args, fs: &dyn Fs) -> Outcome {
    let tool_failure = |text: String| Outcome { text, code: 2 };
    let text = match fs.read_to_string(&args.rules) {
        Ok(text) => text,
        Err(e) => {
            return tool_failure(format!(
                "cannot read the rules {}: {e}\n",
                args.rules.display()
            ))
        }
    };
    let rules = match Rules::parse(&text) {
        Ok(rules) => rules,
        Err(e) => return tool_failure(format!("{}: {e}\n", args.rules.display())),
    };
    let graph = match load(&args.manifest_path) {
        Ok(graph) => graph,
        Err(e) => {
            return tool_failure(format!(
                "cannot read the workspace {}: {e}\n",
                args.manifest_path.display()
            ))
        }
    };
    let report = check(&graph, &rules);
    Outcome {
        text: report.render(args.deny_warnings),
        code: i32::from(report.failed(args.deny_warnings)),
    }
}

#[cfg(test)]
#[path = "cli.test.rs"]
mod tests;
