use crate::document::LineIndex;
use specforge_registry::FieldRegistry;
use tower_lsp::lsp_types::Position;

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

    // First check the current line up to cursor position. `col` arrives as an
    // LSP UTF-16 code-unit offset; the slice needs a byte offset, clamped
    // to the line.
    let current_line = lines[line];
    let scan_end = LineIndex::new(current_line).offset_clamped(Position {
        line: 0,
        character: col as u32,
    });
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
    if matches!(
        first_word,
        "use" | "define" | "verify" | "requires" | "ensures" | "maintains" | "//" | "{" | "}"
    ) {
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

/// Find the entity kind of the enclosing block at the given line (its
/// header line included).
pub fn enclosing_entity_kind(content: &str, line: usize) -> Option<String> {
    enclosing_block(content, line, usize::MAX).map(|(kind, _)| kind)
}

/// The entity block enclosing the cursor at (`line`, UTF-16 `col`): its
/// kind (the first word of the line that opened it) and the cursor's brace
/// depth (1 in the block's own body, more inside a nested clause). `None`
/// at the top level. Braces in strings and comments don't count.
pub fn enclosing_block(content: &str, line: usize, col: usize) -> Option<(String, usize)> {
    let mut depth = 0usize;
    let mut kind: Option<String> = None;
    for (index, text) in content.lines().enumerate().take(line + 1) {
        let end = if index == line {
            LineIndex::new(text).offset_clamped(Position {
                line: 0,
                character: col as u32,
            })
        } else {
            text.len()
        };
        let mut chars = text[..end.min(text.len())].char_indices().peekable();
        let mut in_string = false;
        while let Some((_, c)) = chars.next() {
            match c {
                '\\' if in_string => {
                    chars.next();
                }
                '"' => in_string = !in_string,
                '/' if !in_string && chars.peek().is_some_and(|(_, next)| *next == '/') => break,
                '{' if !in_string => {
                    if depth == 0 {
                        kind = text.split_whitespace().next().map(str::to_string);
                    }
                    depth += 1;
                }
                '}' if !in_string => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        kind = None;
                    }
                }
                _ => {}
            }
        }
    }
    kind.filter(|_| depth > 0).map(|kind| (kind, depth))
}

/// Insert text for `field` as snippet placeholder `n`: a reference list
/// scaffolds its brackets, a string its quotes.
pub fn field_snippet(field: &specforge_registry::FieldRegistryEntry, n: usize) -> String {
    use specforge_registry::ManifestFieldType;
    let name = &field.declared.name;
    match field.field_type {
        ManifestFieldType::ReferenceList | ManifestFieldType::StringList => {
            format!("{name} [${n}]")
        }
        ManifestFieldType::String => format!("{name} \"${n}\""),
        ManifestFieldType::Block => format!("{name} {{\n    ${n}\n  }}"),
        _ => format!("{name} ${n}"),
    }
}

/// Snippet that scaffolds a `kind` block with its required fields.
pub fn keyword_snippet(kind: &str, field_registry: &FieldRegistry) -> String {
    let mut required: Vec<_> = field_registry
        .fields_for_kind(kind)
        .into_iter()
        .filter(|f| f.declared.required)
        .collect();
    required.sort_by(|a, b| a.declared.name.cmp(&b.declared.name));
    let mut snippet = format!("{kind} ${{1:id}} \"${{2:Title}}\" {{\n");
    for (i, field) in required.iter().enumerate() {
        snippet.push_str(&format!("  {}\n", field_snippet(field, i + 3)));
    }
    snippet.push_str("  $0\n}");
    snippet
}

/// Return field names valid for a given entity kind from the FieldRegistry.
pub fn complete_field_names(kind: &str, field_registry: Option<&FieldRegistry>) -> Vec<String> {
    if let Some(reg) = field_registry {
        let fields = reg.fields_for_kind(kind);
        if !fields.is_empty() {
            let mut names: Vec<String> = fields.iter().map(|f| f.declared.name.clone()).collect();
            names.sort();
            return names;
        }
    }
    vec![]
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
