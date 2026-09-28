use specforge_common::Diagnostic;
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

/// Extract the `did you mean 'x'?` candidate from a suggestion string.
fn did_you_mean_target(suggestion: &str) -> Option<String> {
    let rest = suggestion.strip_prefix("did you mean '")?;
    let end = rest.find('\'')?;
    (end > 0).then(|| rest[..end].to_string())
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

/// C4-09: quickfixes derived from the file's own diagnostics. E003
/// (unresolved reference) and E025 (import target not found) carry a
/// `did you mean 'x'?` suggestion when the resolver has a close match —
/// turn it into a one-tap rename.
pub fn code_actions_from_diagnostics(diagnostics: &[Diagnostic], content: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    for diag in diagnostics {
        if !matches!(diag.code.as_str(), "E003" | "E025") {
            continue;
        }
        let (Some(suggestion), Some(span)) = (&diag.suggestion, &diag.span) else {
            continue;
        };
        let Some(candidate) = did_you_mean_target(suggestion) else {
            continue;
        };
        // Find the token to replace: for E003 the misspelled reference ID
        // (from the message) inside the entity span; for E025 the import
        // path on the import line.
        let needle = if diag.code == "E003" {
            diag.message
                .split("unresolved reference '")
                .nth(1)
                .and_then(|rest| rest.split('\'').next())
                .map(str::to_string)
        } else {
            diag.message
                .split("import target not found: ")
                .nth(1)
                .map(str::to_string)
        };
        let Some(needle) = needle else {
            continue;
        };
        let mut found = None;
        for line_1based in span.start_line..=span.end_line.max(span.start_line) {
            if let Some(hit) = find_word_on_line(content, line_1based, &needle) {
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
            edit_text: candidate,
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
    testable_kinds: &[&str],
) -> Vec<CodeAction> {
    graph
        .nodes_in_file(file)
        .into_iter()
        .filter(|n| testable_kinds.contains(&n.kind.raw.as_str()))
        .filter(|n| {
            // Check if entity has any verify fields already
            n.fields.get("verify").is_none()
        })
        .map(|n| {
            let stub = format!("  verify unit \"{} — TODO\"", n.id.raw);
            CodeAction {
                entity_id: n.id.raw.to_string(),
                file: n.source_span.file.to_string(),
                action_kind: "quickfix".into(),
                title: format!("Add verify stub for {}", n.id.raw),
                edit_text: stub,
                insert_line: n.source_span.end_line,
                replace_cols: None,
            }
        })
        .collect()
}

/// Generate a code action to add a missing import for an entity that exists
/// in another file.
pub fn code_action_add_import(
    graph: &Graph,
    entity_id: &str,
    current_file: &str,
    spec_root: &str,
) -> Option<CodeAction> {
    let node = graph.node(entity_id)?;
    let source_file = node.source_span.file.as_str();

    // Don't offer import if the entity is in the same file
    if source_file == current_file {
        return None;
    }

    // Convert file path to use-import path: strip spec_root prefix and .spec suffix
    let import_path = source_file
        .strip_prefix(spec_root)
        .unwrap_or(source_file)
        .strip_prefix('/')
        .unwrap_or(source_file)
        .strip_suffix(".spec")
        .unwrap_or(source_file);

    Some(CodeAction {
        entity_id: entity_id.to_string(),
        file: current_file.to_string(),
        action_kind: "quickfix".into(),
        title: format!("Add import for {entity_id}"),
        edit_text: format!("use \"{import_path}\""),
        insert_line: 0,
        replace_cols: None,
    })
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
