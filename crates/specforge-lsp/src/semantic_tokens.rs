use tower_lsp::lsp_types::*;
use tree_sitter::Parser;

use crate::backend::Backend;
use crate::position;

// Token types indexed by position in this array
const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,   // 0
    SemanticTokenType::TYPE,      // 1
    SemanticTokenType::FUNCTION,  // 2
    SemanticTokenType::VARIABLE,  // 3
    SemanticTokenType::STRING,    // 4
    SemanticTokenType::NUMBER,    // 5
    SemanticTokenType::PROPERTY,  // 6
    SemanticTokenType::NAMESPACE, // 7
];

const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
];

/// Return the semantic token options for server capabilities.
pub fn options() -> SemanticTokensOptions {
    SemanticTokensOptions {
        legend: SemanticTokensLegend {
            token_types: TOKEN_TYPES.to_vec(),
            token_modifiers: TOKEN_MODIFIERS.to_vec(),
        },
        full: Some(SemanticTokensFullOptions::Bool(true)),
        range: None,
        ..Default::default()
    }
}

pub fn semantic_tokens_full(
    backend: &Backend,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(&params.text_document.uri)?;
    let source = state.sources.get(&file_path)?;

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .ok()?;

    let tree = parser.parse(source, None)?;

    let mut tokens = Vec::new();
    collect_tokens(tree.root_node(), source, &mut tokens);

    // Sort by position
    tokens.sort_by_key(|t| (t.0, t.1));

    // Delta-encode
    let mut data = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (line, start, length, token_type, modifiers) in tokens {
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            start - prev_start
        } else {
            start
        };

        data.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: modifiers,
        });

        prev_line = line;
        prev_start = start;
    }

    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data,
    }))
}

/// Collect (line, start, length, token_type, modifiers) tuples from tree-sitter CST.
fn collect_tokens(
    node: tree_sitter::Node,
    source: &str,
    tokens: &mut Vec<(u32, u32, u32, u32, u32)>,
) {
    let kind = node.kind();
    let start = node.start_position();
    let end = node.end_position();
    let line = start.row as u32;
    let col = start.column as u32;

    // Only classify leaf/terminal nodes
    match kind {
        // Entity keywords
        "spec" | "invariant" | "behavior" | "feature" | "event" | "type" | "port" | "ref"
        | "capability" | "deliverable" | "roadmap" | "library" | "glossary" | "decision"
        | "constraint" | "failure_mode" => {
            if node.parent().is_some_and(|p| p.kind().ends_with("_block") || p.kind().starts_with("type_") || p.kind().starts_with("ref_")) {
                let text = &source[node.byte_range()];
                tokens.push((line, col, text.len() as u32, 0, 0)); // keyword
            }
        }

        // Sub-block keywords
        "verify" | "scenario" | "given" | "when" | "then" | "use" | "persona" | "surface"
        | "providers" | "coverage" | "gen" | "term" | "method" => {
            if node.parent().is_some() {
                let text = &source[node.byte_range()];
                tokens.push((line, col, text.len() as u32, 0, 0)); // keyword
            }
        }

        "string" | "triple_quoted_string" => {
            let length = if start.row == end.row {
                (end.column - start.column) as u32
            } else {
                // Multi-line string: just mark the first line
                let line_text = source.lines().nth(start.row).unwrap_or("");
                (line_text.len() - start.column) as u32
            };
            tokens.push((line, col, length, 4, 0)); // string
        }

        "integer" | "date_literal" => {
            let text = &source[node.byte_range()];
            tokens.push((line, col, text.len() as u32, 5, 0)); // number
        }

        "identifier" => {
            let text = &source[node.byte_range()];
            let parent = node.parent();

            if let Some(parent) = parent {
                let parent_kind = parent.kind();

                // Check if this is an entity ID at declaration (named field "id")
                if parent_kind.ends_with("_block") || parent_kind.starts_with("type_") {
                    if node
                        .parent()
                        .and_then(|p| {
                            // Check if this node is the "id" field child
                            let mut cursor = p.walk();
                            for child in p.children_by_field_name("id", &mut cursor) {
                                if child.id() == node.id() {
                                    return Some(());
                                }
                            }
                            None
                        })
                        .is_some()
                    {
                        // Entity ID at declaration
                        tokens.push((line, col, text.len() as u32, 3, 1)); // variable + declaration
                        return;
                    }
                }

                // Field names (key in key_value)
                if parent_kind == "key_value" {
                    if node
                        .parent()
                        .and_then(|p| {
                            let mut cursor = p.walk();
                            for child in p.children_by_field_name("key", &mut cursor) {
                                if child.id() == node.id() {
                                    return Some(());
                                }
                            }
                            None
                        })
                        .is_some()
                    {
                        tokens.push((line, col, text.len() as u32, 6, 0)); // property
                        return;
                    }
                }

                // Type names
                if parent_kind == "type_struct"
                    || parent_kind == "type_union"
                    || parent_kind == "type_params"
                    || parent_kind == "generic_type"
                    || parent_kind == "array_type"
                    || parent_kind == "optional_type"
                    || parent_kind == "type_expr"
                    || parent_kind == "union_variants"
                {
                    tokens.push((line, col, text.len() as u32, 1, 0)); // type
                    return;
                }

                // Type field names
                if parent_kind == "type_field" {
                    if node
                        .parent()
                        .and_then(|p| {
                            let mut cursor = p.walk();
                            for child in p.children_by_field_name("name", &mut cursor) {
                                if child.id() == node.id() {
                                    return Some(());
                                }
                            }
                            None
                        })
                        .is_some()
                    {
                        tokens.push((line, col, text.len() as u32, 6, 0)); // property
                        return;
                    }
                }

                // Method names
                if parent_kind == "method_def" {
                    if node
                        .parent()
                        .and_then(|p| {
                            let mut cursor = p.walk();
                            for child in p.children_by_field_name("name", &mut cursor) {
                                if child.id() == node.id() {
                                    return Some(());
                                }
                            }
                            None
                        })
                        .is_some()
                    {
                        tokens.push((line, col, text.len() as u32, 2, 0)); // function
                        return;
                    }
                }

                // Verify kind
                if parent_kind == "verify_statement" {
                    if node
                        .parent()
                        .and_then(|p| {
                            let mut cursor = p.walk();
                            for child in p.children_by_field_name("kind", &mut cursor) {
                                if child.id() == node.id() {
                                    return Some(());
                                }
                            }
                            None
                        })
                        .is_some()
                    {
                        tokens.push((line, col, text.len() as u32, 1, 0)); // type qualifier
                        return;
                    }
                }

                // Import paths
                if parent_kind == "import_path" || parent_kind == "use_import" {
                    tokens.push((line, col, text.len() as u32, 7, 0)); // namespace
                    return;
                }

                // Default: identifiers in reference lists are variables
                if parent_kind == "identifier_list" || parent_kind == "string_list" {
                    tokens.push((line, col, text.len() as u32, 3, 0)); // variable (reference)
                    return;
                }
            }

            // Fallback for identifiers
            tokens.push((line, col, text.len() as u32, 3, 0)); // variable
        }

        "import_path" => {
            let text = &source[node.byte_range()];
            tokens.push((line, col, text.len() as u32, 7, 0)); // namespace
        }

        "scheme_ref_id" => {
            let text = &source[node.byte_range()];
            tokens.push((line, col, text.len() as u32, 4, 0)); // string.special → string
        }

        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tokens(child, source, tokens);
    }
}
