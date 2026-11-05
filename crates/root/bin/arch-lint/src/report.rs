//! What the check found, and how it reads.

use std::fmt;

use crate::graph::Kind;

/// How much a finding matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Fails the gate.
    Violation,
    /// Reported; fails the gate only when warnings are denied.
    Warning,
}

/// What is wrong. The variants are in the order findings are listed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Problem {
    /// A package is not at `<layout>/<ring>/<role>/<crate>`.
    Misplaced {
        package: String,
        dir: String,
        layout: String,
    },
    /// A package's role has no rule in the table.
    NoRule { package: String, role: String },
    /// An edge the table does not allow.
    Edge {
        from: String,
        to: String,
        kind: Kind,
        from_role: String,
        to_role: String,
        allowed: Vec<String>,
        rule: Option<String>,
    },
    /// An exception that excuses nothing.
    StaleException {
        from: String,
        to: String,
        kind: Option<Kind>,
    },
}

/// One thing the check found.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub level: Level,
    pub problem: Problem,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.level {
            Level::Violation => "violation: ",
            Level::Warning => "warning: ",
        })?;
        match &self.problem {
            Problem::Misplaced {
                package,
                dir,
                layout,
            } => write!(
                f,
                "{package} is at {dir}, not {layout}/<ring>/<role>/<crate>"
            ),
            Problem::NoRule { package, role } => {
                write!(f, "{package} ({role}) has no rule in the rules table")
            }
            Problem::Edge {
                from,
                to,
                kind,
                from_role,
                to_role,
                allowed,
                rule,
            } => {
                write!(
                    f,
                    "{from} ({from_role}) -> {to} ({to_role}) [{kind}]: {from_role} may depend on "
                )?;
                if allowed.is_empty() {
                    f.write_str("nothing")?;
                } else {
                    f.write_str(&allowed.join(", "))?;
                }
                match rule {
                    Some(rule) => write!(f, "; {rule}"),
                    None => Ok(()),
                }
            }
            Problem::StaleException { from, to, kind } => {
                write!(f, "exception {from} -> {to}")?;
                if let Some(kind) = kind {
                    write!(f, " [{kind}]")?;
                }
                f.write_str(" matches no finding")
            }
        }
    }
}

/// The check's result: the findings, in order, and what was examined.
#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Workspace members.
    pub crates: usize,
    /// Edges between members.
    pub edges: usize,
}

impl Report {
    /// How many findings fail the gate.
    pub fn violations(&self) -> usize {
        self.count(Level::Violation)
    }

    /// How many findings are warnings.
    pub fn warnings(&self) -> usize {
        self.count(Level::Warning)
    }

    fn count(&self, level: Level) -> usize {
        self.findings.iter().filter(|f| f.level == level).count()
    }

    /// Whether the gate fails: a violation, or a warning when those are
    /// denied.
    pub fn failed(&self, deny_warnings: bool) -> bool {
        self.violations() > 0 || (deny_warnings && self.warnings() > 0)
    }

    /// The report as text: a failure's count line, or a pass's `ok` line,
    /// then the findings, one per line.
    pub fn render(&self, deny_warnings: bool) -> String {
        let (v, w) = (self.violations(), self.warnings());
        let mut out = if self.failed(deny_warnings) {
            if w == 0 {
                format!("{}\n", plural(v, "violation"))
            } else {
                format!("{}, {}\n", plural(v, "violation"), plural(w, "warning"))
            }
        } else if w > 0 {
            format!(
                "ok: {} crates, {} edges ({})\n",
                self.crates,
                self.edges,
                plural(w, "warning")
            )
        } else {
            format!("ok: {} crates, {} edges\n", self.crates, self.edges)
        };
        for finding in &self.findings {
            out.push_str(&finding.to_string());
            out.push('\n');
        }
        out
    }
}

fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
#[path = "report.test.rs"]
mod tests;
