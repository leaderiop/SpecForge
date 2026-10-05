use specforge_common::{Diagnostic, DiagnosticData};
use specforge_graph::Graph;

/// A code action to be offered in the editor.
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub entity_id: String,
    pub file: String,
    pub action_kind: String,
    pub title: String,
    pub edit_text: String,
    pub insert_line: usize,
    /// C4-09: when set, the edit REPLACES `start_col..end_col` on
    /// `insert_line` (byte columns) instead of inserting at line start.
    pub replace_cols: Option<(usize, usize)>,
}

/// Locate `needle` as a standalone word inside `content.lines()[line]`
/// (1-based line), returning (line_index, start_col, end_col) in bytes.
fn find_word_on_line(
    content: &str,
    line_1based: usize,
    needle: &str,
) -> Option<(usize, usize, usize)> {
    let idx = line_1based.checked_sub(1)?;
    let line = content.lines().nth(idx)?;
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/';
    let bytes = line.as_bytes();
    let mut search_from = 0usize;
    while let Some(rel) = line[search_from..].find(needle) {
        let start = search_from + rel;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end >= line.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return Some((idx, start, end));
        }
        search_from = start + 1;
    }
    let _ = is_word;
    None
}

/// C4-09: quickfixes derived from the file's own diagnostics. An
/// unresolved reference (E003) or import (E025) whose data names a close
/// match becomes a one-tap rename of the token its data names. Read from
/// the diagnostic's data, never its message or suggestion text.
pub fn code_actions_from_diagnostics(diagnostics: &[Diagnostic], content: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    for diag in diagnostics {
        let Some(span) = &diag.span else {
            continue;
        };
        // The token to replace — the misspelled reference id inside the
        // reference's span, or the import path on the import line — and
        // its replacement.
        let (needle, candidate) = match diag.data.as_deref() {
            Some(DiagnosticData::UnresolvedReference {
                target,
                did_you_mean: Some(candidate),
                ..
            }) => (target, candidate),
            Some(DiagnosticData::UnresolvedImport {
                path,
                did_you_mean: Some(candidate),
            }) => (path, candidate),
            _ => continue,
        };
        let mut found = None;
        for line_1based in span.start_line..=span.end_line.max(span.start_line) {
            if let Some(hit) = find_word_on_line(content, line_1based, needle) {
                found = Some(hit);
                break;
            }
        }
        let Some((line_idx, start_col, end_col)) = found else {
            continue;
        };
        actions.push(CodeAction {
            entity_id: candidate.clone(),
            file: span.file.to_string(),
            action_kind: "quickfix".into(),
            title: format!("Replace with '{candidate}'"),
            edit_text: candidate.clone(),
            insert_line: line_idx + 1,
            replace_cols: Some((start_col, end_col)),
        });
    }
    actions
}

/// Generate "add missing verify" code actions for testable entities in a file
/// that have no verify statements.
pub fn code_actions_missing_verify(
    graph: &Graph,
    file: &str,
    kinds: &specforge_registry::KindRegistry,
) -> Vec<CodeAction> {
    graph
        .nodes_in_file(file)
        .into_iter()
        .filter(|n| specforge_graph::obligations(n).is_empty())
        .filter_map(|n| {
            let kind = kinds
                .get(n.kind.raw.as_str())
                .filter(|entry| entry.supports_verify)?;
            // The kind's first allowed verify kind; unit when it names none.
            let verify_kind = kind
                .allowed_verify_kinds
                .first()
                .map_or("unit", String::as_str);
            let stub = format!("  verify {verify_kind} \"{} — TODO\"", n.id.raw);
            Some(CodeAction {
                entity_id: n.id.raw.to_string(),
                file: n.source_span.file.to_string(),
                action_kind: "quickfix".into(),
                title: format!("Add verify stub for {}", n.id.raw),
                edit_text: stub,
                insert_line: n.source_span.end_line,
                replace_cols: None,
            })
        })
        .collect()
}

/// Stubs for the file's unresolved references (E003) to ids that exist
/// nowhere, one per target: the kind is the one the referring field
/// targets in the FieldRegistry (`target_kind`); none without it. Target,
/// entity and field are the diagnostic's data, never its message.
pub fn code_actions_create_stubs(
    diagnostics: &[Diagnostic],
    graph: &Graph,
    fields: &specforge_registry::FieldRegistry,
    current_file: &str,
) -> Vec<CodeAction> {
    let mut stubbed = std::collections::HashSet::new();
    diagnostics
        .iter()
        .filter_map(|diag| match diag.data.as_deref() {
            Some(DiagnosticData::UnresolvedReference {
                target,
                entity,
                field,
                ..
            }) => Some((target, entity, field)),
            _ => None,
        })
        .filter(|(target, _, _)| graph.node(target).is_none() && stubbed.insert(*target))
        .filter_map(|(target, entity, field)| {
            let node = graph.node(entity)?;
            let target_kind = fields
                .get(node.kind.raw.as_str(), field)
                .and_then(|entry| entry.declared.target_kind.as_deref());
            code_action_create_stub(target, target_kind, current_file)
        })
        .collect()
}

/// Generate a code action to create an entity stub for an unresolved reference.
/// Returns None if `target_kind` is None (can't infer kind without field metadata).
pub fn code_action_create_stub(
    entity_id: &str,
    target_kind: Option<&str>,
    current_file: &str,
) -> Option<CodeAction> {
    let kind = target_kind?;

    let stub = format!("{kind} {entity_id} \"{entity_id}\" {{\n  // TODO: fill in fields\n}}");

    Some(CodeAction {
        entity_id: entity_id.to_string(),
        file: current_file.to_string(),
        action_kind: "refactor".into(),
        title: format!("Create {kind} stub for {entity_id}"),
        edit_text: stub,
        insert_line: usize::MAX, // append to end of file
        replace_cols: None,
    })
}
