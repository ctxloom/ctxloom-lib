//! The grammar of a span of time written as text, and of a cutoff written as
//! a span or a date. Pure: it reads no clock, no environment, no file; a
//! caller hands it the instant "now" through a [`Now`] (a closure over its own
//! clock, or a fixed instant), in std's own types ([`Duration`],
//! [`SystemTime`]).
//!
//! # The grammar
//!
//! A span is one or more links, each a number then a unit, written without
//! spaces: `90m`, `1h30m`, `2w1d`, `1.5d`, `.5h`, `0d`.
//!
//! - Units: `s`, `m`, `h`, `d`, `w`, lowercase only. They are fixed lengths:
//!   a day is 24 hours and a week 7 days, whatever the calendar or a
//!   daylight-saving change. No other unit is read (`ms`, `y`, `min`).
//! - A chain runs longest to shortest and uses each unit at most once.
//! - A number is decimal digits, with an optional fraction: `1.5`, `.5` (a
//!   leading dot is read), never `1.` (a trailing dot) or an exponent.
//! - A bare number (`30`, `0`) is refused: a span always has a unit.
//! - No sign (`-3d` and `+3d` are refused), no inner whitespace; whitespace
//!   around the whole text is ignored.
//!
//! # Rounding
//!
//! A span is whole seconds. A fraction is read exactly (up to 18 digits; later
//! digits cannot move a result, because a tie is representable in 18) and the
//! whole chain's total is rounded once, to the nearest second, ties up:
//! `0.5s` is 1 s, `1.5s` 2 s, `2.5s` 3 s, `0.4s` 0 s. The largest span is
//! `u64::MAX` seconds; more is [`DurationError::Overflow`].
//!
//! # A cutoff
//!
//! [`DurationText::cutoff`] reads "older than this span, or before this date":
//! a span is that long before now, and a date `YYYY-MM-DD` is that day's
//! midnight, **UTC** (a date-only literal is never local time). Any other
//! date form (a single-digit month, a time of day, an offset) is refused; none
//! of the parsers this unifies read one.
//!
//! # Now
//!
//! [`DurationText::new`] takes a [`Now`]: a `Fn() -> SystemTime` closure, or a
//! `SystemTime` for a fixed instant. Each method reads it at most once.
//! [`DurationText::frozen`] reads it once and returns an instance whose "now"
//! is that instant, for a query that judges many items against one instant.

mod date;
mod error;

use std::cmp::Ordering;
use std::time::{Duration, SystemTime};

pub use error::{DurationError, EN_US};

#[cfg(any(test, feature = "test-support"))]
pub use error::diagnostic_samples;

/// A fraction's digits that are read: 18.
const FRACTION_DIGITS: usize = 18;
/// One whole unit of the 18-digit fraction scale.
const SCALE: u128 = 10_u128.pow(18);

/// The units, longest first, with their lengths in seconds.
const UNITS: [(char, u128); 5] = [
    ('w', 604_800),
    ('d', 86_400),
    ('h', 3_600),
    ('m', 60),
    ('s', 1),
];

/// Where "now" comes from: a closure over the caller's clock, or a fixed
/// instant. This crate never reads the system clock itself.
pub trait Now {
    fn now(&self) -> SystemTime;
}

impl<F: Fn() -> SystemTime> Now for F {
    fn now(&self) -> SystemTime {
        self()
    }
}

impl Now for SystemTime {
    fn now(&self) -> SystemTime {
        *self
    }
}

/// How an age compares with a span.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Comparison {
    /// `>=`
    AtLeast,
    /// `>`
    MoreThan,
    /// `<`
    LessThan,
    /// `<=`
    AtMost,
}

impl Comparison {
    /// Whether `age` is `self` than `span`, in whole seconds: an age's
    /// fraction of a second is dropped, as spans are whole seconds, so
    /// `2d + 0.5s` is exactly `2d` old for `>`, `>=`, `<` and `<=` alike.
    pub fn holds(self, age: Duration, span: Duration) -> bool {
        let (age, span) = (age.as_secs(), span.as_secs());
        match self {
            Self::AtLeast => age >= span,
            Self::MoreThan => age > span,
            Self::LessThan => age < span,
            Self::AtMost => age <= span,
        }
    }
}

/// The grammar bound to a source of "now".
#[derive(Clone, Debug)]
pub struct DurationText<N> {
    now: N,
}

impl<N: Now> DurationText<N> {
    pub fn new(now: N) -> Self {
        Self { now }
    }

    /// The source's "now", read once.
    pub fn now(&self) -> SystemTime {
        self.now.now()
    }

    /// `text` as a span (reads no clock).
    #[allow(clippy::unused_self)]
    pub fn parse(&self, text: &str) -> Result<Duration, DurationError> {
        parse(text)
    }

    /// `text` as a cutoff: that span before now, or the date's UTC midnight.
    pub fn cutoff(&self, text: &str) -> Result<SystemTime, DurationError> {
        cutoff_at(text, self.now.now())
    }

