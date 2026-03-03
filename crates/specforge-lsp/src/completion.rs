use specforge_common::{EdgeType, EntityKind};
use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::position;

pub fn completion(backend: &Backend, params: CompletionParams) -> Option<CompletionResponse> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(
        &params.text_document_position.text_document.uri,
    )?;
    let pos = params.text_document_position.position;

    let source = state.sources.get(&file_path)?;

    // 1. Top-level keyword completion (outside any block)
    if let Some(items) = keyword_completions(source, pos.line, pos.character) {
        if !items.is_empty() {
            return Some(CompletionResponse::Array(items));
        }
    }

    // 2. Field name completion (inside a block body, not inside [...])
    if let Some(items) = field_name_completions(source, pos.line, pos.character) {
        if !items.is_empty() {
            return Some(CompletionResponse::Array(items));
        }
    }

    // 3. Entity ID completion (inside a reference list [...])
    let expected_kind = determine_expected_kind(source, pos.line, pos.character);
    let prefix = get_typed_prefix(source, pos.line, pos.character);

    let mut items = Vec::new();

    let candidates: Vec<_> = if let Some(kind) = expected_kind {
        state.graph.nodes_of_kind(kind)
    } else {
        state.graph.nodes().collect()
    };

    for node in candidates {
        let raw_id = node.id.raw().to_string();

        // Filter by prefix
        if !prefix.is_empty() && !raw_id.starts_with(&prefix) {
            continue;
        }

        let detail = node.title.clone().unwrap_or_default();

        items.push(CompletionItem {
            label: raw_id,
            kind: Some(CompletionItemKind::REFERENCE),
            detail: Some(format!("{} — {}", node.kind, detail)),
            ..Default::default()
        });
    }

    Some(CompletionResponse::Array(items))
}

