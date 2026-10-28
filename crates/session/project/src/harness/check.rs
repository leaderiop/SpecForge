//! The ledger's diagnostics (ADR 0287): E083 for a harness value no line records, W163 for a line that is not one,
//! I203 once for a file the host would write differently.

use std::path::Path;

use specforge_common::{Diagnostic, SourceSpan, codes};
use specforge_graph::{FieldForms, Graph};
use specforge_registry::Disk;

use super::Harness;
use super::format::{LEDGER_FILE, Ledger, VERDICT_KEY, digest};
use super::rewrite::{LedgerChange, View};

/// The diagnostics of the ledger at `root` for the harness values `graph` holds: nothing when no extension
/// declares a harness field. The file is read through `disk`, so it is an input of the check. With `source_errors` (a
/// source did not parse) a line whose entity is missing is kept: the entity may be in the file that did not parse.
pub fn check(
    harness: &Harness,
    graph: &Graph,
    forms: &FieldForms,
    root: &Path,
    disk: &dyn Disk,
    source_errors: bool,
) -> Vec<Diagnostic> {
    if harness.is_empty() {
        return Vec::new();
    }
    let bytes = disk.read(&root.join(LEDGER_FILE)).unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes);
    let parsed = Ledger::parse(&text);
    let mut diagnostics: Vec<Diagnostic> = parsed
        .malformed
        .iter()
        .map(|(line, why)| {
            Diagnostic::new(
                codes::W163,
                format!("{LEDGER_FILE} line {line} is not a ledger line: {why}"),
            )
            .with_suggestion("run `specforge harness gc` to write the ledger again")
        })
        .collect();

    for node in graph.nodes() {
        let (path, kind) = (node.id.raw.as_str(), node.kind.raw.as_str());
        // The values the ledger records for the entity: its harness fields, and its verdicts (the first of a rule).
        let mut values: Vec<(String, SourceSpan, String)> = harness
            .fields(kind)
            .map(|field| {
                let span = node
                    .fields
                    .entries()
                    .iter()
                    .find(|e| e.key.as_str() == field)
                    .and_then(|e| e.value_span.clone())
                    .unwrap_or_else(|| node.source_span.clone());
                (field.to_string(), span, format!("harness field '{field}'"))
            })
            .collect();
        let mut seen = Vec::new();
        for verdict in &node.verdicts {
            if !seen.contains(&verdict.rule) {
                seen.push(verdict.rule.clone());
                values.push((
                    format!("{VERDICT_KEY}{}", verdict.rule),
                    verdict.span.clone(),
                    format!("verdict '{}'", verdict.rule),
                ));
            }
        }
        for (key, span, what) in values {
            let Some(value) = harness.value(graph, forms, path, &key) else {
                continue;
            };
            if parsed
                .ledger
                .records(path, &key, &digest(path, &key, &value))
            {
                continue;
            }
            let how = if key.starts_with(VERDICT_KEY) {
                "only set_verdict records one"
            } else {
                "only a command's plan sets it"
            };
            let mut message =
                format!("{what} of {kind} '{path}' has no line in {LEDGER_FILE}: {how}");
            // An orphan line (its entity is gone) that records this value under its own path: a rename by hand.
            let orphan = parsed.ledger.lines().find(|l| {
                l.key == key
                    && graph.node(&l.path).is_none()
                    && l.digest == digest(&l.path, &key, &value)
            });
            if let Some(orphan) = orphan {
                message.push_str(&format!(
                    "; {LEDGER_FILE} records it under '{}': renamed by hand?",
                    orphan.path
                ));
            }
            diagnostics.push(
                Diagnostic::new(codes::E083, message)
                    .with_span(span)
                    .with_suggestion(
                        "restore it from git, or set it through the command that owns it",
                    ),
            );
        }
    }

    // The file the host would write: only a ledger that reads at all is judged.
    if parsed.malformed.iter().all(|(line, _)| *line != 1) && !text.is_empty() {
        let view = View {
            graph,
            forms,
            harness,
            keeps_gone: source_errors,
        };
        let rewritten = parsed
            .ledger
            .rewritten(&view, &view, &LedgerChange::default());
        if parsed.uncanonical || rewritten != parsed.ledger {
            diagnostics.push(
                Diagnostic::new(
                    codes::I203,
                    format!("{LEDGER_FILE} is not the file the host would write"),
                )
                .with_suggestion("run `specforge harness gc`"),
            );
        }
    }
    diagnostics
}
