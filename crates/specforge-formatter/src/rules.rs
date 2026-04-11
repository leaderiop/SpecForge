use crate::config::FormatConfig;

/// A formatting decision for a whitespace region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhitespaceDecision {
    /// Keep the original whitespace as-is.
    Keep,
    /// Replace with the given string.
    Replace(String),
}

/// Normalize indentation: replace leading whitespace with `indent_str * depth`.
pub fn apply_indent(line: &str, depth: usize, config: &FormatConfig) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return String::new();
    }
    let indent = config.indent_str().repeat(depth);
    format!("{indent}{trimmed}")
}

/// Normalize spacing: collapse multiple spaces between tokens to single space.
/// Preserves leading indentation (only collapses within the line content).
pub fn normalize_spacing(line: &str) -> String {
    let leading_ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = &line[leading_ws.len()..];
    if rest.is_empty() {
        return leading_ws;
    }

    let mut result = leading_ws;
    let mut in_string = false;
    let mut in_triple_string = false;
    let mut prev_was_space = false;
    let mut chars = rest.chars().peekable();

    while let Some(ch) = chars.next() {
        // Track triple-quoted strings
        if ch == '"' && !in_triple_string {
            let rest_starts_with_triple = chars.clone().take(2).collect::<String>() == "\"\"";
            if rest_starts_with_triple && !in_string {
                in_triple_string = true;
                result.push(ch);
                prev_was_space = false;
                continue;
            }
            if !in_triple_string {
                in_string = !in_string;
            }
        }

        if in_triple_string {
            result.push(ch);
            if ch == '"' {
                let rest_ends_with_triple = chars.clone().take(2).collect::<String>() == "\"\"";
                if rest_ends_with_triple {
                    // Consume the next two quotes
                    result.push(chars.next().unwrap());
                    result.push(chars.next().unwrap());
                    in_triple_string = false;
                }
            }
            prev_was_space = false;
            continue;
        }

        if in_string {
            result.push(ch);
            if ch == '\\'
                && let Some(next) = chars.next()
            {
                result.push(next);
            }
            prev_was_space = false;
            continue;
        }

        if ch == ' ' || ch == '\t' {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(ch);
            prev_was_space = false;
        }
    }

    result
}

/// Normalize comment spacing: ensure `// ` has exactly one space after `//`.
pub fn normalize_comment(line: &str) -> String {
    let trimmed = line.trim_start();
    let leading_ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();

    if let Some(rest) = trimmed.strip_prefix("//") {
        let rest_trimmed = rest.trim_start();
        if rest_trimmed.is_empty() {
            format!("{leading_ws}//")
        } else {
            format!("{leading_ws}// {rest_trimmed}")
        }
    } else {
        line.to_string()
    }
}

/// Sort use-import lines alphabetically.
/// Returns lines reordered so that consecutive `use` blocks are sorted.
pub fn sort_imports(lines: &[String]) -> Vec<String> {
    let mut result = Vec::with_capacity(lines.len());
    let mut import_group: Vec<String> = Vec::new();
    let mut import_start = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
            if import_start.is_none() {
                import_start = Some(i);
            }
            import_group.push(line.clone());
        } else {
            if !import_group.is_empty() {
                import_group.sort();
                result.append(&mut import_group);
                import_start = None;
            }
            result.push(line.clone());
        }
    }

    if !import_group.is_empty() {
        import_group.sort();
        result.extend(import_group);
    }

    result
}

/// Determine the indentation depth of a CST node kind.
/// Returns how many levels deep the node should be indented.
pub fn depth_for_node(kind: &str, parent_kind: Option<&str>) -> usize {
    match kind {
        "source_file" => 0,
        "entity_block" | "spec_block" | "ref_full" | "ref_inline" | "define_block"
        | "union_block" | "use_import" | "pub_use_import" => 0,
        "field" | "verify_statement" => {
            match parent_kind {
                Some("nested_block") => 2, // nested blocks get extra indent
                _ => 1,
            }
        }
        "nested_block" => 1,
        _ => {
            if parent_kind == Some("nested_block") {
                2
            } else if parent_kind.is_some_and(|p| {
                matches!(
                    p,
                    "entity_block" | "spec_block" | "ref_full" | "define_block"
                )
            }) {
                1
            } else {
                0
            }
        }
    }
}

