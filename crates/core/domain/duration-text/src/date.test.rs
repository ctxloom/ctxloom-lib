use std::time::{Duration, UNIX_EPOCH};

use super::{at, read, Dated};

#[test]
fn a_date_is_its_utc_midnight() {
    for (text, seconds) in [
        ("1970-01-01", 0),
        ("1969-12-31", -86_400),
        ("2000-02-29", 951_782_400),
        ("2026-01-01", 1_767_225_600),
        ("2026-12-31", 1_798_675_200),
        ("0000-01-01", -62_167_219_200),
        ("9999-12-31", 253_402_214_400),
    ] {
        assert_eq!(read(text), Dated::At(seconds), "{text}");
    }
}

#[test]
fn every_month_ends_on_its_last_day() {
    for (month, last) in (1..=12).zip([31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]) {
        assert!(matches!(
            read(&format!("2026-{month:02}-{last:02}")),
            Dated::At(_)
        ));
        assert!(matches!(read(&format!("2026-{month:02}-01")), Dated::At(_)));
        assert_eq!(
            read(&format!("2026-{month:02}-{:02}", last + 1)),
            Dated::Invalid
        );
        assert_eq!(read(&format!("2026-{month:02}-00")), Dated::Invalid);
    }
    assert_eq!(read("2026-00-10"), Dated::Invalid);
    assert_eq!(read("2026-13-10"), Dated::Invalid);
}

#[test]
fn february_has_a_twenty_ninth_in_leap_years_only() {
    for (year, leap) in [
        ("2024", true),
        ("2000", true),
        ("1900", false),
        ("2100", false),
        ("2026", false),
        ("2023", false),
    ] {
        let valid = matches!(read(&format!("{year}-02-29")), Dated::At(_));
        assert_eq!(valid, leap, "{year}");
    }
}

#[test]
fn only_four_two_two_digits_are_a_date() {
    for text in [
        "2026-1-1",
        "02026-01-01",
        "2026-01-1",
        "26-01-01",
        "2026-001-01",
    ] {
        assert_eq!(read(text), Dated::Invalid, "{text}");
    }
    for text in [
        "",
        "30d",
        "2026",
        "2026-01",
        "2026-01-01-01",
        "-2026-01-01",
        "2026--01",
        "a-b-c",
        "1h-2h-3h",
        "2026-01-01T00:00:00Z",
        "+026-01-01",
    ] {
        assert_eq!(read(text), Dated::NotADate, "{text}");
    }
}

#[test]
fn an_instant_is_that_many_seconds_from_the_epoch() {
    assert_eq!(at(0), Some(UNIX_EPOCH));
    assert_eq!(at(5), Some(UNIX_EPOCH + Duration::from_secs(5)));
    assert_eq!(at(-5), Some(UNIX_EPOCH - Duration::from_secs(5)));
}
