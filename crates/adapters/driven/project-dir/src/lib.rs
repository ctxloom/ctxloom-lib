//! The mechanics of finding a project's root and its per-project directory.
//!
//! What a project IS belongs to the application: the marker that makes a
//! directory a root, and where the walk gives up, arrive as predicates
//! ([`nearest_ancestor`]). This crate walks, looks on the filesystem seam
//! ([`holds_dir`], [`holds_entry`]) and builds the per-project directory from
//! a checked id ([`project_dir`]), so no application has to repeat them.
//!
//! - [`nearest_ancestor`]: the nearest directory, from a start upward, that
//!   satisfies a rule, stopping where a second rule says.
//! - [`holds_dir`] and [`holds_entry`]: the usual rules' building blocks, over
//!   `Fs` (a directory of that name, or anything of it).
//! - [`project_dir`]: `<home>/<app dir>/projects/<id>`, the id refused unless
//!   it is one plain name ([`check_id`]).

mod error;

use std::path::{Path, PathBuf};

use file_system::{Fs, Kind};

#[cfg(any(test, feature = "test-support"))]
pub use error::diagnostic_samples;
pub use error::{ProjectDirError, EN_US};

/// The directory under an app's directory that holds one directory per
/// project.
const PROJECTS: &str = "projects";

/// The nearest of `start` and its ancestors, nearest first, for which
/// `is_root` is true; `None` where none is, or where `stop` is true of a
/// directory first. `stop` is asked before `is_root`, so the directory it
/// names is itself never a root; a walk that never stops passes
/// `|_| false`.
pub fn nearest_ancestor(
    start: &Path,
    stop: impl Fn(&Path) -> bool,
    is_root: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if stop(dir) {
            return None;
        }
        if is_root(dir) {
            return Some(dir.to_path_buf());
        }
    }
    None
}

/// Whether `dir` holds a directory called `name`. What cannot be looked at
/// (a symlink loop, no permission) is not one.
pub fn holds_dir(fs: &dyn Fs, dir: &Path, name: &str) -> bool {
    fs.metadata(&dir.join(name))
        .is_ok_and(|m| m.kind == Kind::Dir)
}

/// Whether `dir` holds anything called `name`, a file included. What cannot
/// be looked at is not there.
pub fn holds_entry(fs: &dyn Fs, dir: &Path, name: &str) -> bool {
    fs.exists(&dir.join(name)).unwrap_or(false)
}

/// Whether `id` is one plain name that is safe to join onto a directory: not
/// empty, no NUL, not absolute, no path separator (`/` or `\`, on every
/// platform), and not `.` or `..`.
pub fn check_id(id: &str) -> Result<(), ProjectDirError> {
    let own = || id.to_string();
    if id.is_empty() {
        Err(ProjectDirError::IdEmpty)
    } else if id.contains('\0') {
        Err(ProjectDirError::IdNul { id: own() })
    } else if Path::new(id).is_absolute() {
        Err(ProjectDirError::IdAbsolute { id: own() })
    } else if id.contains(['/', '\\']) {
        Err(ProjectDirError::IdSeparator { id: own() })
    } else if id == "." || id == ".." {
        Err(ProjectDirError::IdRelative { id: own() })
    } else {
        Ok(())
    }
}

/// A project's directory: `<home>/<app_dir>/projects/<id>`, where `app_dir`
/// is the application's own directory name (the caller's constant) and `id`
/// is [checked](check_id).
pub fn project_dir(home: &Path, app_dir: &str, id: &str) -> Result<PathBuf, ProjectDirError> {
    check_id(id)?;
    Ok(home.join(app_dir).join(PROJECTS).join(id))
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
