//! The grammar, as a table. Each row is an input, what the three parsers it
//! unifies made of it, and what the grammar makes of it.
//!
//! The three: `prune` (a command's `--older-than`, a span or a date), `rs`
//! (a sweep's and a clean's age bound, `d`, `w` or `h` only) and `tags` (the
//! duration literals of a tag query). Each is observed, not
//! remembered: the nanoseconds it gave, or `None` where it refused. A row
//! whose parsers disagree with the grammar (or with each other, where the
//! grammar sides with one) has a `why`; a row with none is a row on which
//! every parser and the grammar agree, so the table is also the list of every
//! change.

use std::cell::Cell;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use angzarr_diagnostic::{Arg, Diagnostic};

use crate::{age_between, cutoff_at, parse, Comparison, DurationError, DurationText};

const NS: i128 = 1_000_000_000;

/// What the grammar makes of an input.
enum Is {
    Secs(u64),
    /// Refused, with the code's suffix.
    Refused(&'static str),
}
use Is::{Refused, Secs};

struct Row {
    text: &'static str,
    prune: Option<i128>,
    rs: Option<i128>,
    tags: Option<i128>,
    grammar: Is,
    /// Why a parser's answer changes, empty when none does.
    why: &'static str,
}

const fn s(n: i128) -> Option<i128> {
    Some(n * NS)
}

const SAME: &str = "";

#[rustfmt::skip]
const TABLE: &[Row] = &[
    // The forms every parser reads.
    Row { text: "30d", prune: s(2_592_000), rs: s(2_592_000), tags: s(2_592_000), grammar: Secs(2_592_000), why: SAME },
    Row { text: "12w", prune: s(7_257_600), rs: s(7_257_600), tags: s(7_257_600), grammar: Secs(7_257_600), why: SAME },
    Row { text: "720h", prune: s(2_592_000), rs: s(2_592_000), tags: s(2_592_000), grammar: Secs(2_592_000), why: SAME },
    Row { text: "7d", prune: s(604_800), rs: s(604_800), tags: s(604_800), grammar: Secs(604_800), why: SAME },
    Row { text: " 30d", prune: s(2_592_000), rs: s(2_592_000), tags: s(2_592_000), grammar: Secs(2_592_000), why: SAME },
    Row { text: "1h ", prune: s(3_600), rs: s(3_600), tags: s(3_600), grammar: Secs(3_600), why: SAME },
    // Zero.
    Row { text: "0d", prune: None, rs: s(0), tags: s(0), grammar: Secs(0), why: "intended: 0d is valid (owner); prune refused it as not positive, so prune's caller keeps that check if it wants it" },
    Row { text: "0s", prune: None, rs: None, tags: s(0), grammar: Secs(0), why: "intended: 0 of a unit is valid" },
    Row { text: "0", prune: None, rs: None, tags: None, grammar: Refused("no-unit"), why: SAME },
    // Units: s m h d w.
    Row { text: "90m", prune: s(5_400), rs: None, tags: s(5_400), grammar: Secs(5_400), why: "intended: rs read only d w h" },
    Row { text: "45s", prune: s(45), rs: None, tags: s(45), grammar: Secs(45), why: "intended: rs read only d w h" },
    // Compound chains, longest to shortest, each unit once.
    Row { text: "1h30m", prune: s(5_400), rs: None, tags: s(5_400), grammar: Secs(5_400), why: "intended: rs read no chain" },
    Row { text: "2w1d", prune: None, rs: None, tags: s(1_296_000), grammar: Secs(1_296_000), why: "intended: prune read a chain only of h m s" },
    Row { text: "1w1d1h1m1s", prune: None, rs: None, tags: s(694_861), grammar: Secs(694_861), why: "intended: prune read a chain only of h m s" },
    Row { text: "1d2w", prune: None, rs: None, tags: None, grammar: Refused("out-of-order"), why: SAME },
    Row { text: "30m1h", prune: s(5_400), rs: None, tags: None, grammar: Refused("out-of-order"), why: "intended: prune read any order (Go's duration)" },
    Row { text: "1h1h", prune: s(7_200), rs: None, tags: None, grammar: Refused("repeated-unit"), why: "intended: prune read a repeated unit" },
    Row { text: "1h30m1h", prune: s(9_000), rs: None, tags: None, grammar: Refused("repeated-unit"), why: "intended: prune read a repeated unit" },
    Row { text: "1h30", prune: None, rs: None, tags: None, grammar: Refused("no-unit"), why: SAME },
    // Fractions.
    Row { text: "1.5h", prune: s(5_400), rs: None, tags: None, grammar: Secs(5_400), why: "intended: fractions permitted (owner); rs and tags refused them" },
    Row { text: "0.5d", prune: None, rs: None, tags: None, grammar: Secs(43_200), why: "intended: fractions permitted; prune read a fraction of h m s only" },
    Row { text: "1.5d", prune: None, rs: None, tags: None, grammar: Secs(129_600), why: "intended: fractions permitted; prune read a fraction of h m s only" },
    Row { text: "1.5w", prune: None, rs: None, tags: None, grammar: Secs(907_200), why: "intended: fractions permitted" },
    Row { text: "1.5m", prune: s(90), rs: None, tags: None, grammar: Secs(90), why: "intended: fractions permitted" },
    Row { text: "1.5h30m", prune: s(7_200), rs: None, tags: None, grammar: Secs(7_200), why: "intended: fractions permitted" },
    Row { text: ".5h", prune: s(1_800), rs: None, tags: None, grammar: Secs(1_800), why: "intended: a leading dot is read (prune and Go read it); rs and tags refused it" },
    Row { text: "1.h", prune: s(3_600), rs: None, tags: None, grammar: Refused("malformed"), why: "intended: a trailing dot is refused (prune and Go read it)" },
    Row { text: "1..5h", prune: None, rs: None, tags: None, grammar: Refused("malformed"), why: SAME },
    Row { text: "1.5.5h", prune: None, rs: None, tags: None, grammar: Refused("no-unit"), why: SAME },
    Row { text: ".", prune: None, rs: None, tags: None, grammar: Refused("malformed"), why: SAME },
    Row { text: ".h", prune: None, rs: None, tags: None, grammar: Refused("malformed"), why: SAME },
    Row { text: "1,5h", prune: None, rs: None, tags: None, grammar: Refused("unknown-unit"), why: SAME },
    // Rounding of a fraction to whole seconds: nearest, ties up.
    Row { text: "0.4s", prune: Some(400_000_000), rs: None, tags: None, grammar: Secs(0), why: "intended: results are whole seconds; prune kept the nanoseconds" },
    Row { text: "0.5s", prune: Some(500_000_000), rs: None, tags: None, grammar: Secs(1), why: "intended: results are whole seconds; prune kept the nanoseconds" },
    Row { text: "1.5s", prune: Some(1_500_000_000), rs: None, tags: None, grammar: Secs(2), why: "intended: results are whole seconds; prune kept the nanoseconds" },
    Row { text: "2.5s", prune: Some(2_500_000_000), rs: None, tags: None, grammar: Secs(3), why: "intended: results are whole seconds; prune kept the nanoseconds" },
    // Bare numbers.
    Row { text: "30", prune: None, rs: None, tags: None, grammar: Refused("no-unit"), why: SAME },
    Row { text: "30 ", prune: None, rs: None, tags: None, grammar: Refused("no-unit"), why: SAME },
    // Whitespace inside.
    Row { text: "30 d", prune: None, rs: None, tags: None, grammar: Refused("whitespace"), why: SAME },
    Row { text: "3 0d", prune: None, rs: None, tags: None, grammar: Refused("whitespace"), why: SAME },
    Row { text: "1 h30m", prune: None, rs: None, tags: None, grammar: Refused("whitespace"), why: SAME },
    Row { text: "1h\t30m", prune: None, rs: None, tags: None, grammar: Refused("whitespace"), why: SAME },
    // Signs.
    Row { text: "-3d", prune: None, rs: None, tags: None, grammar: Refused("negative"), why: SAME },
    Row { text: "-1.5h", prune: None, rs: None, tags: None, grammar: Refused("negative"), why: SAME },
    Row { text: "-0d", prune: None, rs: s(0), tags: None, grammar: Refused("negative"), why: "intended: any minus is refused; rs read -0d as zero" },
    Row { text: "+3d", prune: s(259_200), rs: s(259_200), tags: None, grammar: Refused("malformed"), why: "intended: a plus is not in the grammar (the tags refused it)" },
    // Not finite.
    Row { text: "inf", prune: None, rs: None, tags: None, grammar: Refused("non-finite"), why: SAME },
    Row { text: "Inf", prune: None, rs: None, tags: None, grammar: Refused("non-finite"), why: SAME },
    Row { text: "infinity", prune: None, rs: None, tags: None, grammar: Refused("non-finite"), why: SAME },
    Row { text: "NaN", prune: None, rs: None, tags: None, grammar: Refused("non-finite"), why: SAME },
    Row { text: "infh", prune: None, rs: None, tags: None, grammar: Refused("non-finite"), why: SAME },
    // Overflow.
    Row { text: "99999999999999999999999999d", prune: None, rs: None, tags: None, grammar: Refused("overflow"), why: SAME },
    Row { text: "18446744073709551616s", prune: None, rs: None, tags: None, grammar: Refused("overflow"), why: SAME },
    Row { text: "1e400h", prune: None, rs: None, tags: None, grammar: Refused("unknown-unit"), why: SAME },
    Row { text: "9223372036854775807s", prune: None, rs: None, tags: None, grammar: Secs(9_223_372_036_854_775_807), why: "intended: the limit is the result's (u64 seconds), not nanoseconds in an i64" },
    Row { text: "5000000000000h", prune: None, rs: s(18_000_000_000_000_000), tags: None, grammar: Secs(18_000_000_000_000_000), why: "intended: prune and tags overflowed at nanoseconds" },
    // Units outside s m h d w.
    Row { text: "500ms", prune: Some(500_000_000), rs: None, tags: None, grammar: Refused("unknown-unit"), why: "intended: sub-second units are not an age's (prune alone read ns us ms)" },
    Row { text: "2us", prune: Some(2_000), rs: None, tags: None, grammar: Refused("unknown-unit"), why: "intended: sub-second units are not an age's" },
    Row { text: "3ns", prune: Some(3), rs: None, tags: None, grammar: Refused("unknown-unit"), why: "intended: sub-second units are not an age's" },
    Row { text: "1y", prune: None, rs: None, tags: None, grammar: Refused("unknown-unit"), why: SAME },
    Row { text: "1min", prune: None, rs: None, tags: None, grammar: Refused("unknown-unit"), why: SAME },
    Row { text: "30D", prune: None, rs: None, tags: None, grammar: Refused("unknown-unit"), why: SAME },
    Row { text: "1.5H", prune: None, rs: None, tags: None, grammar: Refused("unknown-unit"), why: SAME },
    Row { text: "1hm", prune: None, rs: None, tags: None, grammar: Refused("unknown-unit"), why: SAME },
    // No number, empty.
    Row { text: "h", prune: None, rs: None, tags: None, grammar: Refused("malformed"), why: SAME },
    Row { text: "d", prune: None, rs: None, tags: None, grammar: Refused("malformed"), why: SAME },
    Row { text: "soon", prune: None, rs: None, tags: None, grammar: Refused("malformed"), why: SAME },
    Row { text: "\u{ff11}d", prune: None, rs: None, tags: None, grammar: Refused("malformed"), why: SAME },
    Row { text: "", prune: None, rs: None, tags: None, grammar: Refused("empty"), why: SAME },
    Row { text: "  ", prune: None, rs: None, tags: None, grammar: Refused("empty"), why: SAME },
];

/// The grammar's answer, as the table states it.
fn answer(text: &str) -> Result<u64, String> {
    parse(text)
        .map(|d| d.as_secs())
        .map_err(|e| e.code().trim_start_matches("duration-text-").to_string())
}

#[test]
fn the_grammar_answers_every_row() {
    for row in TABLE {
        let expected = match row.grammar {
            Secs(n) => Ok(n),
            Refused(code) => Err(code.to_string()),
        };
        assert_eq!(answer(row.text), expected, "{:?}", row.text);
    }
}

/// A row with no `why` is one nothing changes on, and a row with a `why` is
/// one something does: the table is the list of every change.
#[test]
fn a_row_states_a_change_exactly_when_a_parser_changes() {
    for row in TABLE {
        let grammar = match row.grammar {
            Secs(n) => Some(i128::from(n) * NS),
            Refused(_) => None,
        };
        let changes = [row.prune, row.rs, row.tags]
            .iter()
            .any(|old| *old != grammar);
        assert_eq!(changes, !row.why.is_empty(), "{:?}", row.text);
    }
}

#[test]
fn the_last_whole_day_that_fits_is_read() {
    assert_eq!(
        parse("213503982334601d").unwrap().as_secs(),
        18_446_744_073_709_526_400
    );
}

#[test]
fn a_leading_dot_is_read_and_a_trailing_dot_is_not() {
    assert_eq!(parse(".5h"), Ok(Duration::from_secs(1_800)));
    assert_eq!(parse("1.h").unwrap_err().code(), "duration-text-malformed");
}

#[test]
fn units_are_fixed_lengths() {
    for (text, seconds) in [
        ("1s", 1),
        ("1m", 60),
        ("1h", 3_600),
        ("1d", 86_400),
        ("1w", 604_800),
    ] {
        assert_eq!(parse(text), Ok(Duration::from_secs(seconds)), "{text}");
    }
}

#[test]
fn a_fraction_is_rounded_once_over_the_whole_chain() {
    // 0.0001h is 0.36 s and 0.2s is 0.2 s: each rounds to 0 alone, the
    // total, 0.56 s, to 1.
    assert_eq!(parse("0.0001h").unwrap().as_secs(), 0);
    assert_eq!(parse("0.2s").unwrap().as_secs(), 0);
    assert_eq!(parse("0.0001h0.2s").unwrap().as_secs(), 1);
    assert_eq!(parse("0.9999h").unwrap().as_secs(), 3_600);
    assert_eq!(parse("0.0083m").unwrap().as_secs(), 0);
    assert_eq!(parse("0.0084m").unwrap().as_secs(), 1);
}

#[test]
fn digits_past_the_eighteenth_cannot_move_a_rounding() {
    assert_eq!(parse("0.49999999999999999999s").unwrap().as_secs(), 0);
    assert_eq!(parse("0.5000000000000000001s").unwrap().as_secs(), 1);
    assert_eq!(parse("0.000000000000000000001w").unwrap().as_secs(), 0);
}

#[test]
fn the_largest_span_is_u64_seconds() {
    assert_eq!(
        parse("18446744073709551615s"),
        Ok(Duration::from_secs(u64::MAX))
    );
    assert_eq!(
        parse("18446744073709551615.4s"),
        Ok(Duration::from_secs(u64::MAX))
    );
    assert_eq!(
        parse("18446744073709551615.5s").unwrap_err().code(),
        "duration-text-overflow"
    );
    assert_eq!(
        parse("213503982334602d").unwrap_err().code(),
        "duration-text-overflow"
    );
}

#[test]
fn a_refusal_names_the_text_and_what_was_wrong() {
    let args = |text: &str| {
        let e = parse(text).unwrap_err();
        e.args()
            .into_iter()
            .map(|(k, v)| match v {
                Arg::Str(s) => (k.to_string(), s.to_string()),
                other => panic!("{other:?}"),
            })
            .collect::<Vec<_>>()
    };
    let pair = |k: &str, v: &str| (k.to_string(), v.to_string());
    assert_eq!(args("1y"), [pair("text", "1y"), pair("unit", "y")]);
    assert_eq!(args("1h1h"), [pair("text", "1h1h"), pair("unit", "h")]);
    assert_eq!(
        args("30m1h"),
        [pair("text", "30m1h"), pair("unit", "h"), pair("after", "m")]
    );
    assert_eq!(args("-3d"), [pair("text", "-3d")]);
    assert_eq!(args(" "), Vec::<(String, String)>::new());
}

#[test]
fn an_error_is_a_diagnostic_whose_display_is_its_code_and_args() {
    let e: DurationError = parse("1y").unwrap_err();
    assert_eq!(e.to_string(), "duration-text-unknown-unit text=1y unit=y");
}

fn at(seconds: u64, nanos: u32) -> SystemTime {
    UNIX_EPOCH + Duration::new(seconds, nanos)
}

const NOW: fn() -> SystemTime = || at(1_800_000_000, 7);

#[test]
fn a_span_cutoff_is_that_long_before_now_keeping_its_fraction() {
    for (text, seconds) in [
        ("30d", 2_592_000),
        ("12w", 7_257_600),
        ("720h", 2_592_000),
        ("1h30m", 5_400),
        ("0d", 0),
    ] {
        assert_eq!(
            cutoff_at(text, NOW()),
            Ok(at(1_800_000_000 - seconds, 7)),
            "{text}"
        );
    }
}

#[test]
fn a_date_cutoff_is_its_utc_midnight_whatever_now_is() {
    assert_eq!(cutoff_at("2026-01-01", NOW()), Ok(at(1_767_225_600, 0)));
    assert_eq!(
        cutoff_at(" 2026-01-01 ", UNIX_EPOCH),
        Ok(at(1_767_225_600, 0))
    );
    assert_eq!(
        cutoff_at("1969-12-31", NOW()),
        Ok(UNIX_EPOCH - Duration::from_secs(86_400))
    );
}

#[test]
fn a_cutoff_refuses_what_is_neither() {
    let code = |text: &str| cutoff_at(text, NOW()).unwrap_err().code().to_string();
    for (text, expected) in [
        ("2026-13-40", "invalid-date"),
        ("2026-02-29", "invalid-date"),
        ("2026-1-1", "invalid-date"),
        ("2026-01-01T00:00:00Z", "unknown-unit"),
        ("30", "no-unit"),
        ("-3d", "negative"),
        ("", "empty"),
        ("2026-01-01 x", "whitespace"),
        ("soon", "malformed"),
    ] {
        assert_eq!(code(text), format!("duration-text-{expected}"), "{text}");
    }
}

#[test]
fn a_cutoff_before_the_earliest_instant_overflows() {
    let earliest = UNIX_EPOCH - Duration::from_secs(1 << 62);
    let far = format!("{}s", u64::MAX);
    assert_eq!(
        cutoff_at(&far, earliest).unwrap_err().code(),
        "duration-text-overflow"
    );
    assert!(cutoff_at("1s", NOW()).is_ok());
}

#[test]
fn an_age_is_zero_for_a_since_after_now() {
    let now = at(100, 500);
    assert_eq!(age_between(at(40, 0), now), Duration::new(60, 500));
    assert_eq!(age_between(now, now), Duration::ZERO);
    assert_eq!(age_between(at(100, 501), now), Duration::ZERO);
    assert_eq!(age_between(at(200, 0), now), Duration::ZERO);
}

#[test]
fn a_comparison_is_exact_in_whole_seconds() {
    use Comparison::{AtLeast, AtMost, LessThan, MoreThan};
    let span = Duration::from_secs(172_800);
    let on = Duration::from_secs(172_800);
    let on_and_a_half = Duration::new(172_800, 500_000_000);
    let under = Duration::new(172_799, 999_999_999);
    let over = Duration::from_secs(172_801);
    for (op, expected) in [
        (AtLeast, [false, true, true, true]),
        (MoreThan, [false, false, false, true]),
        (LessThan, [true, false, false, false]),
        (AtMost, [true, true, true, false]),
    ] {
        let got = [under, on, on_and_a_half, over].map(|age| op.holds(age, span));
        assert_eq!(got, expected, "{op:?}");
    }
}

/// A factory that counts its reads.
fn counting(reads: &Cell<u32>) -> impl Fn() -> SystemTime + '_ {
    move || {
        reads.set(reads.get() + 1);
        NOW()
    }
}

