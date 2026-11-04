//! Where a checker's finding lands: on the value, as a diagnostic of its extension (ADR 0286), and W150 when the code
//! is not one the checker may report.

use std::collections::HashSet;

use specforge_common::{Diagnostic, Severity, SourceSpan, codes};
use specforge_diagnostics::{Level, letter_level, qualify};
use specforge_protocol_types::CheckFinding;
use specforge_registry::{CheckCall, RegistryBuild};

use super::Value;

/// Whether any of `findings` is an error, by its code's letter.
pub(super) fn any_error(findings: &[CheckFinding]) -> bool {
    findings
        .iter()
        .any(|finding| letter_level(&finding.code) == Some(Level::Error))
}

/// The level a finding's code states; a code with no letter a level has is a warning, and W150 says so.
fn level(code: &str) -> Level {
    letter_level(code).unwrap_or(Level::Warning)
}

/// `finding` as its extension's diagnostic on `value`: inside its quotes when the finding names bytes that map.
pub(super) fn finding(
    registries: &RegistryBuild,
    call: &CheckCall,
    finding: &CheckFinding,
    value: &Value<'_>,
    one_of: Option<&str>,
) -> Diagnostic {
    let severity = match level(&finding.code) {
        Level::Error => Severity::Error,
        Level::Info => Severity::Info,
        _ => Severity::Warning,
    };
    let book = registries.codes();
    let mut reported = Diagnostic::from_extension(
        &call.extension,
        qualify(&call.extension, &finding.code),
        severity,
        finding.message.clone(),
    );
    reported.machine = book.machine(&call.extension, &finding.code);
    reported.suggestion = finding.help.clone().or(one_of.map(str::to_string));
    value.places(&mut reported, span_of(finding, value));
    reported
}

fn span_of(finding: &CheckFinding, value: &Value<'_>) -> Option<SourceSpan> {
    let span = value.span.clone()?;
    match (finding.at, value.inner) {
        (Some([start, end]), Some((line, column))) if start <= end && end <= value.text.len() => {
            Some(SourceSpan {
                start_line: line,
                start_col: column + start,
                end_line: line,
                end_col: column + end,
                ..span
            })
        }
        _ => Some(span),
    }
}

/// W150 when `call`'s checker may not report `code`: not among its declared `codes`, or not its extension's. Once per
/// extension and code in a run; the finding itself stays, under the extension's namespace.
pub(super) fn misuse(
    registries: &RegistryBuild,
    call: &CheckCall,
    code: &str,
    seen: &mut HashSet<(String, String)>,
) -> Option<Diagnostic> {
    let declared = registries
        .checks()
        .checker(&call.extension, &call.checker)
        .is_some_and(|checker| checker.codes.iter().any(|c| c == code));
    let why = match registries.codes().check(&call.extension, code, level(code)) {
        Err(misuse) => Some((misuse.to_string(), Some(misuse.suggestion()))),
        Ok(()) if !declared => Some(("its `codes` does not list it".to_string(), None)),
        Ok(()) => None,
    };
    let (why, suggestion) = why?;
    if !seen.insert((call.extension.clone(), code.to_string())) {
        return None;
    }
    let mut diagnostic = Diagnostic::new(
        codes::W150,
        format!(
            "extension '{}' checker '{}' reported '{code}': {why}",
            call.extension, call.checker
        ),
    );
    if let Some(suggestion) = suggestion {
        diagnostic = diagnostic.with_suggestion(suggestion);
    }
    Some(diagnostic)
}
