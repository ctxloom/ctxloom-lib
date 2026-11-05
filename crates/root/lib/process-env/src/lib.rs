//! The once-read snapshot of the process environment.
//!
//! A program reads its environment once, at its composition root:
//! [`EnvSnapshot::from_process`] is the only call into `std::env` the
//! workspace's clippy rules allow (the working directory, the temporary
//! directory and every variable, as the process holds them), and everything
//! else reads the snapshot it is handed. A test builds one from parts
//! ([`EnvSnapshot::of`], [`EnvSnapshot::from_parts`]) rather than setting the
//! process's own variables.
//!
//! The snapshot keeps names and values as the process gives them (`OsString`,
//! not necessarily UTF-8), and every view that wants text says what it does
//! with the rest: [`EnvSnapshot::utf8`] finds none, [`EnvSnapshot::vars_utf8`]
//! skips the pair. Names are compared as the host's flavor compares them
//! (case-insensitively on Windows).

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

use os::Flavor;

/// The working directory as the process had it, or why it had none.
#[derive(Debug)]
enum Cwd {
    Known(PathBuf),
    /// `io::Error` is not `Clone`: its kind, its OS code and its message are
    /// kept, and each [`EnvSnapshot::cwd`] builds the error again.
    Unreadable {
        kind: io::ErrorKind,
        code: Option<i32>,
        message: String,
    },
}

/// The process environment, read once.
#[derive(Debug)]
pub struct EnvSnapshot {
    flavor: Flavor,
    cwd: Cwd,
    temp_dir: PathBuf,
    vars: Vec<(OsString, OsString)>,
}

impl EnvSnapshot {
    /// The process's environment as it is now: the host's flavor, its working
    /// directory (or why it has none), its temporary directory and every
    /// variable. The one place the workspace reads `std::env`.
    // The seam: this is the single permitted call site of the disallowed `std::env` reads.
    #[allow(clippy::disallowed_methods)]
    pub fn from_process() -> EnvSnapshot {
        EnvSnapshot::from_parts(
            os::host().flavor(),
            std::env::current_dir(),
            std::env::temp_dir(),
            std::env::vars_os(),
        )
    }

    /// A snapshot of exactly these parts.
    pub fn from_parts(
        flavor: Flavor,
        cwd: io::Result<PathBuf>,
        temp_dir: PathBuf,
        vars: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> EnvSnapshot {
        EnvSnapshot {
            flavor,
            cwd: match cwd {
                Ok(path) => Cwd::Known(path),
                Err(why) => Cwd::Unreadable {
                    kind: why.kind(),
                    code: why.raw_os_error(),
                    message: why.to_string(),
                },
            },
            temp_dir,
            vars: vars.into_iter().collect(),
        }
    }

    /// A snapshot of these variables on the host's flavor, in a working
    /// directory `.` and a temporary directory `tmp`; [`EnvSnapshot::with_cwd`]
    /// and the other `with_` methods change the rest. For tests.
    pub fn of<N: Into<OsString>, V: Into<OsString>>(
        vars: impl IntoIterator<Item = (N, V)>,
    ) -> EnvSnapshot {
        EnvSnapshot::from_parts(
            os::host().flavor(),
            Ok(PathBuf::from(".")),
            PathBuf::from("tmp"),
            vars.into_iter().map(|(k, v)| (k.into(), v.into())),
        )
    }

    /// The same, in the working directory `cwd`.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> EnvSnapshot {
        self.cwd = Cwd::Known(cwd.into());
        self
    }

    /// The same, with no readable working directory, for `why`.
    pub fn without_cwd(self, why: io::Error) -> EnvSnapshot {
        EnvSnapshot::from_parts(self.flavor, Err(why), self.temp_dir, self.vars)
    }

    /// The same, with `temp_dir` as the temporary directory.
    pub fn with_temp_dir(mut self, temp_dir: impl Into<PathBuf>) -> EnvSnapshot {
        self.temp_dir = temp_dir.into();
        self
    }

    /// The same, following `flavor`'s rules for names and the search path.
    pub fn with_flavor(mut self, flavor: Flavor) -> EnvSnapshot {
        self.flavor = flavor;
        self
    }

    /// The flavor whose rules names and the search path follow.
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    /// The working directory the process had; the error it had instead.
    pub fn cwd(&self) -> io::Result<PathBuf> {
        match &self.cwd {
            Cwd::Known(path) => Ok(path.clone()),
            Cwd::Unreadable {
                code: Some(code), ..
            } => Err(io::Error::from_raw_os_error(*code)),
            Cwd::Unreadable { kind, message, .. } => Err(io::Error::new(*kind, message.clone())),
        }
    }

    /// The temporary directory.
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// The value of the variable `name`, exactly as the process holds it
    /// (empty if set empty). Names compare as the flavor compares them.
    pub fn var(&self, name: &str) -> Option<&OsStr> {
        let key = self.flavor.env_key(name);
        self.vars
            .iter()
            .find(|(candidate, _)| {
                candidate
                    .to_str()
                    .is_some_and(|candidate| self.flavor.env_key(candidate) == key)
            })
            .map(|(_, value)| value.as_os_str())
    }

    /// The variable `name`, unless it is unset or empty.
    pub fn non_empty(&self, name: &str) -> Option<&OsStr> {
        self.var(name).filter(|value| !value.is_empty())
    }

    /// The variable `name` as text: none if it is unset or not UTF-8 (empty
    /// is `""`).
    pub fn utf8(&self, name: &str) -> Option<&str> {
        self.var(name).and_then(OsStr::to_str)
    }

    /// The user's home directory: the flavor's home variable, unless unset
    /// or empty.
    pub fn home(&self) -> Option<&OsStr> {
        self.non_empty(self.flavor.home_var())
    }

    /// `PATH`'s directories in order, as the flavor splits them; none when
    /// `PATH` is unset.
    pub fn search_path(&self) -> Vec<PathBuf> {
        self.var("PATH")
            .map(|path| self.flavor.split_search_path_os(path))
            .unwrap_or_default()
    }

    /// Every variable, as the process held it, in its order.
    pub fn vars(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.vars
            .iter()
            .map(|(name, value)| (name.as_os_str(), value.as_os_str()))
    }

    /// Every variable whose name and value are both UTF-8; a pair that is
    /// not is skipped, never an error.
    pub fn vars_utf8(&self) -> Vec<(String, String)> {
        self.vars
            .iter()
            .filter_map(|(name, value)| {
                Some((name.to_str()?.to_string(), value.to_str()?.to_string()))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