#[test]
fn an_instance_reads_now_once_per_use() {
    let reads = Cell::new(0);
    let spans = DurationText::new(counting(&reads));
    assert_eq!(reads.get(), 0);
    assert_eq!(spans.parse("1h"), Ok(Duration::from_secs(3_600)));
    assert_eq!(reads.get(), 0, "parse reads no clock");
    assert_eq!(spans.cutoff("1d"), Ok(at(1_800_000_000 - 86_400, 7)));
    assert_eq!(reads.get(), 1);
    assert_eq!(spans.age(at(1_799_999_000, 0)), Duration::new(1_000, 7));
    assert_eq!(reads.get(), 2);
    assert!(spans.age_is(
        at(1_799_999_000, 0),
        Comparison::AtLeast,
        Duration::from_secs(1_000)
    ));
    assert_eq!(reads.get(), 3);
    assert_eq!(spans.now(), NOW());
    assert_eq!(reads.get(), 4);
}

#[test]
fn a_frozen_instance_reads_the_factory_once_and_keeps_that_instant() {
    let reads = Cell::new(0);
    let spans = DurationText::new(counting(&reads));
    let frozen = spans.frozen();
    assert_eq!(reads.get(), 1);
    for _ in 0..3 {
        assert_eq!(frozen.cutoff("1d"), Ok(at(1_800_000_000 - 86_400, 7)));
        assert_eq!(frozen.age(at(1_799_999_000, 0)), Duration::new(1_000, 7));
    }
    assert_eq!(frozen.frozen().now(), NOW());
    assert_eq!(reads.get(), 1);
}

#[test]
fn a_fixed_instant_is_a_factory() {
    let spans = DurationText::new(at(1_000, 0));
    assert_eq!(spans.cutoff("10s"), Ok(at(990, 0)));
    assert_eq!(spans.age(at(400, 0)), Duration::from_secs(600));
}
