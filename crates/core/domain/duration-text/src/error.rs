//! Why a span or a date is refused: one code each.

use std::fmt;

use angzarr_diagnostic::{code, Arg, Diagnostic};

/// Why a text is not a span (or, for [`cutoff`](crate::cutoff), a date).
/// Each variant has its own code and carries the text it refused.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DurationError {
    /// Nothing but whitespace (or nothing).
    Empty,
    /// Whitespace inside the text: `1h 30m`.
    Whitespace { text: String },
    /// A leading minus: `-3d`.
    Negative { text: String },
    /// `inf`, `infinity` or `nan`, whatever case.
    NonFinite { text: String },
    /// Not a number and a unit: no digits (`h`, `.`), a trailing dot
    /// (`1.h`), a plus sign, a second dot (`1..5h`).
    Malformed { text: String },
    /// A number with no unit after it: `30`, or the `30` of `1h30`.
    NoUnit { text: String },
    /// A unit that is not `s`, `m`, `h`, `d` or `w`: `1y`, `500ms`, `1H`.
    UnknownUnit { text: String, unit: String },
    /// A unit written twice: `1h1h`, `1h30m1h`.
    RepeatedUnit { text: String, unit: String },
    /// A unit after a shorter one: `30m1h`.
    OutOfOrder {
        text: String,
        unit: String,
        after: String,
    },
    /// More seconds than a span (u64) or a date (i64 seconds from the epoch)
    /// can hold, or a cutoff before the earliest time `SystemTime` holds.
    Overflow { text: String },
    /// Shaped like a date (`1-2-3`) but not a calendar date written
    /// `YYYY-MM-DD`: `2026-13-40`, `2026-02-29`, `2026-1-1`.
    InvalidDate { text: String },
}

impl fmt::Display for DurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        angzarr_diagnostic::display(self, f)
    }
}

impl Diagnostic for DurationError {
    fn code(&self) -> &str {
        match self {
            Self::Empty => code!("empty"),
            Self::Whitespace { .. } => code!("whitespace"),
            Self::Negative { .. } => code!("negative"),
            Self::NonFinite { .. } => code!("non-finite"),
            Self::Malformed { .. } => code!("malformed"),
            Self::NoUnit { .. } => code!("no-unit"),
            Self::UnknownUnit { .. } => code!("unknown-unit"),
            Self::RepeatedUnit { .. } => code!("repeated-unit"),
            Self::OutOfOrder { .. } => code!("out-of-order"),
            Self::Overflow { .. } => code!("overflow"),
            Self::InvalidDate { .. } => code!("invalid-date"),
        }
    }

    fn args(&self) -> Vec<(&str, Arg)> {
        match self {
            Self::Empty => Vec::new(),
            Self::Whitespace { text }
            | Self::Negative { text }
            | Self::NonFinite { text }
            | Self::Malformed { text }
            | Self::NoUnit { text }
            | Self::Overflow { text }
            | Self::InvalidDate { text } => vec![("text", text.as_str().into())],
            Self::UnknownUnit { text, unit } | Self::RepeatedUnit { text, unit } => {
                vec![
                    ("text", text.as_str().into()),
                    ("unit", unit.as_str().into()),
                ]
            }
            Self::OutOfOrder { text, unit, after } => vec![
                ("text", text.as_str().into()),
                ("unit", unit.as_str().into()),
                ("after", after.as_str().into()),
            ],
        }
    }
}

/// This crate's en-US messages, its name first.
pub const EN_US: (&str, &str) = (
    env!("CARGO_PKG_NAME"),
    include_str!(concat!(env!("OUT_DIR"), "/en-US.ftl")),
);

/// One value of every error, for the catalog test.
#[cfg(any(test, feature = "test-support"))]
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
    let text = || "1h 30m".to_string();
    let unit = || "y".to_string();
    vec![
        Box::new(DurationError::Empty),
        Box::new(DurationError::Whitespace { text: text() }),
        Box::new(DurationError::Negative { text: "-3d".into() }),
        Box::new(DurationError::NonFinite { text: "inf".into() }),
        Box::new(DurationError::Malformed { text: "h".into() }),
        Box::new(DurationError::NoUnit { text: "30".into() }),
        Box::new(DurationError::UnknownUnit {
            text: "1y".into(),
            unit: unit(),
        }),
        Box::new(DurationError::RepeatedUnit {
            text: "1h1h".into(),
            unit: "h".into(),
        }),
        Box::new(DurationError::OutOfOrder {
            text: "30m1h".into(),
            unit: "h".into(),
            after: "m".into(),
        }),
        Box::new(DurationError::Overflow {
            text: "99999999999999999999999999d".into(),
        }),
        Box::new(DurationError::InvalidDate {
            text: "2026-13-40".into(),
        }),
    ]
}

#[cfg(test)]
#[path = "error.test.rs"]
mod tests;
