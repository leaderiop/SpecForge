//! What a diagnostics publish sends, computed from the LSP state alone
//! (ADR 0023): placement, conversion and the files to clear, before any
//! client call.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::SourceSpan;
use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag,
    NumberOrString, Range, Url,
};

use crate::LspState;
use crate::navigation::{Ranges, navigator, uri_of};

/// What one publish sends, read from the state alone (so it is tested
/// without a client): each target file's diagnostics, placed and
/// converted, with the document version they were computed against. An
/// empty list clears the file.
#[derive(Debug, Clone, Default)]
pub struct Publication {
    pub files: BTreeMap<Url, FilePublication>,
    /// Where a diagnostic about no entity went: the next publish's anchor.
    pub anchor: Option<Url>,
}

/// One file's part of a [`Publication`].
#[derive(Debug, Clone, Default)]
pub struct FilePublication {
    /// As the client receives them.
    pub diagnostics: Vec<Diagnostic>,
    /// As code actions read them back (spanless ones placed).
    pub placed: Vec<specforge_common::Diagnostic>,
    pub version: Option<i32>,
}

impl Publication {
    /// What the project reports now: a diagnostic on the file its span
    /// names; one without a span about entities at the first one's name,
    /// related information at each other's (ADR 0016); one about none on
    /// `edited`, else on the anchor while it is open, else on the first
    /// open document. Targets: every file with diagnostics, every file
    /// published before, `edited` and `touched`.
    pub fn of(state: &LspState, edited: Option<&Url>, touched: &[Url]) -> Publication {
        let nowhere = edited
            .cloned()
            .or_else(|| state.anchor().and_then(|uri| Url::parse(uri).ok()))
            .or_else(|| {
                state
                    .open_uris()
                    .first()
                    .and_then(|uri| Url::parse(uri).ok())
            });
        let diagnostics = state.session().map(|s| s.diagnostics()).unwrap_or_default();
        let nav = navigator(state);
        let ranges = Ranges::new(state);
        let mut files: BTreeMap<Url, FilePublication> = BTreeMap::new();
        let mut anchor = None;
        for diagnostic in &diagnostics {
            let mut related = Vec::new();
            let placed = match &diagnostic.span {
                Some(_) => diagnostic.clone(),
                None => match place_at_subjects(&ranges, &nav, diagnostic) {
                    Some((at, others)) => {
                        related = others;
                        at
                    }
                    None => diagnostic.clone(),
                },
            };
            let uri = match &placed.span {
                Some(span) => uri_of(state, span.file.as_str()),
                None => match &nowhere {
                    Some(uri) => {
                        anchor = Some(uri.clone());
                        uri.clone()
                    }
                    None => continue,
                },
            };
            let mut lsp = diagnostic_to_lsp(&placed, |span| ranges.range(span));
            if !related.is_empty() {
                lsp.related_information = Some(related);
            }
            let file = files.entry(uri).or_default();
            file.diagnostics.push(lsp);
            file.placed.push(placed);
        }
        let mut targets: BTreeSet<Url> = touched.iter().cloned().collect();
        targets.extend(
            state
                .published_uris()
                .iter()
                .filter_map(|uri| Url::parse(uri).ok()),
        );
        targets.extend(edited.cloned());
        for uri in targets {
            files.entry(uri).or_default();
        }
        for (uri, file) in &mut files {
            file.version = state.document(uri.as_str()).and_then(|d| d.version());
        }
        Publication { files, anchor }
    }
}

