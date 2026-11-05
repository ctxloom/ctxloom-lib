//! A command's error, said on the error stream.

use std::io::{self, Write};

use angzarr_diagnostic::Diagnostic;
use angzarr_presentation::{write_json, Catalog, Format};

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
