//! A calendar date, `YYYY-MM-DD`, at 00:00 UTC, in std's own types.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SECONDS_PER_DAY: i64 = 86_400;

/// What a text shaped like a date is: not one, a calendar date, or shaped
/// like one and not valid.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Dated {
    /// Not date-shaped (not three `-`-separated runs of digits).
    NotADate,
    /// Date-shaped, and not a calendar date written `YYYY-MM-DD`.
    Invalid,
    /// Seconds from 1970-01-01T00:00:00Z to the date's midnight.
    At(i64),
}

pub(crate) fn read(text: &str) -> Dated {
    let parts: Vec<&str> = text.split('-').collect();
    let digits = |p: &&str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    if parts.len() != 3 || !parts.iter().all(digits) {
        return Dated::NotADate;
    }
    if (parts[0].len(), parts[1].len(), parts[2].len()) != (4, 2, 2) {
        return Dated::Invalid;
    }
    let number = |p: &str| p.parse::<i64>().unwrap_or(0);
    let (year, month, day) = (number(parts[0]), number(parts[1]), number(parts[2]));
    if !(1..=days_in_month(year, month)).contains(&day) {
        return Dated::Invalid;
    }
    Dated::At(days_from_civil(year, month, day) * SECONDS_PER_DAY)
}

/// The days in `month` (1 to 12) of `year`; 0 for any other month.
fn days_in_month(year: i64, month: i64) -> i64 {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The instant `seconds` from the epoch (before it, when negative).
pub(crate) fn at(seconds: i64) -> Option<SystemTime> {
    match u64::try_from(seconds) {
        Ok(after) => UNIX_EPOCH.checked_add(Duration::from_secs(after)),
        Err(_) => UNIX_EPOCH.checked_sub(Duration::from_secs(seconds.unsigned_abs())),
    }
}

#[cfg(test)]
#[path = "date.test.rs"]
mod tests;