/// A spanless diagnostic about entities, placed at the first one's name,
/// and the related information pointing at each other's name. `None`
/// when its data names no entity the graph holds.
fn place_at_subjects<F: Fn(&str) -> Option<String>>(
    ranges: &Ranges,
    nav: &specforge_ops::navigate::Navigator<'_, F>,
    diagnostic: &specforge_common::Diagnostic,
) -> Option<(
    specforge_common::Diagnostic,
    Vec<DiagnosticRelatedInformation>,
)> {
    let subjects = specforge_ops::navigate::subjects(ranges.state().graph(), diagnostic);
    let (first, others) = subjects.split_first()?;
    let name = |node: &specforge_graph::Node| {
        nav.definition(node.id.raw.as_str())
            .map(|d| d.name)
            .unwrap_or_else(|_| node.source_span.clone())
    };
    let placed = specforge_common::Diagnostic {
        span: Some(name(first)),
        ..diagnostic.clone()
    };
    let related = others
        .iter()
        .map(|node| DiagnosticRelatedInformation {
            location: ranges.location(&name(node)),
            message: format!("also about '{}'", node.id.raw),
        })
        .collect();
    Some((placed, related))
}

/// The docs link for `code`, or `None` when the catalog has no entry for it
/// (a third-party code, or anything outside the catalog).
fn docs_href(code: &str) -> Option<Url> {
    specforge_diagnostics::docs_href(code).and_then(|href| Url::parse(&href).ok())
}

/// The code of a define block (ADR 0005): the block registers nothing.
const DEFINE_BLOCK: &str = "W143";

/// A diagnostic as the client receives it; `range_of` converts its span.
pub(crate) fn diagnostic_to_lsp(
    diag: &specforge_common::Diagnostic,
    range_of: impl Fn(&SourceSpan) -> Range,
) -> Diagnostic {
    let range = diag.span.as_ref().map(range_of).unwrap_or_default();
    Diagnostic {
        range,
        code: Some(NumberOrString::String(diag.code.clone())),
        // C4-10: editors can render this as a "view docs" link to the
        // code's section of docs/diagnostics.md.
        code_description: docs_href(&diag.code).map(|href| CodeDescription { href }),
        severity: Some(match diag.severity {
            specforge_common::Severity::Error => DiagnosticSeverity::ERROR,
            specforge_common::Severity::Warning => DiagnosticSeverity::WARNING,
            specforge_common::Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        source: Some("specforge".into()),
        // C4-08: the suggestion is the actionable half of the diagnostic
        // ("did you mean X / do Y") — surface it in the editor instead of
        // dropping it at the LSP boundary.
        message: match &diag.suggestion {
            Some(suggestion) => format!("{}\n\nsuggestion: {suggestion}", diag.message),
            None => diag.message.clone(),
        },
        // A define block is inert code: editors fade it (as for inactive
        // code) instead of only underlining it.
        tags: (diag.code == DEFINE_BLOCK).then(|| vec![DiagnosticTag::UNNECESSARY]),
        // The typed payload, as the diagnostics JSON presents it: a client
        // echoes it back in a code-action request's context.
        data: diag
            .data
            .as_deref()
            .and_then(|data| serde_json::to_value(data).ok()),
        ..Default::default()
    }
}

#[cfg(test)]
mod docs_link_tests {
    use super::*;

    fn href(code: &str) -> Option<String> {
        let diag = specforge_common::Diagnostic::error(code, "message");
        diagnostic_to_lsp(&diag, |_| Range::default())
            .code_description
            .map(|d| d.href.to_string())
    }

    /// C6: the "view docs" link points at the code's anchor in
    /// docs/diagnostics.md on the canonical repository (ADR 0004 D6-b), and
    /// only for codes that have an anchor there.
    #[test]
    fn docs_links_only_codes_with_an_anchor_on_the_canonical_repository() {
        assert_eq!(
            href("E001").as_deref(),
            Some("https://github.com/leaderiop/SpecForge/blob/main/docs/diagnostics.md#e001")
        );
        assert!(
            href("R-RES-005").is_some_and(|h| h.ends_with("#r-res-005")),
            "catalogued registry codes are linked"
        );
        assert_eq!(href("E901"), None, "third-party codes have no anchor");
        assert_eq!(
            href("F011"),
            None,
            "codes outside the catalog have no anchor"
        );
        assert_eq!(
            href("E047"),
            None,
            "retired codes have no anchor of their own"
        );
    }
}
