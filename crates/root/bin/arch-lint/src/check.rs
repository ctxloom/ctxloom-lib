//! The graph against the table.

use std::collections::{BTreeMap, BTreeSet};

use crate::graph::{role_of, Graph, Kind, Package};
use crate::report::{Finding, Level, Problem, Report};
use crate::rules::Rules;

/// Every edge between workspace members that `rules` does not allow, a
/// package off the layout or without a rule, and every exception that
/// excuses nothing. Edges to a crate outside the workspace are not edges.
/// An edge from or to a package that is off the layout or has no rule is
/// not judged: the package is itself the finding.
pub fn check(graph: &Graph, rules: &Rules) -> Report {
    let by_name: BTreeMap<&str, &Package> = graph
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let role_of_package = |p: &Package| -> Option<String> {
        role_of(rules.layout(), &p.dir).filter(|role| rules.has_role(role))
    };
    let mut findings = Vec::new();
    let mut used = BTreeSet::new();
    let mut edges = 0;
    for package in &graph.packages {
        if let Some(problem) = placement(rules, package) {
            findings.push(Finding {
                level: Level::Violation,
                problem,
            });
        }
        let from_role = role_of_package(package);
        for (to, kind) in &package.deps {
            let Some(target) = by_name.get(to.as_str()) else {
                continue;
            };
            edges += 1;
            let (Some(from_role), Some(to_role)) = (&from_role, role_of_package(target)) else {
                continue;
            };
            if *kind == Kind::Dev && !rules.checks_dev(from_role) {
                continue;
            }
            if rules.allows(from_role, to, &to_role) {
                continue;
            }
            if let Some(i) = excepted(rules, &package.name, to, *kind) {
                used.insert(i);
                continue;
            }
            findings.push(Finding {
                level: if *kind == Kind::Dev {
                    Level::Warning
                } else {
                    Level::Violation
                },
                problem: Problem::Edge {
                    from: package.name.clone(),
                    to: to.clone(),
                    kind: *kind,
                    from_role: from_role.clone(),
                    to_role,
                    allowed: rules.allowed_roles(from_role).to_vec(),
                    rule: rules.rule_text(from_role).map(str::to_string),
                },
            });
        }
    }
    for (i, e) in rules.exceptions().iter().enumerate() {
        if !used.contains(&i) {
            findings.push(Finding {
                level: Level::Violation,
                problem: Problem::StaleException {
                    from: e.from.clone(),
                    to: e.to.clone(),
                    kind: e.kind,
                },
            });
        }
    }
    findings.sort();
    Report {
        findings,
        crates: graph.packages.len(),
        edges,
    }
}

/// What is wrong with where `package` is, if anything.
fn placement(rules: &Rules, package: &Package) -> Option<Problem> {
    match role_of(rules.layout(), &package.dir) {
        None => Some(Problem::Misplaced {
            package: package.name.clone(),
            dir: package.dir.clone(),
            layout: rules.layout().to_string(),
        }),
        Some(role) if !rules.has_role(&role) => Some(Problem::NoRule {
            package: package.name.clone(),
            role,
        }),
        Some(_) => None,
    }
}

/// The index of the exception that excuses the edge, if one does.
fn excepted(rules: &Rules, from: &str, to: &str, kind: Kind) -> Option<usize> {
    rules
        .exceptions()
        .iter()
        .position(|e| e.from == from && e.to == to && e.kind.is_none_or(|k| k == kind))
}

#[cfg(test)]
#[path = "check.test.rs"]
mod tests;
