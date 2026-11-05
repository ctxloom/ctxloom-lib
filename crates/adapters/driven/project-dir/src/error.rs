//! Why a project id was refused.

use std::fmt;

use angzarr_diagnostic::{code, Arg, Diagnostic};

/// This crate's en-US messages, its name first.
pub const EN_US: (&str, &str) = (
    env!("CARGO_PKG_NAME"),
    include_str!(concat!(env!("OUT_DIR"), "/en-US.ftl")),
);

/// A project id that cannot safely be one directory name under the projects
/// directory. The id is shown as given.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProjectDirError {
    /// The id is empty.
    IdEmpty,
    /// The id has a NUL character in it.
    IdNul { id: String },
    /// The id is an absolute path.
    IdAbsolute { id: String },
    /// The id has a path separator (`/` or `\`) in it.
    IdSeparator { id: String },
    /// The id is `.` or `..`.
    IdRelative { id: String },
}

impl fmt::Display for ProjectDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl std::error::Error for ProjectDirError {}

impl ProjectDirError {
    /// The error's code, for a caller that must hold it past the error (a
    /// store's cause, which names its code as a `&'static str`).
    pub fn static_code(&self) -> &'static str {
        match self {
            Self::IdEmpty => code!("id-empty"),
            Self::IdNul { .. } => code!("id-nul"),
            Self::IdAbsolute { .. } => code!("id-absolute"),
            Self::IdSeparator { .. } => code!("id-separator"),
            Self::IdRelative { .. } => code!("id-relative"),
        }
    }

    /// The id the error refused, `None` for the empty one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::IdEmpty => None,
            Self::IdNul { id }
            | Self::IdAbsolute { id }
            | Self::IdSeparator { id }
            | Self::IdRelative { id } => Some(id),
        }
    }
}

impl Diagnostic for ProjectDirError {
    fn code(&self) -> &str {
        self.static_code()
    }

    fn args(&self) -> Vec<(&str, Arg)> {
        self.id().map(|id| ("id", id.into())).into_iter().collect()
    }
}

/// One value of every error, for the catalog test.
#[cfg(any(test, feature = "test-support"))]
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
    vec![
        Box::new(ProjectDirError::IdEmpty),
        Box::new(ProjectDirError::IdNul { id: "a\0b".into() }),
        Box::new(ProjectDirError::IdAbsolute {
            id: "/etc/x".into(),
        }),
        Box::new(ProjectDirError::IdSeparator { id: "a/b".into() }),
        Box::new(ProjectDirError::IdRelative { id: "..".into() }),
    ]
}

#[cfg(test)]
#[path = "error.test.rs"]
mod tests;
