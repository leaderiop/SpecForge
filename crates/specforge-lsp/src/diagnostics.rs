use specforge_common::{Diagnostic, Severity};
use tower_lsp::lsp_types::{self, DiagnosticSeverity, NumberOrString};

use crate::position;

/// Convert a specforge Diagnostic to an LSP Diagnostic.
pub fn to_lsp_diagnostic(diag: &Diagnostic) -> lsp_types::Diagnostic {
    let severity = match diag.severity() {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
    };

    let related_information = if diag.labels.is_empty() {
        None
    } else {
        let infos: Vec<_> = diag
            .labels
            .iter()
            .filter_map(|label| {
                let location = position::span_to_location(&label.span)?;
                Some(lsp_types::DiagnosticRelatedInformation {
                    location,
                    message: label.message.clone(),
                })
            })
            .collect();
        if infos.is_empty() {
            None
        } else {
            Some(infos)
        }
    };

    lsp_types::Diagnostic {
        range: position::span_to_range(&diag.span),
        severity: Some(severity),
        code: Some(NumberOrString::String(diag.code.to_string())),
        source: Some("specforge".to_string()),
        message: diag.message.clone(),
        related_information,
        ..Default::default()
    }
}