/// Calculate alignment column for field values within a block.
/// Returns the column at which values should start for aligned output.
pub fn alignment_column(field_keys: &[&str]) -> usize {
    field_keys.iter().map(|k| k.len()).max().unwrap_or(0) + 1
}

/// Align a field line: `key value` becomes `key<spaces>value` where value starts at `col`.
pub fn align_field(line: &str, col: usize, indent: &str) -> String {
    let trimmed = line.trim();

    // Don't align non-field lines
    if trimmed.is_empty()
        || trimmed.starts_with("//")
        || trimmed.starts_with('{')
        || trimmed.starts_with('}')
        || trimmed.starts_with("verify")
        || trimmed.starts_with('[')
        || trimmed.starts_with('"')
    {
        return line.to_string();
    }

    // Split into key and rest
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let key = match parts.next() {
        Some(k) => k,
        None => return line.to_string(),
    };
    let rest = match parts.next() {
        Some(r) => r.trim_start(),
        None => return line.to_string(),
    };

    if rest.is_empty() {
        return line.to_string();
    }

    let padding = if key.len() < col {
        " ".repeat(col - key.len())
    } else {
        " ".to_string()
    };

    format!("{indent}{key}{padding}{rest}")
}

/// Decide whether a list should be wrapped to multi-line.
/// Returns true if the single-line representation exceeds max_width.
pub fn should_wrap_list(items: &[&str], current_indent: usize, config: &FormatConfig) -> bool {
    // Calculate single-line length: [item1, item2, item3]
    let single_line_len = items.iter().map(|i| i.len()).sum::<usize>()
        + (items.len().saturating_sub(1)) * 2 // ", " separators
        + 2 // brackets
        + current_indent;

    single_line_len > config.max_width
}

/// Format a list as multi-line with proper indentation.
pub fn format_list_multiline(items: &[&str], indent: &str, item_indent: &str) -> String {
    let mut result = String::from("[\n");
    for item in items {
        result.push_str(item_indent);
        result.push_str(item);
        result.push(',');
        result.push('\n');
    }
    result.push_str(indent);
    result.push(']');
    result
}