    /// How long since `since`, at now; zero for a `since` after now.
    pub fn age(&self, since: SystemTime) -> Duration {
        age_between(since, self.now.now())
    }

    /// Whether the age of `since` is `op` `span`, by [`Comparison::holds`].
    pub fn age_is(&self, since: SystemTime, op: Comparison, span: Duration) -> bool {
        op.holds(self.age(since), span)
    }

    /// An instance whose "now" is the one this reads, once, now.
    pub fn frozen(&self) -> DurationText<SystemTime> {
        DurationText::new(self.now.now())
    }
}

/// How long from `since` to `now`: zero when `since` is after `now`.
pub fn age_between(since: SystemTime, now: SystemTime) -> Duration {
    now.duration_since(since).unwrap_or(Duration::ZERO)
}

/// `text` as a cutoff, read at `now`: [`DurationText::cutoff`] for a caller
/// that has the instant in hand.
pub fn cutoff_at(text: &str, now: SystemTime) -> Result<SystemTime, DurationError> {
    let text = text.trim();
    let overflow = || DurationError::Overflow {
        text: text.to_string(),
    };
    match date::read(text) {
        date::Dated::At(seconds) => date::at(seconds).ok_or_else(overflow),
        date::Dated::Invalid => Err(DurationError::InvalidDate {
            text: text.to_string(),
        }),
        date::Dated::NotADate => now.checked_sub(parse(text)?).ok_or_else(overflow),
    }
}

/// `text` as a span of whole seconds.
pub fn parse(text: &str) -> Result<Duration, DurationError> {
    let text = text.trim();
    let owned = || text.to_string();
    if text.is_empty() {
        return Err(DurationError::Empty);
    }
    if text.contains(char::is_whitespace) {
        return Err(DurationError::Whitespace { text: owned() });
    }
    if text.starts_with('-') {
        return Err(DurationError::Negative { text: owned() });
    }
    if non_finite(text) {
        return Err(DurationError::NonFinite { text: owned() });
    }
    let overflow = || DurationError::Overflow { text: owned() };
    let (mut rest, mut seen, mut last, mut total) = (text, [false; 5], None::<usize>, 0_u128);
    while !rest.is_empty() {
        let (whole, fraction, after) =
            number(rest).ok_or_else(|| DurationError::Malformed { text: owned() })?;
        let unit_len = after
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(after.len());
        let (unit, next) = after.split_at(unit_len);
        if unit.is_empty() {
            return Err(DurationError::NoUnit { text: owned() });
        }
        let Some(rank) = UNITS
            .iter()
            .position(|(name, _)| unit.len() == 1 && unit.starts_with(*name))
        else {
            return Err(DurationError::UnknownUnit {
                text: owned(),
                unit: unit.to_string(),
            });
        };
        if seen[rank] {
            return Err(DurationError::RepeatedUnit {
                text: owned(),
                unit: unit.to_string(),
            });
        }
        if let Some(last) = last.filter(|last| matches!(rank.cmp(last), Ordering::Less)) {
            return Err(DurationError::OutOfOrder {
                text: owned(),
                unit: unit.to_string(),
                after: UNITS[last].0.to_string(),
            });
        }
        seen[rank] = true;
        last = Some(rank);
        let scaled = scaled(whole, fraction).ok_or_else(overflow)?;
        total = scaled
            .checked_mul(UNITS[rank].1)
            .and_then(|link| total.checked_add(link))
            .ok_or_else(overflow)?;
        rest = next;
    }
    // Nearest, ties up.
    let seconds = total / SCALE + u128::from(total % SCALE >= SCALE / 2);
    u64::try_from(seconds)
        .map(Duration::from_secs)
        .map_err(|_| overflow())
}

/// Whether `text` is `inf`, `infinity` or `nan` (any case), alone or with a
/// unit letter after it.
fn non_finite(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let core = lower
        .strip_suffix(|c| UNITS.iter().any(|(unit, _)| *unit == c))
        .unwrap_or(&lower);
    matches!(core, "inf" | "infinity" | "nan")
}

/// The number at the front of `text`: its whole digits, its fraction's
/// digits, and the rest. A number has whole digits, or a dot and fraction
/// digits; a dot with no fraction digits is no number.
fn number(text: &str) -> Option<(&str, &str, &str)> {
    let digits = |s: &str| s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (whole, after) = text.split_at(digits(text));
    let Some(after) = after.strip_prefix('.') else {
        return (!whole.is_empty()).then_some((whole, "", after));
    };
    let (fraction, rest) = after.split_at(digits(after));
    (!fraction.is_empty()).then_some((whole, fraction, rest))
}

/// `whole.fraction` times [`SCALE`], the fraction's digits past the
/// eighteenth dropped; none when it does not fit.
fn scaled(whole: &str, fraction: &str) -> Option<u128> {
    let whole: u128 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let kept = &fraction[..fraction.len().min(FRACTION_DIGITS)];
    let fraction: u128 = format!("{kept:0<FRACTION_DIGITS$}").parse().ok()?;
    whole.checked_mul(SCALE)?.checked_add(fraction)
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
