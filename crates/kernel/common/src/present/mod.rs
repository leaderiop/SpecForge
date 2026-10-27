//! Diagnostics as every surface prints them: the one-line text form, the
//! JSON form (with the catalog's title per code), the output cap, the exit
//! code, and the human, source-annotated form the CLI prints
//! ([`render_diagnostics`]) with its summary ([`diagnostic_summary`]) and
//! the tally they and the exit code share ([`Counts`]).

mod annotated;
mod summary;

pub use annotated::render_diagnostics;
pub use summary::{Counts, diagnostic_summary};

use crate::{Diagnostic, DiagnosticData, Severity, SourceSpan};
use serde::Serialize;
use specforge_diagnostics::Codebook;

/// A diagnostic as one heading line, `severity[CODE]: message`, with its
/// suggestion on a `  = help:` line. This is the form for a diagnostic with
/// no span, which has no snippet to show ([`render_diagnostics`] writes it
/// so), and for any command's refusal.
pub fn render_plain(diagnostic: &Diagnostic) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    let mut text = format!("{severity}[{}]: {}", diagnostic.code, diagnostic.message);
    if let Some(suggestion) = &diagnostic.suggestion {
        text.push_str(&format!("\n  = help: {suggestion}"));
    }
    text
}

pub fn format_diagnostic(diag: &Diagnostic) -> String {
    let severity_label = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };

    let location = if let Some(span) = &diag.span {
        format!("{}:{}:{}", span.file, span.start_line, span.start_col)
    } else {
        // Use the diagnostic code as a deterministic fallback so that
        // spanless diagnostics are still identifiable and grep-able.
        format!("<{}>", diag.code)
    };

    let mut output = format!(
        "{}: {}[{}]: {}",
        location, severity_label, diag.code, diag.message
    );

    if let Some(suggestion) = &diag.suggestion {
        output.push_str(&format!("\n  help: {}", suggestion));
    }

    output
}

/// Diagnostics as the JSON every surface prints (`check --format json`,
/// MCP validate, the `specforge://diagnostics` resource, tool `_meta`):
/// one entry per diagnostic with code, the code book's title for it,
/// severity, message, suggestion, the
/// span nested under `span`, and the span's start flat as `file`, `line`
/// and `column`. Absent values are `null`, never missing keys — except
/// `data`, the typed payload, which is there only when the diagnostic
/// carries one (so a diagnostic without it prints as it always has), and
/// `origin`, the extension that reported it, there only for an extension's. The
/// shape is the superset of the nested (CLI) and flat (MCP) shapes the
/// surfaces printed before, so readers of either keep working.
pub fn diagnostics_json<'a>(
    diagnostics: &'a [Diagnostic],
    codes: &'a Codebook,
) -> Vec<DiagnosticJson<'a>> {
    diagnostics
        .iter()
        .map(|d| DiagnosticJson {
            code: &d.code,
            title: codes
                .describes(&d.code, d.origin())
                .map(|entry| entry.title),
            severity: &d.severity,
            message: &d.message,
            span: d.span.as_ref(),
            suggestion: d.suggestion.as_deref(),
            file: d.span.as_ref().map(|s| s.file.as_str()),
            line: d.span.as_ref().map(|s| s.start_line),
            column: d.span.as_ref().map(|s| s.start_col),
            data: d.data.as_deref(),
            origin: d.origin(),
        })
        .collect()
}

/// [`diagnostics_json`] as compact JSON text.
pub fn serialize_diagnostics(diagnostics: &[Diagnostic], codes: &Codebook) -> String {
    serde_json::to_string(&diagnostics_json(diagnostics, codes))
        .expect("diagnostic serialization cannot fail")
}

/// Maximum number of diagnostics to emit before truncating.
/// This prevents runaway output on extremely malformed specs.
pub const MAX_DIAGNOSTICS: usize = 100;

