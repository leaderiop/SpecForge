/// Semantic token types used in the legend.
/// The order matters — indices are sent over the wire.
pub const TOKEN_TYPES: &[&str] = &[
    "keyword",     // 0: structural keywords (use, define, verify, ref)
    "type",        // 1: entity kind keywords (behavior, feature, event, ...)
    "function",    // 2: entity IDs at declaration site
    "variable",    // 3: identifiers in reference lists
    "property",    // 4: field names
    "string",      // 5: string literals
    "comment",     // 6: comments
    "number",      // 7: numeric literals
    "enumMember",  // 8: verify kinds (unit, integration, property, ...)
];

/// Semantic token modifiers. Bit positions.
pub const TOKEN_MODIFIERS: &[&str] = &[
    "declaration", // bit 0: entity declaration site
    "reference",   // bit 1: reference in a list
];

pub const MOD_DECLARATION: u32 = 1 << 0;
pub const MOD_REFERENCE: u32 = 1 << 1;

/// A classified token for semantic highlighting.
#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub text: String,
    pub token_type: String,
    pub modifiers: u32,
    pub line: usize,
    pub col: usize,
}

/// Classify tokens in source text for semantic highlighting.
/// `entity_kinds` are the registered entity kind keywords.
pub fn classify_tokens(source: &str, entity_kinds: &[&str]) -> Vec<SemanticToken> {
    let structural_keywords = ["use", "define", "ref"];
    let mut tokens = Vec::new();
    let mut in_triple_quote = false;

    // Track parser state for context-aware classification
    let mut in_entity_block = false;
    let mut in_list = false;

    for (line_num, line) in source.lines().enumerate() {
        if in_triple_quote {
            tokens.push(SemanticToken {
                text: line.to_string(),
                token_type: "string".into(),
                modifiers: 0,
                line: line_num,
                col: 0,
            });
            if line.trim().ends_with("\"\"\"") {
                in_triple_quote = false;
            }
            continue;
        }

        let trimmed = line.trim();

        // Comments
        if trimmed.starts_with("//") {
            tokens.push(SemanticToken {
                text: trimmed.to_string(),
                token_type: "comment".into(),
                modifiers: 0,
                line: line_num,
                col: line.find("//").unwrap_or(0),
            });
            continue;
        }

        // Check for triple-quoted string start
        if trimmed.contains("\"\"\"") {
            let before = trimmed.split("\"\"\"").next().unwrap_or("").trim();
            if !before.is_empty() {
                let word = before.split_whitespace().next().unwrap_or("");
                if !word.is_empty() {
                    tokens.push(SemanticToken {
                        text: word.to_string(),
                        token_type: "property".into(),
                        modifiers: 0,
                        line: line_num,
                        col: line.find(word).unwrap_or(0),
                    });
                }
            }
            tokens.push(SemanticToken {
                text: "\"\"\"".to_string(),
                token_type: "string".into(),
                modifiers: 0,
                line: line_num,
                col: line.find("\"\"\"").unwrap_or(0),
            });
            let after_first = &trimmed[trimmed.find("\"\"\"").unwrap() + 3..];
            if !after_first.contains("\"\"\"") {
                in_triple_quote = true;
            }
            continue;
        }

        // Track block boundaries
        if trimmed == "}" {
            in_entity_block = false;
            in_list = false;
            continue;
        }

        // Track list boundaries
        if trimmed.contains(']') {
            in_list = false;
        }

        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }

        let first = words[0];

        // Line starting with "verify"
        if first == "verify" {
            tokens.push(SemanticToken {
                text: "verify".into(),
                token_type: "keyword".into(),
                modifiers: 0,
                line: line_num,
                col: line.find("verify").unwrap_or(0),
            });
            // Second word is the verify kind
            if words.len() >= 2 {
                let kind_word = words[1];
                let tt = "enumMember"; // all verify kinds get enumMember
                tokens.push(SemanticToken {
                    text: kind_word.to_string(),
                    token_type: tt.into(),
                    modifiers: 0,
                    line: line_num,
                    col: line.find(kind_word).unwrap_or(0),
                });
            }
            // The rest is a quoted string — find it
            if let Some(quote_start) = line.find('"') {
                let string_part = &line[quote_start..];
                tokens.push(SemanticToken {
                    text: string_part.to_string(),
                    token_type: "string".into(),
                    modifiers: 0,
                    line: line_num,
                    col: quote_start,
                });
            }
            continue;
        }

        // Structural keywords: use, define, ref
        if structural_keywords.contains(&first) {
            tokens.push(SemanticToken {
                text: first.to_string(),
                token_type: "keyword".into(),
                modifiers: 0,
                line: line_num,
                col: line.find(first).unwrap_or(0),
            });
            // For "use", the rest is a string path
            if first == "use"
                && let Some(quote_start) = line.find('"') {
                    let string_part = &line[quote_start..];
                    tokens.push(SemanticToken {
                        text: string_part.to_string(),
                        token_type: "string".into(),
                        modifiers: 0,
                        line: line_num,
                        col: quote_start,
                    });
            }
            // For "define", the second word is an ID (declaration)
            if first == "define" && words.len() >= 2 {
                tokens.push(SemanticToken {
                    text: words[1].to_string(),
                    token_type: "function".into(),
                    modifiers: MOD_DECLARATION,
                    line: line_num,
                    col: find_word_col(line, words[1], first.len()),
                });
            }
            continue;
        }

        // Entity block header: `kind name "title" {`
        if entity_kinds.contains(&first) {
            tokens.push(SemanticToken {
                text: first.to_string(),
                token_type: "type".into(),
                modifiers: 0,
                line: line_num,
                col: line.find(first).unwrap_or(0),
            });
            // Second word is entity ID (declaration)
            if words.len() >= 2 {
                let id_word = words[1];
                // Skip if it looks like a string (title)
                if !id_word.starts_with('"') {
                    tokens.push(SemanticToken {
                        text: id_word.to_string(),
                        token_type: "function".into(),
                        modifiers: MOD_DECLARATION,
                        line: line_num,
                        col: find_word_col(line, id_word, first.len()),
                    });
                }
            }
            // Title string
            if let Some(quote_start) = line.find('"')
                && let Some(end) = find_quoted_string(line, quote_start) {
                    tokens.push(SemanticToken {
                        text: line[quote_start..=end].to_string(),
                        token_type: "string".into(),
                        modifiers: 0,
                        line: line_num,
                        col: quote_start,
                    });
            }
            if trimmed.ends_with('{') {
                in_entity_block = true;
            }
            continue;
        }

        // Inside entity block — field lines
        if in_entity_block {
            // Check for list opening: `field_name [id1, id2, ...]`
            if let Some(bracket_pos) = trimmed.find('[') {
                let field_name = trimmed[..bracket_pos].trim();
                if !field_name.is_empty() {
                    tokens.push(SemanticToken {
                        text: field_name.to_string(),
                        token_type: "property".into(),
                        modifiers: 0,
                        line: line_num,
                        col: line.find(field_name).unwrap_or(0),
                    });
                }
                in_list = true;
                // Classify identifiers inside brackets on this line
                let after_bracket = &trimmed[bracket_pos + 1..];
                classify_list_items(after_bracket, line, line_num, bracket_pos + 1, &mut tokens);
                if trimmed.contains(']') {
                    in_list = false;
                }
                continue;
            }

            // Continuation of a list (no field name, just identifiers)
            if in_list {
                classify_list_items(trimmed, line, line_num, 0, &mut tokens);
                if trimmed.contains(']') {
                    in_list = false;
                }
                continue;
            }

            // Regular field: `field_name value`
            if !first.starts_with('"') && !first.starts_with('}') {
                // First word is the field name
                tokens.push(SemanticToken {
                    text: first.to_string(),
                    token_type: "property".into(),
                    modifiers: 0,
                    line: line_num,
                    col: line.find(first).unwrap_or(0),
                });
                // If the value is a quoted string
                if let Some(quote_start) = line.find('"') {
                    let string_part = &line[quote_start..];
                    tokens.push(SemanticToken {
                        text: string_part.to_string(),
                        token_type: "string".into(),
                        modifiers: 0,
                        line: line_num,
                        col: quote_start,
                    });
                }
                // If the value is a number
                else if words.len() >= 2
                    && let Some(num_word) = words.get(1)
                    && (num_word.parse::<f64>().is_ok() || num_word.starts_with('-') && num_word[1..].parse::<f64>().is_ok()) {
                            tokens.push(SemanticToken {
                                text: num_word.to_string(),
                                token_type: "number".into(),
                                modifiers: 0,
                                line: line_num,
                                col: find_word_col(line, num_word, first.len()),
                            });
                }
            }
            continue;
        }
    }

    tokens
}

