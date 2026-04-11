use specforge_graph::Graph;
use specforge_registry::FieldRegistry;

/// A completion item returned to the editor.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
}

/// Context about the cursor position within a .spec file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorContext {
    /// The entity kind of the enclosing block (e.g. "behavior").
    pub entity_kind: String,
    /// The field name whose reference list the cursor is inside (e.g. "invariants").
    pub field_name: String,
}

/// Detect whether the cursor at (line, col) is inside a `[...]` reference list.
///
/// Returns `Some(CursorContext)` with the enclosing entity kind and field name,
/// or `None` if the cursor is not inside a reference list.
///
/// Detection strategy:
/// 1. Scan backwards from cursor line to find an unmatched `[` (not closed by `]`).
/// 2. On the line containing `[`, extract the field name preceding it.
/// 3. Scan further back to find the entity block header (`kind name "title" {`).
pub fn cursor_context(content: &str, line: usize, col: usize) -> Option<CursorContext> {
    let lines: Vec<&str> = content.lines().collect();
    if line >= lines.len() {
        return None;
    }

    // 1. Check if we're inside [...] by scanning backwards for unmatched '['
    let mut bracket_depth: i32 = 0;
    let mut bracket_line: Option<usize> = None;

    // First check the current line up to cursor position
    let current_line = lines[line];
    let scan_end = col.min(current_line.len());
    for ch in current_line[..scan_end].chars().rev() {
        match ch {
            ']' => bracket_depth += 1,
            '[' => {
                if bracket_depth == 0 {
                    bracket_line = Some(line);
                    break;
                }
                bracket_depth -= 1;
            }
            _ => {}
        }
    }

    // If not found on current line, scan previous lines
    if bracket_line.is_none() {
        for l in (0..line).rev() {
            for ch in lines[l].chars().rev() {
                match ch {
                    ']' => bracket_depth += 1,
                    '[' => {
                        if bracket_depth == 0 {
                            bracket_line = Some(l);
                            break;
                        }
                        bracket_depth -= 1;
                    }
                    _ => {}
                }
            }
            if bracket_line.is_some() {
                break;
            }
            // If we hit a `}` or entity header, stop searching
            let trimmed = lines[l].trim();
            if trimmed == "}" || trimmed.ends_with('{') {
                return None;
            }
        }
    }

    let bracket_line = bracket_line?;

    // 2. Extract field name: the word before `[` on the bracket line
    let bl = lines[bracket_line];
    let bracket_pos = bl.find('[')?;
    let before_bracket = bl[..bracket_pos].trim_end();
    let field_name = before_bracket.split_whitespace().last()?;

    // 3. Find the entity block header by scanning backwards from bracket_line
    for l in (0..=bracket_line).rev() {
        let trimmed = lines[l].trim();
        // Match entity header: `kind name` or `kind name "title"` followed by `{`
        // The `{` may be on the same line or a subsequent line
        if let Some(entity_kind) = parse_entity_header(trimmed) {
            return Some(CursorContext {
                entity_kind,
                field_name: field_name.to_string(),
            });
        }
    }

    None
}

/// Try to parse an entity block header line, returning the entity kind.
/// Matches patterns like:
///   `behavior parse_spec "Parse Spec" {`
///   `type MyType {`
fn parse_entity_header(line: &str) -> Option<String> {
    // Must contain `{` (entity block opening)
    if !line.contains('{') {
        return None;
    }
    // Skip use/define/verify/requires/ensures/maintains lines
    let first_word = line.split_whitespace().next()?;
    if matches!(first_word, "use" | "define" | "verify" | "requires" | "ensures" | "maintains" | "//" | "{" | "}") {
        return None;
    }
    // The first word is the entity kind, second is the ID
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() >= 2 {
        Some(first_word.to_string())
    } else {
        None
    }
}

/// Find the entity kind of the enclosing block at the given line.
/// Scans backwards from `line` looking for an entity header pattern.
pub fn enclosing_entity_kind(content: &str, line: usize) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    for l in (0..=line.min(lines.len().saturating_sub(1))).rev() {
        let trimmed = lines[l].trim();
        if let Some(kind) = parse_entity_header(trimmed) {
            return Some(kind);
        }
        // Stop at file-level scope (closing brace at col 0, not indented)
        if lines[l].starts_with('}') && l < line {
            return None;
        }
    }
    None
}

/// Suggest entity IDs matching a prefix.
pub fn complete_entity_ids(graph: &Graph, prefix: &str) -> Vec<CompletionItem> {
    complete_entity_ids_filtered(graph, prefix, None)
}

/// Suggest entity IDs matching a prefix, optionally filtered by target kind.
pub fn complete_entity_ids_filtered(
    graph: &Graph,
    prefix: &str,
    target_kind: Option<&str>,
) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = graph
        .nodes()
        .into_iter()
        .filter(|n| n.id.raw.as_str().starts_with(prefix))
        .filter(|n| target_kind.is_none_or(|k| n.kind.raw == k))
        .map(|n| CompletionItem {
            id: n.id.raw.to_string(),
            kind: n.kind.raw.to_string(),
            title: n.title.clone(),
        })
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items
}

/// Return field names valid for a given entity kind.
/// Queries the FieldRegistry first; falls back to hardcoded defaults when empty.
pub fn complete_field_names(kind: &str, field_registry: Option<&FieldRegistry>) -> Vec<String> {
    if let Some(reg) = field_registry {
        let fields = reg.fields_for_kind(kind);
        if !fields.is_empty() {
            let mut names: Vec<String> = fields.iter().map(|f| f.field_name.clone()).collect();
            names.sort();
            return names;
        }
    }
    complete_field_names_fallback(kind)
}

/// Hardcoded field name defaults for common entity kinds.
fn complete_field_names_fallback(kind: &str) -> Vec<String> {
    match kind {
        "behavior" => vec![
            "contract".into(),
            "verify".into(),
            "invariants".into(),
            "types".into(),
            "ports".into(),
            "produces".into(),
            "consumes".into(),
            "requires".into(),
            "ensures".into(),
        ],
        "type" => vec!["fields".into(), "verify".into()],
        "feature" => vec!["problem".into(), "solution".into(), "behaviors".into()],
        "invariant" => vec![
            "guarantee".into(),
            "enforced_by".into(),
            "risk".into(),
            "verify".into(),
        ],
        "event" => vec![
            "trigger".into(),
            "payload".into(),
            "channel".into(),
            "consumers".into(),
            "verify".into(),
        ],
        "port" => vec![
            "direction".into(),
            "methods".into(),
            "verify".into(),
        ],
        _ => vec![],
    }
}

/// Return keyword completions including structural keywords and registered entity kinds.
pub fn complete_keywords(registered_kinds: &[&str]) -> Vec<String> {
    let mut keywords: Vec<String> = vec!["use".into(), "define".into()];
    for kind in registered_kinds {
        if *kind != "use" && *kind != "define" {
            keywords.push(kind.to_string());
        }
    }
    keywords.sort();
    keywords.dedup();
    keywords
}
