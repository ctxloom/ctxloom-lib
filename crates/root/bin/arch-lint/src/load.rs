//! The workspace's graph, as `cargo metadata` reports it.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use cargo_metadata::{DependencyKind, MetadataCommand};

use crate::graph::{Graph, Kind, Package};

/// Why the workspace could not be read.
#[derive(Debug)]
pub struct LoadError(cargo_metadata::Error);

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for LoadError {}

/// The members of the workspace whose manifest is `manifest`, and the edges
/// among them. `--no-deps --offline`: only the workspace's own manifests are
/// read, so there is no network and no lock to resolve. An edge is a path
/// dependency (no registry or git source) on a member's name.
pub fn load(manifest: &Path) -> Result<Graph, LoadError> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest)
        .no_deps()
        .other_options(vec!["--offline".to_string()])
        .exec()
        .map_err(LoadError)?;
    let members = metadata.workspace_packages();
    let names: BTreeSet<&str> = members.iter().map(|p| p.name.as_str()).collect();
    let packages = members
        .iter()
        .map(|p| Package {
            name: p.name.to_string(),
            dir: p
                .manifest_path
                .parent()
                .and_then(|dir| dir.strip_prefix(&metadata.workspace_root).ok())
                .map_or_else(String::new, |dir| dir.as_str().replace('\\', "/")),
            deps: p
                .dependencies
                .iter()
                .filter(|d| d.source.is_none() && names.contains(d.name.as_str()))
                .filter_map(|d| Some((d.name.clone(), kind_of(d.kind)?)))
                .collect(),
        })
        .collect();
    Ok(Graph { packages })
}

fn kind_of(kind: DependencyKind) -> Option<Kind> {
    match kind {
        DependencyKind::Normal => Some(Kind::Normal),
        DependencyKind::Build => Some(Kind::Build),
        DependencyKind::Development => Some(Kind::Dev),
        _ => None,
    }
}

#[cfg(test)]
#[path = "load.test.rs"]
mod tests;