/// Classify identifiers inside a `[...]` reference list segment.
fn classify_list_items(
    segment: &str,
    full_line: &str,
    line_num: usize,
    _segment_offset: usize,
    tokens: &mut Vec<SemanticToken>,
) {
    // Strip trailing ] and split by comma
    let cleaned = segment.replace(']', "");
    for item in cleaned.split(',') {
        let id = item.trim();
        if id.is_empty() || id == "[" {
            continue;
        }
        // Find this identifier's position in the full line
        if let Some(col) = find_identifier_col(full_line, id) {
            tokens.push(SemanticToken {
                text: id.to_string(),
                token_type: "variable".into(),
                modifiers: MOD_REFERENCE,
                line: line_num,
                col,
            });
        }
    }
}

/// Find a word's column position in a line, searching after `after_offset`.
fn find_word_col(line: &str, word: &str, after_offset: usize) -> usize {
    line[after_offset..]
        .find(word)
        .map(|p| p + after_offset)
        .unwrap_or(0)
}

/// Find a quoted string's ending position (returns index of closing quote).
fn find_quoted_string(line: &str, start: usize) -> Option<usize> {
    let rest = &line[start + 1..];
    rest.find('"').map(|p| start + 1 + p)
}

/// Find the column of an identifier in a line (word-boundary aware).
fn find_identifier_col(line: &str, id: &str) -> Option<usize> {
    let mut search_start = 0;
    while let Some(pos) = line[search_start..].find(id) {
        let abs_pos = search_start + pos;
        let before_ok = abs_pos == 0
            || !line.as_bytes()[abs_pos - 1].is_ascii_alphanumeric()
                && line.as_bytes()[abs_pos - 1] != b'_';
        let after_pos = abs_pos + id.len();
        let after_ok = after_pos >= line.len()
            || !line.as_bytes()[after_pos].is_ascii_alphanumeric()
                && line.as_bytes()[after_pos] != b'_';
        if before_ok && after_ok {
            return Some(abs_pos);
        }
        search_start = abs_pos + 1;
    }
    None
}
