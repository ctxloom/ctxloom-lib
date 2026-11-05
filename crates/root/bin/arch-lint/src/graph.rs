//! The workspace as the check sees it.

use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;

/// How a package depends on another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Normal,
    Build,
    Dev,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Normal => "normal",
            Kind::Build => "build",
            Kind::Dev => "dev",
        })
    }
}

/// A workspace member: its name, its directory under the workspace root
/// (`/`-separated) and the members it depends on, by name and kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub dir: String,
    pub deps: BTreeSet<(String, Kind)>,
}

/// The workspace's members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    pub packages: Vec<Package>,
}

/// The role (`<ring>/<role>`) of a package in `dir`, when `dir` is
/// `<layout>/<ring>/<role>/<crate>`; `None` for any other place.
pub fn role_of(layout: &str, dir: &str) -> Option<String> {
    match dir.split('/').collect::<Vec<_>>().as_slice() {
        [root, ring, role, krate]
            if *root == layout && ![ring, role, krate].iter().any(|c| c.is_empty()) =>
        {
            Some(format!("{ring}/{role}"))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "graph.test.rs"]
mod tests;