/// Truncate diagnostics to MAX_DIAGNOSTICS, adding a summary if truncated.
pub fn truncate_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    if diagnostics.len() > MAX_DIAGNOSTICS {
        let total = diagnostics.len();
        diagnostics.truncate(MAX_DIAGNOSTICS);
        diagnostics.push(Diagnostic::new(
            crate::codes::I999,
            format!(
                "showing first {} of {} diagnostics — fix these and rerun",
                MAX_DIAGNOSTICS, total
            ),
        ));
    }
}

/// One diagnostic as JSON: see [`diagnostics_json`].
#[derive(Debug, Serialize, crate::shape::Shape)]
#[shape(define = "Diagnostic")]
pub struct DiagnosticJson<'a> {
    pub code: &'a str,
    /// The code book's title for the code, when it describes this diagnostic
    /// (`Codebook::describes`); null for a code it doesn't have (an
    /// uncatalogued one) and for one an extension reported that is another
    /// owner's (W150).
    pub title: Option<&'a str>,
    pub severity: &'a Severity,
    pub message: &'a str,
    /// The location nested: file, start and end line and column.
    pub span: Option<&'a SourceSpan>,
    pub suggestion: Option<&'a str>,
    /// The span's start, flat.
    pub file: Option<&'a str>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// The diagnostic's typed payload ([`DiagnosticData`]); the key is
    /// absent when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<&'a DiagnosticData>,
    /// The extension that reported the diagnostic (a rule's or a pass's);
    /// the key is absent for the host's own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<&'a str>,
}

/// Diagnostics as every surface prints them ([`diagnostics_json`]), owned so
/// a reply can hold them: serializes as the [`DiagnosticJson`] array, titled
/// from the code book it was built with (the project's, so an extension's
/// code is titled as `check --format json` titles it).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiagnosticList(pub Vec<Diagnostic>, Codebook);

impl DiagnosticList {
    /// `diagnostics`, titled from `codes`.
    pub fn new(diagnostics: Vec<Diagnostic>, codes: &Codebook) -> Self {
        DiagnosticList(diagnostics, codes.clone())
    }
}

impl Serialize for DiagnosticList {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        diagnostics_json(&self.0, &self.1).serialize(serializer)
    }
}

impl crate::shape::Shape for DiagnosticList {
    fn body(defs: &mut crate::shape::Definitions) -> serde_json::Value {
        <Vec<DiagnosticJson<'static>> as crate::shape::Shape>::schema(defs)
    }
}

/// One diagnostic as every surface prints it ([`diagnostics_json`]), owned so
/// a document can hold it: serializes as one [`DiagnosticJson`], and its
/// schema refers to the named `Diagnostic` shape.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticDocument(pub Diagnostic, Codebook);

impl DiagnosticDocument {
    /// `diagnostic`, titled from `codes`.
    pub fn new(diagnostic: Diagnostic, codes: &Codebook) -> Self {
        DiagnosticDocument(diagnostic, codes.clone())
    }
}

impl Serialize for DiagnosticDocument {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        diagnostics_json(std::slice::from_ref(&self.0), &self.1)[0].serialize(serializer)
    }
}

impl crate::shape::Shape for DiagnosticDocument {
    fn body(defs: &mut crate::shape::Definitions) -> serde_json::Value {
        <DiagnosticJson<'static> as crate::shape::Shape>::schema(defs)
    }
}

impl From<Vec<Diagnostic>> for DiagnosticList {
    fn from(diagnostics: Vec<Diagnostic>) -> Self {
        DiagnosticList(diagnostics, Codebook::default())
    }
}

/// Compute the process exit code from collected diagnostics.
///
/// Returns 0 if no error-level diagnostics exist, 1 otherwise. Strict
/// mode is not a separate rule: `specforge_project::DiagnosticPolicy`
/// promotes warnings to errors before the exit code is computed.
pub fn compute_exit_code(diagnostics: &[Diagnostic]) -> i32 {
    i32::from(Counts::of(diagnostics).errors > 0)
}