/// Normalize triple-quoted string indentation.
/// Ensures the content lines are indented relative to the opening `"""`.
pub fn normalize_triple_string(text: &str, base_indent: &str) -> String {
    if !text.starts_with("\"\"\"") || !text.ends_with("\"\"\"") {
        return text.to_string();
    }

    let inner = &text[3..text.len() - 3];
    let lines: Vec<&str> = inner.lines().collect();

    if lines.is_empty() {
        return text.to_string();
    }

    // Find minimum indentation of non-empty lines (skip first line which is on the """ line)
    let min_indent = lines
        .iter()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    let content_indent = format!("{base_indent}  ");
    let mut result = String::from("\"\"\"");

    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            // First line stays on the same line as opening """
            result.push_str(line);
        } else if line.trim().is_empty() {
            // Empty lines
            result.push('\n');
        } else {
            result.push('\n');
            let stripped = if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line.trim_start()
            };
            result.push_str(&content_indent);
            result.push_str(stripped);
        }
    }

    result.push('\n');
    result.push_str(base_indent);
    result.push_str("\"\"\"");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "indentation rules normalize to configured indent style")]
    #[test]
    fn test_indent_normalizes_to_configured_width() {
        let config = FormatConfig { indent_width: 2, use_tabs: false, max_width: 100 };
        assert_eq!(apply_indent("    contract \"test\"", 1, &config), "  contract \"test\"");
        assert_eq!(apply_indent("\tcontract \"test\"", 1, &config), "  contract \"test\"");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "indentation rules normalize to configured indent style")]
    #[test]
    fn test_indent_with_tabs() {
        let config = FormatConfig { indent_width: 4, use_tabs: true, max_width: 100 };
        assert_eq!(apply_indent("  contract \"test\"", 1, &config), "\tcontract \"test\"");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "spacing rules normalize single spaces between tokens")]
    #[test]
    fn test_normalize_spacing_collapses_multiple_spaces() {
        assert_eq!(normalize_spacing("  contract   \"test\""), "  contract \"test\"");
        assert_eq!(normalize_spacing("  key    value"), "  key value");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "spacing rules normalize single spaces between tokens")]
    #[test]
    fn test_normalize_spacing_preserves_string_content() {
        assert_eq!(
            normalize_spacing("  contract \"multiple   spaces   inside\""),
            "  contract \"multiple   spaces   inside\""
        );
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "comment rules normalize spacing around inline comments")]
    #[test]
    fn test_normalize_comment_spacing() {
        assert_eq!(normalize_comment("//comment"), "// comment");
        assert_eq!(normalize_comment("//  extra  spaces"), "// extra  spaces");
        assert_eq!(normalize_comment("  //  indented"), "  // indented");
        assert_eq!(normalize_comment("// already good"), "// already good");
        assert_eq!(normalize_comment("//"), "//");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "import sorting produces alphabetical order")]
    #[test]
    fn test_sort_imports_alphabetical() {
        let lines = vec![
            "use \"types/core\"".to_string(),
            "use \"behaviors/auth\"".to_string(),
            "use \"events/compilation\"".to_string(),
        ];
        let sorted = sort_imports(&lines);
        assert_eq!(sorted[0], "use \"behaviors/auth\"");
        assert_eq!(sorted[1], "use \"events/compilation\"");
        assert_eq!(sorted[2], "use \"types/core\"");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "import sorting produces alphabetical order")]
    #[test]
    fn test_sort_imports_preserves_non_import_lines() {
        let lines = vec![
            "use \"types/core\"".to_string(),
            "use \"behaviors/auth\"".to_string(),
            "".to_string(),
            "behavior foo \"Foo\" {".to_string(),
        ];
        let sorted = sort_imports(&lines);
        assert_eq!(sorted[0], "use \"behaviors/auth\"");
        assert_eq!(sorted[1], "use \"types/core\"");
        assert_eq!(sorted[2], "");
        assert_eq!(sorted[3], "behavior foo \"Foo\" {");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "alignment rules align field values within blocks")]
    #[test]
    fn test_alignment_column() {
        assert_eq!(alignment_column(&["invariants", "types", "ports"]), 11);
        assert_eq!(alignment_column(&["a", "bb", "ccc"]), 4);
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "alignment rules align field values within blocks")]
    #[test]
    fn test_align_field() {
        assert_eq!(align_field("  invariants [a, b]", 12, "  "), "  invariants  [a, b]");
        assert_eq!(align_field("  types [x]", 12, "  "), "  types       [x]");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "wrapping rules break long reference lists to multi-line")]
    #[test]
    fn test_should_wrap_list() {
        let config = FormatConfig { indent_width: 2, use_tabs: false, max_width: 40 };
        assert!(should_wrap_list(&["very_long_item_a", "very_long_item_b", "very_long_item_c"], 2, &config));
        assert!(!should_wrap_list(&["a", "b"], 2, &config));
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "wrapping rules break long reference lists to multi-line")]
    #[test]
    fn test_format_list_multiline() {
        let result = format_list_multiline(&["a", "b", "c"], "  ", "    ");
        assert_eq!(result, "[\n    a,\n    b,\n    c,\n  ]");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "string rules normalize multiline string literal indentation")]
    #[test]
    fn test_normalize_triple_string() {
        let input = "\"\"\"\n    First line\n    Second line\n  \"\"\"";
        let result = normalize_triple_string(input, "  ");
        assert!(result.starts_with("\"\"\""));
        assert!(result.ends_with("\"\"\""));
        assert!(result.contains("First line"));
        assert!(result.contains("Second line"));
    }
}
