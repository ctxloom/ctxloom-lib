//! A command's error, said on the error stream.

use std::io::{self, Write};

use angzarr_diagnostic::Diagnostic;
use angzarr_presentation::{write_json, Catalog, Envelope, Format};
use serde::Serialize;

use crate::notices::fix_of;

/// How an error reads under text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportText {
    /// `program: message`, one line.
    Prefixed,
    /// The message alone, as the parser words it (a usage error).
    Verbatim,
}

/// Say `error` on `err`: its envelope as json, or under text its catalog
/// message, prefixed with `program` or verbatim by `text`.
pub fn report_failure(
    program: &str,
    format: Format,
    text: ReportText,
    catalog: &Catalog,
    error: &dyn Diagnostic,
    err: &mut dyn Write,
) -> io::Result<()> {
    match (format, text) {
        (Format::Json, _) => write_json(err, &catalog.envelope(error)),
        (Format::Text, ReportText::Prefixed) => {
            writeln!(err, "{program}: {}", catalog.message(error))
        }
        (Format::Text, ReportText::Verbatim) => write!(err, "{}", catalog.message(error)),
    }
}

/// A failure as JSON: its envelope (`code`, `args`, `message`), then `fix`
/// where the catalog has the `<code>-fix` id (the key left out otherwise),
/// then `refusals`, each reported the same way, in order (the key left out
/// when there are none).
#[derive(Serialize, Debug)]
pub struct Reported {
    #[serde(flatten)]
    envelope: Envelope,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    refusals: Vec<Reported>,
}

impl Reported {
    /// `error` alone: its envelope and its fix.
    pub fn of(catalog: &Catalog, error: &dyn Diagnostic) -> Reported {
        Reported {
            envelope: catalog.envelope(error),
            fix: fix_of(catalog, error),
            refusals: Vec::new(),
        }
    }

    /// `error`, then each of `refusals` as [`of`](Reported::of) says it.
    pub fn with_refusals(
        catalog: &Catalog,
        error: &dyn Diagnostic,
        refusals: &[Box<dyn Diagnostic>],
    ) -> Reported {
        let _ = refusals;
        Reported {
            refusals: Vec::new(),
            ..Reported::of(catalog, error)
        }
    }
}

/// Say `error` and its `refusals` on `err`: as json the [`Reported`]
/// document; under text `program: message` and, where the catalog has one,
/// `  fix: remedy`, for `error` and then for each refusal in order.
pub fn say_failure(
    program: &str,
    format: Format,
    catalog: &Catalog,
    error: &dyn Diagnostic,
    refusals: &[Box<dyn Diagnostic>],
    err: &mut dyn Write,
) -> io::Result<()> {
    match format {
        Format::Json => write_json(err, &Reported::with_refusals(catalog, error, refusals)),
        Format::Text => say_one(program, catalog, error, err),
    }
}

/// `program: message`, then `  fix: remedy` where the catalog has one.
fn say_one(
    program: &str,
    catalog: &Catalog,
    error: &dyn Diagnostic,
    err: &mut dyn Write,
) -> io::Result<()> {
    writeln!(err, "{program}: {}", catalog.message(error))?;
    if let Some(fix) = fix_of(catalog, error) {
        writeln!(err, "  fix: {fix}")?;
    }
    Ok(())
}
