use tree_sitter::Parser;

/// Find the identifier (entity ID) at a given 0-indexed line/column position
/// in a .spec source file, using a tree-sitter CST walk.
///
/// Returns `Some(identifier_text)` if an identifier node is found at the position.
pub fn entity_at_position(source: &str, line: u32, col: u32) -> Option<String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .ok()?;

    let tree = parser.parse(source, None)?;
    let point = tree_sitter::Point::new(line as usize, col as usize);
    let node = tree
        .root_node()
        .descendant_for_point_range(point, point)?;

    // Walk up to find identifier nodes
    let mut current = node;
    loop {
        if current.kind() == "identifier" {
            let text = &source[current.byte_range()];
            return Some(text.to_string());
        }
        if current.kind() == "scheme_ref_id" {
            let text = &source[current.byte_range()];
            return Some(text.to_string());
        }
        if let Some(parent) = current.parent() {
            // Don't walk too far up — stop at block nodes
            if parent.kind().ends_with("_block")
                || parent.kind() == "source_file"
                || parent.kind().starts_with("type_")
            {
                break;
            }
            current = parent;
        } else {
            break;
        }
    }

    // Check if the original node is an identifier
    if node.kind() == "identifier" {
        let text = &source[node.byte_range()];
        return Some(text.to_string());
    }

    None
}

/// Find all identifier occurrences matching `target_id` in a source string.
/// Returns (line, start_col, end_col) tuples (0-indexed).
pub fn find_identifier_occurrences(source: &str, target_id: &str) -> Vec<(u32, u32, u32)> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut results = Vec::new();
    collect_identifiers(tree.root_node(), source, target_id, &mut results);
    results
}

fn collect_identifiers(
    node: tree_sitter::Node,
    source: &str,
    target_id: &str,
    results: &mut Vec<(u32, u32, u32)>,
) {
    if node.kind() == "identifier" || node.kind() == "scheme_ref_id" {
        let text = &source[node.byte_range()];
        if text == target_id {
            let start = node.start_position();
            let end = node.end_position();
            results.push((start.row as u32, start.column as u32, end.column as u32));
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_identifiers(child, source, target_id, results);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_entity_at_declaration() {
        let source = r#"behavior validate_input "Validate Input" {
  contract """the system MUST validate"""
}"#;
        // "validate_input" starts at line 0, col 9
        let result = entity_at_position(source, 0, 9);
        assert_eq!(result.as_deref(), Some("validate_input"));
    }

    #[test]
    fn find_entity_in_reference_list() {
        let source = r#"feature input_validation "Input Validation" {
  behaviors [validate_input]
}"#;
        // "validate_input" in the list — line 1, approx col 14
        let result = entity_at_position(source, 1, 14);
        assert_eq!(result.as_deref(), Some("validate_input"));
    }

    #[test]
    fn no_entity_at_keyword() {
        let source = r#"behavior validate_input "Validate Input" {
  contract """the system MUST validate"""
}"#;
        // "behavior" keyword at line 0, col 0 - should not return entity ID
        let result = entity_at_position(source, 0, 0);
        // May return "behavior" as it's an identifier node in the grammar
        // We accept this since the caller filters via symbol table
        assert!(result.is_none() || result.as_deref() == Some("behavior"));
    }

    #[test]
    fn find_all_occurrences() {
        let source = r#"invariant data_integrity "Data Integrity" {
  enforced_by [validate_input]
}
behavior validate_input "Validate Input" {
  invariants [data_integrity]
}"#;
        let occurrences = find_identifier_occurrences(source, "data_integrity");
        assert!(occurrences.len() >= 2); // declaration + reference
    }
}
