//! The architecture lint: a workspace's dependency edges, held to a table of
//! which role may depend on which.
//!
//! A crate's directory is its role: `crates/<ring>/<role>/<crate>` gives the
//! role `<ring>/<role>`. The consumer supplies the table (`arch-rules.toml`:
//! for each role, the roles and crate classes it may depend on; the
//! exceptions, each with its reason); this crate reads the workspace with
//! `cargo metadata` and reports every edge between workspace members that the
//! table does not allow. Dev-dependency edges are checked more leniently: a
//! finding is a warning, and only for the roles the table names.
//!
//! This is a developer's gate, not a user-facing program, so its failures are
//! plain text for the terminal, not catalog diagnostics.
//!
//! - [`Rules`]: the table, parsed.
//! - [`load`]: the workspace's [`Graph`], from `cargo metadata --no-deps`.
//! - [`check`]: the graph against the table: a [`Report`].
//! - [`run`]: all of it, for the binary: the text to print and the exit code.

mod check;
mod cli;
mod graph;
mod load;
mod report;
mod rules;

pub use check::check;
pub use cli::{run, Args, Outcome};
pub use graph::{role_of, Graph, Kind, Package};
pub use load::{load, LoadError};
pub use report::{Finding, Level, Problem, Report};
pub use rules::{Exception, Rules, RulesError};