/// Suggest entity keywords when cursor is at top-level (not inside a block).
fn keyword_completions(source: &str, line: u32, col: u32) -> Option<Vec<CompletionItem>> {
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get(line as usize).unwrap_or(&"");
    let before_cursor = if (col as usize) <= current_line.len() {
        &current_line[..col as usize]
    } else {
        current_line
    };

    // Only activate at column 0 or when only whitespace/partial keyword before cursor
    // and we are NOT inside a block (check brace nesting)
    if !before_cursor.trim_start().is_empty()
        && before_cursor
            .trim_start()
            .contains(|c: char| c == '{' || c == '[' || c == '"')
    {
        return None;
    }

    // Check if we're at top level by counting unmatched braces above cursor
    let brace_depth = count_brace_depth(source, line);
    if brace_depth > 0 {
        return None;
    }

    let prefix = get_typed_prefix(source, line, col);
    let mut items = Vec::new();

    for kind in EntityKind::ALL {
        let kw = kind.keyword();
        if !prefix.is_empty() && !kw.starts_with(&prefix) {
            continue;
        }

        let snippet = if kind.is_singleton() {
            format!("{kw} {{\n  $0\n}}")
        } else {
            format!("{kw} ${{1:name}} \"${{2:Title}}\" {{\n  $0\n}}")
        };

        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(format!("{} block", kw)),
            insert_text: Some(snippet),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    // Also suggest `use` keyword
    let use_kw = "use";
    if prefix.is_empty() || use_kw.starts_with(&prefix) {
        items.push(CompletionItem {
            label: use_kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("import statement".to_string()),
            insert_text: Some("use ${1:path}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    Some(items)
}

/// Suggest field names when cursor is inside an entity block body but NOT inside a [...] list.
fn field_name_completions(source: &str, line: u32, col: u32) -> Option<Vec<CompletionItem>> {
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get(line as usize)?;

    // Must be indented (inside a block body)
    let brace_depth = count_brace_depth(source, line);
    if brace_depth < 1 {
        return None;
    }

    // Not inside a [...] list on the current line
    let before_cursor = if (col as usize) <= current_line.len() {
        &current_line[..col as usize]
    } else {
        current_line
    };
    if before_cursor.contains('[') {
        return None;
    }

    // Find enclosing entity kind by walking backward
    let entity_kind = find_enclosing_entity_kind(source, line)?;

    let prefix = get_typed_prefix(source, line, col);
    let fields = fields_for_kind(&entity_kind);

    let items: Vec<CompletionItem> = fields
        .iter()
        .filter(|f| prefix.is_empty() || f.starts_with(prefix.as_str()))
        .map(|f| {
            let snippet = field_snippet(f);
            CompletionItem {
                label: f.to_string(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!("{} field", entity_kind.keyword())),
                insert_text: Some(snippet),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            }
        })
        .collect();

    Some(items)
}

/// Count the brace nesting depth at a given line (0-indexed).
fn count_brace_depth(source: &str, line: u32) -> i32 {
    let mut depth: i32 = 0;
    for (i, l) in source.lines().enumerate() {
        if i >= line as usize {
            break;
        }
        for c in l.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
    }
    depth
}

/// Walk backward from a line to find the enclosing entity block header.
fn find_enclosing_entity_kind(source: &str, line: u32) -> Option<EntityKind> {
    let lines: Vec<&str> = source.lines().collect();
    let mut depth: i32 = 0;

    // Walk backward from the line above cursor to find the block header
    for i in (0..line as usize).rev() {
        let l = lines.get(i)?;
        // Count braces on this line (in reverse — closing adds, opening subtracts)
        for c in l.chars().rev() {
            match c {
                '}' => depth += 1,
                '{' => {
                    depth -= 1;
                    if depth < 0 {
                        // This opening brace is our enclosing block
                        // Extract entity kind keyword from the beginning of this line
                        let trimmed = l.trim();
                        let first_word = trimmed.split_whitespace().next()?;
                        return EntityKind::from_keyword(first_word);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Return field names valid for a given entity kind.
fn fields_for_kind(kind: &EntityKind) -> &'static [&'static str] {
    match kind {
        EntityKind::Behavior => &[
            "contract",
            "invariants",
            "types",
            "ports",
            "constraints",
            "verify",
            "scenario",
            "tests",
        ],
        EntityKind::Feature => &["problem", "solution", "behaviors"],
        EntityKind::Invariant => &[
            "guarantee",
            "enforced_by",
            "risk",
            "verify",
            "tests",
        ],
        EntityKind::Event => &[
            "trigger",
            "payload",
            "channel",
            "consumers",
            "produces",
            "verify",
            "tests",
        ],
        EntityKind::TypeDef => &["fields", "variants"],
        EntityKind::Port => &["direction", "category", "method"],
        EntityKind::Constraint => &[
            "category",
            "priority",
            "metric",
            "constrains",
            "protects",
            "verify",
            "tests",
        ],
        EntityKind::Capability => &[
            "persona",
            "surface",
            "features",
            "verify",
            "scenario",
            "tests",
        ],
        EntityKind::Deliverable => &["capabilities", "libraries"],
        EntityKind::Roadmap => &["status", "features", "deliverables"],
        EntityKind::Library => &["features", "ports", "depends_on"],
        EntityKind::Decision => &["status", "context", "decision", "consequences"],
        EntityKind::FailureMode => &[
            "severity",
            "occurrence",
            "detection",
            "mitigates",
            "protects",
        ],
        EntityKind::Glossary => &["terms"],
        EntityKind::Spec => &["name", "version", "plugins", "providers"],
        EntityKind::Ref => &[],
        EntityKind::Custom(_) => &[],
    }
}

/// Generate an appropriate snippet for a field name.
fn field_snippet(field: &str) -> String {
    match field {
        // Reference list fields
        "invariants" | "behaviors" | "types" | "ports" | "constraints" | "consumers"
        | "produces" | "features" | "capabilities" | "libraries" | "depends_on"
        | "constrains" | "protects" | "mitigates" | "deliverables" | "enforced_by" => {
            format!("{field} [$0]")
        }
        // Text block fields
        "contract" | "guarantee" | "problem" | "solution" | "context" | "decision"
        | "consequences" | "payload" => {
            format!("{field} \"\"\"\n    $0\n  \"\"\"")
        }
        // String list fields
        "tests" => {
            format!("{field} [\"$0\"]")
        }
        // Simple value fields
        _ => format!("{field} $0"),
    }
}

/// Determine the expected entity kind from the field context.
/// Looks backward from cursor to find the field name preceding `[`.
fn determine_expected_kind(source: &str, line: u32, _col: u32) -> Option<EntityKind> {
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get(line as usize)?;

    // Look for a field name at the start of the current or previous lines
    let trimmed = current_line.trim();

    // Check patterns like "behaviors [" or "invariants ["
    let field_name = if let Some(bracket_pos) = trimmed.find('[') {
        let before = trimmed[..bracket_pos].trim();
        Some(before)
    } else if trimmed.ends_with(']') || trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
        // Look at previous lines for field context
        if line > 0 {
            let prev_line = lines.get((line - 1) as usize)?;
            let prev_trimmed = prev_line.trim();
            if let Some(bracket_pos) = prev_trimmed.find('[') {
                let before = prev_trimmed[..bracket_pos].trim();
                Some(before)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let field_name = field_name?;

    // Use EdgeType::from_field_name to determine expected target kind
    let edge_type = EdgeType::from_field_name(field_name)?;
    edge_type.target_kind()
}

/// Extract the prefix being typed at the cursor position.
fn get_typed_prefix(source: &str, line: u32, col: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let current_line = match lines.get(line as usize) {
        Some(l) => l,
        None => return String::new(),
    };

    let col = col as usize;
    if col > current_line.len() {
        return String::new();
    }

    let before_cursor = &current_line[..col];

    // Walk backward to find the start of the identifier
    let start = before_cursor
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);

    before_cursor[start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_completions_at_top_level() {
        let source = "beh";
        let items = keyword_completions(source, 0, 3).unwrap();
        assert!(items.iter().any(|i| i.label == "behavior"));
        // Should not include unrelated keywords
        assert!(!items.iter().any(|i| i.label == "feature"));
    }

    #[test]
    fn keyword_completions_empty_line() {
        let source = "";
        let items = keyword_completions(source, 0, 0).unwrap();
        // Should include all keywords + use
        assert!(items.len() >= 17);
    }

    #[test]
    fn no_keyword_completions_inside_block() {
        let source = "behavior foo \"Foo\" {\n  con";
        let items = keyword_completions(source, 1, 5);
        // Inside a block — should not offer keywords (depth > 0)
        assert!(items.is_none() || items.unwrap().is_empty());
    }

    #[test]
    fn field_name_completions_in_behavior() {
        let source = "behavior foo \"Foo\" {\n  con";
        let items = field_name_completions(source, 1, 5).unwrap();
        assert!(items.iter().any(|i| i.label == "contract"));
        assert!(items.iter().any(|i| i.label == "constraints"));
        // "consumers" is an event field, not a behavior field
        assert!(!items.iter().any(|i| i.label == "consumers"));
    }

    #[test]
    fn field_name_completions_in_feature() {
        let source = "feature bar \"Bar\" {\n  ";
        let items = field_name_completions(source, 1, 2).unwrap();
        assert!(items.iter().any(|i| i.label == "behaviors"));
        assert!(items.iter().any(|i| i.label == "problem"));
        assert!(items.iter().any(|i| i.label == "solution"));
    }

    #[test]
    fn no_field_completions_inside_reference_list() {
        let source = "behavior foo \"Foo\" {\n  behaviors [";
        let items = field_name_completions(source, 1, 14);
        assert!(items.is_none() || items.unwrap().is_empty());
    }

    #[test]
    fn enforced_by_snippet_is_reference_list() {
        let snippet = field_snippet("enforced_by");
        assert_eq!(snippet, "enforced_by [$0]");
    }

    #[test]
    fn no_field_completions_at_top_level() {
        let source = "beh";
        let items = field_name_completions(source, 0, 3);
        assert!(items.is_none());
    }

    #[test]
    fn brace_depth_basic() {
        let source = "behavior foo \"Foo\" {\n  contract \"\"\"test\"\"\"\n}";
        assert_eq!(count_brace_depth(source, 0), 0);
        assert_eq!(count_brace_depth(source, 1), 1);
        assert_eq!(count_brace_depth(source, 2), 1);
    }

    #[test]
    fn find_enclosing_kind() {
        let source = "behavior foo \"Foo\" {\n  \n}";
        assert_eq!(
            find_enclosing_entity_kind(source, 1),
            Some(EntityKind::Behavior)
        );
    }

    #[test]
    fn find_enclosing_kind_feature() {
        let source = "feature bar \"Bar\" {\n  \n}";
        assert_eq!(
            find_enclosing_entity_kind(source, 1),
            Some(EntityKind::Feature)
        );
    }
}
