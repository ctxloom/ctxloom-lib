//! The rules table: which role may depend on which, and the exceptions.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;

use crate::graph::Kind;

/// A known departure from the table: the edge from `from` to `to` (of `kind`,
/// or any kind), kept for its `reason`. One that matches no finding is stale.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub kind: Option<Kind>,
    pub reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRules {
    #[serde(default = "default_layout")]
    layout: String,
    #[serde(default)]
    dev_checked: Vec<String>,
    #[serde(default)]
    classes: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    roles: BTreeMap<String, RoleRule>,
    #[serde(default)]
    exception: Vec<Exception>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug)]
struct RoleRule {
    allow: Vec<String>,
    #[serde(default)]
    rule: Option<String>,
}

fn default_layout() -> String {
    "crates".to_string()
}

/// Why a rules file was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum RulesError {
    /// Not TOML, or not the shape of a rules file; the parser's words.
    Syntax(String),
    /// An exception states no reason.
    NoReason { from: String, to: String },
    /// A role allows `@class` and `[classes]` has no such class.
    UnknownClass { role: String, class: String },
    /// `dev_checked` names a role with no rule.
    UnknownDevRole { role: String },
}

impl fmt::Display for RulesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RulesError::Syntax(why) => f.write_str(why),
            RulesError::NoReason { from, to } => {
                write!(f, "the exception {from} -> {to} has no reason")
            }
            RulesError::UnknownClass { role, class } => write!(
                f,
                "the role {role} allows the class @{class}, which is not under [classes]"
            ),
            RulesError::UnknownDevRole { role } => {
                write!(f, "dev_checked names the role {role}, which has no rule")
            }
        }
    }
}

impl std::error::Error for RulesError {}

/// The parsed table.
#[derive(Debug)]
pub struct Rules {
    layout: String,
    dev_checked: BTreeSet<String>,
    classes: BTreeMap<String, Vec<String>>,
    roles: BTreeMap<String, RoleRule>,
    exceptions: Vec<Exception>,
}

impl Rules {
    /// Parse a rules file.
    pub fn parse(text: &str) -> Result<Rules, RulesError> {
        let raw: RawRules = toml::from_str(text).map_err(|e| RulesError::Syntax(e.to_string()))?;
        if let Some(e) = raw.exception.iter().find(|e| e.reason.trim().is_empty()) {
            return Err(RulesError::NoReason {
                from: e.from.clone(),
                to: e.to.clone(),
            });
        }
        for (role, rule) in &raw.roles {
            let unknown = rule
                .allow
                .iter()
                .filter_map(|a| a.strip_prefix('@'))
                .find(|class| !raw.classes.contains_key(*class));
            if let Some(class) = unknown {
                return Err(RulesError::UnknownClass {
                    role: role.clone(),
                    class: class.to_string(),
                });
            }
        }
        if let Some(role) = raw.dev_checked.iter().find(|r| !raw.roles.contains_key(*r)) {
            return Err(RulesError::UnknownDevRole { role: role.clone() });
        }
        Ok(Rules {
            layout: raw.layout,
            dev_checked: raw.dev_checked.into_iter().collect(),
            classes: raw.classes,
            roles: raw.roles,
            exceptions: raw.exception,
        })
    }

    /// The directory the crates live under: `crates` unless the file says.
    pub fn layout(&self) -> &str {
        &self.layout
    }

    /// Whether the table has a rule for `role`.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains_key(role)
    }

    /// What `role` may depend on, as the table spells it; nothing for a role
    /// without a rule.
    pub fn allowed_roles(&self, role: &str) -> &[String] {
        self.roles.get(role).map_or(&[], |r| r.allow.as_slice())
    }

    /// The sentence the table gives for `role`, if any.
    pub fn rule_text(&self, role: &str) -> Option<&str> {
        self.roles.get(role)?.rule.as_deref()
    }

    /// Whether dev-dependency edges from `role` are examined.
    pub fn checks_dev(&self, role: &str) -> bool {
        self.dev_checked.contains(role)
    }

    /// The exceptions, in file order.
    pub fn exceptions(&self) -> &[Exception] {
        &self.exceptions
    }

    /// Whether a package of role `from_role` may depend on the package
    /// `to_name` of role `to_role`: one of its allowed entries is a glob that
    /// matches `to_role`, or an `@class` one of whose name patterns matches
    /// `to_name`.
    pub fn allows(&self, from_role: &str, to_name: &str, to_role: &str) -> bool {
        self.allowed_roles(from_role)
            .iter()
            .any(|entry| match entry.strip_prefix('@') {
                Some(class) => self
                    .classes
                    .get(class)
                    .is_some_and(|patterns| patterns.iter().any(|p| glob(p, to_name))),
                None => glob(entry, to_role),
            })
    }
}

/// Whether `text` matches `pattern`, in which `*` stands for any run of
/// characters (slashes too, and nothing) and every other character for
/// itself.
pub(crate) fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_chars(&p, &t)
}

/// [`glob`] over characters: a star tries every split of the text (patterns
/// are a few characters, so the search is small).
fn glob_chars(pattern: &[char], text: &[char]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some(('*', rest)) => (0..=text.len()).any(|i| glob_chars(rest, &text[i..])),
        Some((c, rest)) => text.first() == Some(c) && glob_chars(rest, &text[1..]),
    }
}

#[cfg(test)]
#[path = "rules.test.rs"]
mod tests;
