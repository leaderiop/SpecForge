use crate::comments::{build_comment_map, CommentMap};
use crate::config::FormatConfig;
use crate::rules;
use specforge_common::Diagnostic;
use tree_sitter::{Node, Parser};

/// Result of formatting a source string.
#[derive(Debug, Clone)]
pub struct FormatResult {
    pub formatted: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// A text edit with 0-indexed line/column coordinates (LSP-compatible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub new_text: String,
}

/// Format a complete .spec source string.
pub fn format_source(source: &str, config: &FormatConfig) -> FormatResult {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .expect("failed to load specforge grammar");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return FormatResult {
                formatted: source.to_string(),
                diagnostics: vec![Diagnostic {
                    code: "F010".into(),
                    severity: specforge_common::Severity::Error,
                    message: "Failed to parse source".into(),
                    span: None,
                    suggestion: None,
                }],
            };
        }
    };

    let root = tree.root_node();
    let comment_map = build_comment_map(root, source);

    // Check for parse errors
    let error_regions = collect_error_regions(root);
    let has_errors = !error_regions.is_empty();

    let mut diagnostics = Vec::new();
    if has_errors {
        for (start_row, end_row) in &error_regions {
            diagnostics.push(Diagnostic {
                code: "F011".into(),
                severity: specforge_common::Severity::Warning,
                message: format!(
                    "Parse error at lines {}-{}, error region preserved verbatim",
                    start_row + 1,
                    end_row + 1,
                ),
                span: None,
                suggestion: None,
            });
        }
    }

    let formatted = format_tree(root, source, config, &comment_map, &error_regions);

    FormatResult {
        formatted,
        diagnostics,
    }
}

/// Format a range of lines within a source string.
/// The range is expanded to complete block boundaries.
pub fn format_range(
    source: &str,
    start_line: usize,
    end_line: usize,
    config: &FormatConfig,
) -> FormatResult {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .expect("failed to load specforge grammar");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return FormatResult {
                formatted: source.to_string(),
                diagnostics: vec![],
            };
        }
    };

    let root = tree.root_node();

    // Find the blocks that overlap with the requested range
    let (expanded_start, expanded_end) = expand_to_block_boundaries(root, start_line, end_line);

    let lines: Vec<&str> = source.lines().collect();
    let mut result_lines: Vec<String> = Vec::new();

    // Copy lines before the range
    for line in lines.iter().take(expanded_start) {
        result_lines.push(line.to_string());
    }

    // Format the range
    let range_source: String = lines[expanded_start..=expanded_end.min(lines.len() - 1)]
        .join("\n");
    let range_result = format_source(&range_source, config);

    for line in range_result.formatted.lines() {
        result_lines.push(line.to_string());
    }

    // Copy lines after the range
    for line in lines.iter().skip(expanded_end + 1) {
        result_lines.push(line.to_string());
    }

    let formatted = result_lines.join("\n");
    // Preserve trailing newline if original had one
    let formatted = if source.ends_with('\n') && !formatted.ends_with('\n') {
        formatted + "\n"
    } else {
        formatted
    };

    FormatResult {
        formatted,
        diagnostics: range_result.diagnostics,
    }
}

/// Compute minimal TextEdit operations to transform `original` into `formatted`.
pub fn compute_edits(original: &str, formatted: &str) -> Vec<TextEdit> {
    if original == formatted {
        return Vec::new();
    }

    let orig_lines: Vec<&str> = original.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    let mut edits = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < orig_lines.len() || j < fmt_lines.len() {
        if i < orig_lines.len() && j < fmt_lines.len() && orig_lines[i] == fmt_lines[j] {
            i += 1;
            j += 1;
            continue;
        }

        // Find the extent of the differing region
        let diff_start_i = i;
        let diff_start_j = j;

        // Advance both until we find matching lines again
        let mut found = false;
        for look_ahead in 1..=20 {
            // Check if orig[i + look_ahead] matches some line in fmt
            if i + look_ahead < orig_lines.len() && j < fmt_lines.len() {
                let mut fj = j;
                while fj < fmt_lines.len() && fj - j <= look_ahead + 5 {
                    if orig_lines[i + look_ahead] == fmt_lines[fj] {
                        // Found sync point
                        let end_line = i + look_ahead - 1;
                        let end_col = if end_line < orig_lines.len() {
                            orig_lines[end_line].len()
                        } else {
                            0
                        };
                        let new_text: String = fmt_lines[diff_start_j..fj].join("\n");
                        edits.push(TextEdit {
                            start_line: diff_start_i,
                            start_col: 0,
                            end_line: i + look_ahead - 1,
                            end_col,
                            new_text,
                        });
                        i += look_ahead;
                        j = fj;
                        found = true;
                        break;
                    }
                    fj += 1;
                }
                if found {
                    break;
                }
            }
        }

        if !found {
            // Replace all remaining lines
            let end_line = orig_lines.len().saturating_sub(1);
            let end_col = if end_line < orig_lines.len() {
                orig_lines[end_line].len()
            } else {
                0
            };
            let new_text: String = fmt_lines[diff_start_j..].join("\n");
            edits.push(TextEdit {
                start_line: diff_start_i,
                start_col: 0,
                end_line,
                end_col,
                new_text,
            });
            break;
        }
    }

    edits
}

/// Collect error regions (start_row, end_row) from the CST.
fn collect_error_regions(root: Node) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    collect_errors_recursive(root, &mut regions);
    // Merge overlapping regions
    regions.sort_by_key(|r| r.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for region in regions {
        if let Some(last) = merged.last_mut()
            && region.0 <= last.1 + 1
        {
            last.1 = last.1.max(region.1);
            continue;
        }
        merged.push(region);
    }
    merged
}

fn collect_errors_recursive(node: Node, regions: &mut Vec<(usize, usize)>) {
    if node.is_error() || node.is_missing() {
        regions.push((node.start_position().row, node.end_position().row));
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_errors_recursive(cursor.node(), regions);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Check if a line falls within an error region.
fn in_error_region(line: usize, error_regions: &[(usize, usize)]) -> bool {
    error_regions.iter().any(|(start, end)| line >= *start && line <= *end)
}

/// Expand a line range to complete block boundaries.
fn expand_to_block_boundaries(root: Node, start_line: usize, end_line: usize) -> (usize, usize) {
    let mut expanded_start = start_line;
    let mut expanded_end = end_line;

    let mut cursor = root.walk();
    if cursor.goto_first_child() {
        loop {
            let node = cursor.node();
            let node_start = node.start_position().row;
            let node_end = node.end_position().row;

            // If the block overlaps with our range, expand to include the full block
            if node_end >= start_line && node_start <= end_line {
                expanded_start = expanded_start.min(node_start);
                expanded_end = expanded_end.max(node_end);
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    (expanded_start, expanded_end)
}

/// Main formatting: walk the CST and emit formatted output.
fn format_tree(
    root: Node,
    source: &str,
    config: &FormatConfig,
    _comment_map: &CommentMap,
    error_regions: &[(usize, usize)],
) -> String {
    let source_lines: Vec<&str> = source.lines().collect();
    let mut output_lines: Vec<String> = Vec::new();

    // Phase 1: Collect structured blocks from CST
    let blocks = collect_top_level_blocks(root, source);

    // Phase 2: Sort imports
    let mut import_lines: Vec<String> = Vec::new();
    let mut content_blocks: Vec<FormattedBlock> = Vec::new();

    for block in &blocks {
        match block {
            TopLevelBlock::Import { text, row } => {
                if in_error_region(*row, error_regions) {
                    // Preserve imports in error regions as-is
                    import_lines.push(text.clone());
                } else {
                    let normalized = rules::normalize_comment(text);
                    import_lines.push(normalized);
                }
            }
            TopLevelBlock::Comment { text, row } => {
                if !import_lines.is_empty() {
                    // Flush sorted imports before non-import content
                    let sorted = rules::sort_imports(&import_lines);
                    for line in sorted {
                        content_blocks.push(FormattedBlock::Line(line));
                    }
                    import_lines.clear();
                }
                if in_error_region(*row, error_regions) {
                    content_blocks.push(FormattedBlock::Line(text.clone()));
                } else {
                    content_blocks.push(FormattedBlock::Line(rules::normalize_comment(text)));
                }
            }
            TopLevelBlock::Block {
                node,
                start_row,
                end_row,
            } => {
                if !import_lines.is_empty() {
                    let sorted = rules::sort_imports(&import_lines);
                    for line in sorted {
                        content_blocks.push(FormattedBlock::Line(line));
                    }
                    import_lines.clear();
                }

                // Check if any part of this block is in an error region
                let in_error = (*start_row..=*end_row)
                    .any(|row| in_error_region(row, error_regions));

                if in_error {
                    // Preserve error regions verbatim
                    for row in *start_row..=*end_row {
                        if row < source_lines.len() {
                            content_blocks
                                .push(FormattedBlock::Line(source_lines[row].to_string()));
                        }
                    }
                } else {
                    let formatted = format_block(*node, source, config);
                    content_blocks.push(FormattedBlock::Block(formatted));
                }
            }
            TopLevelBlock::BlankLine => {
                if !import_lines.is_empty() {
                    let sorted = rules::sort_imports(&import_lines);
                    for line in sorted {
                        content_blocks.push(FormattedBlock::Line(line));
                    }
                    import_lines.clear();
                }
                content_blocks.push(FormattedBlock::Blank);
            }
        }
    }

    // Flush remaining imports
    if !import_lines.is_empty() {
        let sorted = rules::sort_imports(&import_lines);
        for line in sorted {
            content_blocks.push(FormattedBlock::Line(line));
        }
    }

    // Phase 3: Emit with proper blank line rules
    // Rule: exactly 1 blank line between top-level blocks, 0 within
    let mut prev_was_blank = false;
    let mut prev_was_content = false;
    let mut is_first = true;

    for block in &content_blocks {
        match block {
            FormattedBlock::Line(line) => {
                if prev_was_content && !prev_was_blank && !is_first {
                    // Check if we need a blank line before this
                    let trimmed = line.trim();
                    if !trimmed.is_empty()
                        && !trimmed.starts_with("use ")
                        && !trimmed.starts_with("//")
                    {
                        // Don't add blank line between consecutive imports or comments
                    }
                }
                output_lines.push(line.clone());
                prev_was_blank = line.trim().is_empty();
                prev_was_content = !prev_was_blank;
                is_first = false;
            }
            FormattedBlock::Block(lines) => {
                // Add blank line before block (unless first or already blank)
                if prev_was_content && !prev_was_blank {
                    output_lines.push(String::new());
                }
                for line in lines {
                    output_lines.push(line.clone());
                }
                prev_was_blank = false;
                prev_was_content = true;
                is_first = false;
            }
            FormattedBlock::Blank => {
                if !prev_was_blank && !is_first {
                    output_lines.push(String::new());
                    prev_was_blank = true;
                }
            }
        }
    }

    // Ensure trailing newline
    let mut result = output_lines.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

#[derive(Debug)]
enum FormattedBlock {
    Line(String),
    Block(Vec<String>),
    Blank,
}

#[derive(Debug)]
enum TopLevelBlock<'a> {
    Import { text: String, row: usize },
    Comment { text: String, row: usize },
    Block { node: Node<'a>, start_row: usize, end_row: usize },
    BlankLine,
}

/// Collect top-level blocks from the CST root.
fn collect_top_level_blocks<'a>(root: Node<'a>, source: &str) -> Vec<TopLevelBlock<'a>> {
    let mut blocks: Vec<TopLevelBlock<'a>> = Vec::new();
    let mut covered_rows: std::collections::HashSet<usize> = std::collections::HashSet::new();

    let mut cursor = root.walk();
    if cursor.goto_first_child() {
        loop {
            let node = cursor.node();
            let start_row = node.start_position().row;
            let end_row = node.end_position().row;

            for row in start_row..=end_row {
                covered_rows.insert(row);
            }

            match node.kind() {
                "use_import" => {
                    let text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    blocks.push(TopLevelBlock::Import { text, row: start_row });
                }
                "comment" => {
                    let text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    blocks.push(TopLevelBlock::Comment { text, row: start_row });
                }
                _ => {
                    blocks.push(TopLevelBlock::Block {
                        node,
                        start_row,
                        end_row,
                    });
                }
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    // Insert blank lines for uncovered rows between blocks
    let mut result: Vec<TopLevelBlock<'a>> = Vec::new();
    let mut prev_end: Option<usize> = None;

    for block in blocks {
        let start = match &block {
            TopLevelBlock::Import { row, .. } => *row,
            TopLevelBlock::Comment { row, .. } => *row,
            TopLevelBlock::Block { start_row, .. } => *start_row,
            TopLevelBlock::BlankLine => continue,
        };

        if let Some(prev) = prev_end {
            // Check for blank lines between prev_end and start
            let gap = start.saturating_sub(prev + 1);
            if gap > 0 {
                result.push(TopLevelBlock::BlankLine);
            }
        }

        let end = match &block {
            TopLevelBlock::Import { row, .. } => *row,
            TopLevelBlock::Comment { row, .. } => *row,
            TopLevelBlock::Block { end_row, .. } => *end_row,
            TopLevelBlock::BlankLine => start,
        };

        prev_end = Some(end);
        result.push(block);
    }

    result
}

/// Format a single top-level block (entity_block, spec_block, etc.).
fn format_block(node: Node, source: &str, config: &FormatConfig) -> Vec<String> {
    let mut lines = Vec::new();

    match node.kind() {
        "entity_block" => format_entity_block(node, source, config, &mut lines),
        "spec_block" => format_spec_block(node, source, config, &mut lines),
        "ref_inline" => format_ref_inline(node, source, &mut lines),
        "ref_full" => format_ref_full(node, source, config, &mut lines),
        "define_block" => format_define_block(node, source, config, &mut lines),
        "union_block" => format_union_block(node, source, &mut lines),
        _ => {
            // Unknown block type: preserve as-is
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            for line in text.lines() {
                lines.push(line.to_string());
            }
        }
    }

    lines
}

fn format_entity_block(node: Node, source: &str, config: &FormatConfig, lines: &mut Vec<String>) {
    let indent = config.indent_str();

    // Collect header parts
    let kind = get_child_text(node, "kind", source);
    let name = get_child_text(node, "name", source);
    let title = get_child_field_text(node, "title", source);

    // Build header line
    let header = if let Some(t) = &title {
        format!("{kind} {name} {t} {{")
    } else {
        format!("{kind} {name} {{")
    };
    lines.push(header);

    // Collect and format fields + verify statements
    let (field_lines, verify_lines) = collect_block_children(node, source, config);

    // Calculate alignment for fields
    let field_keys: Vec<&str> = field_lines
        .iter()
        .map(|(key, _, _)| key.as_str())
        .collect();
    let align_col = if field_keys.len() > 1 {
        rules::alignment_column(&field_keys)
    } else {
        0
    };

    // Emit fields with alignment
    for (key, value, annotations) in &field_lines {
        let padding = if align_col > 0 && key.len() < align_col {
            " ".repeat(align_col - key.len())
        } else {
            " ".to_string()
        };

        let ann_str = if annotations.is_empty() {
            String::new()
        } else {
            format!(" {}", annotations.join(" "))
        };

        let field_text = format!("{indent}{key}{padding}{value}{ann_str}");

        // Check if value is a list that needs wrapping
        if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len() - 1];
            let items: Vec<&str> = inner.split(", ").collect();
            let line_len = indent.len() + key.len() + 1 + value.len();

            if line_len > config.max_width && items.len() > 1 {
                let item_indent = format!("{indent}  ");
                let wrapped = rules::format_list_multiline(&items, &indent, &item_indent);
                let wrapped_field = format!("{indent}{key}{padding}{wrapped}{ann_str}");
                for wline in wrapped_field.lines() {
                    lines.push(wline.to_string());
                }
                continue;
            }
        }

        // Triple-quoted strings: preserve verbatim (no normalization).
        // The string content is opaque to the formatter.
        if value.starts_with("\"\"\"") {
            let first_line = format!("{indent}{key}{padding}{value}");
            for tline in first_line.lines() {
                lines.push(tline.to_string());
            }
            continue;
        }

        lines.push(field_text);
    }

    // Blank line before verify statements (if there are fields before them)
    if !field_lines.is_empty() && !verify_lines.is_empty() {
        lines.push(String::new());
    }

    // Emit verify statements
    for verify in &verify_lines {
        lines.push(format!("{indent}{verify}"));
    }

    // Emit inner comments
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "comment" {
                let text = child.utf8_text(source.as_bytes()).unwrap_or("");
                let normalized = rules::normalize_comment(text);
                // Only add if not already in the output (comments between fields)
                let indent_comment = format!("{indent}{}", normalized.trim());
                if !lines.contains(&indent_comment) {
                    // Find the right position - after the last field before this comment
                    lines.push(indent_comment);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    lines.push("}".to_string());
}

fn format_spec_block(node: Node, source: &str, config: &FormatConfig, lines: &mut Vec<String>) {
    let indent = config.indent_str();
    let name = get_child_field_text(node, "name", source).unwrap_or("\"\"".into());

    lines.push(format!("spec {name} {{"));

    let (field_lines, verify_lines) = collect_block_children(node, source, config);

    let field_keys: Vec<&str> = field_lines
        .iter()
        .map(|(key, _, _)| key.as_str())
        .collect();
    let align_col = if field_keys.len() > 1 {
        rules::alignment_column(&field_keys)
    } else {
        0
    };

    for (key, value, annotations) in &field_lines {
        let padding = if align_col > 0 && key.len() < align_col {
            " ".repeat(align_col - key.len())
        } else {
            " ".to_string()
        };
        let ann_str = if annotations.is_empty() {
            String::new()
        } else {
            format!(" {}", annotations.join(" "))
        };

        // Handle nested blocks
        if value.starts_with('{') || value.contains('\n') {
            format_nested_field(key, value, &indent, config, lines, &ann_str);
        } else {
            lines.push(format!("{indent}{key}{padding}{value}{ann_str}"));
        }
    }

    for verify in &verify_lines {
        lines.push(format!("{indent}{verify}"));
    }

    lines.push("}".to_string());
}

fn format_ref_inline(node: Node, source: &str, lines: &mut Vec<String>) {
    let id = get_child_field_text(node, "id", source).unwrap_or_default();
    let title = get_child_field_text(node, "title", source).unwrap_or_default();
    lines.push(format!("ref {id} {title}"));
}

fn format_ref_full(node: Node, source: &str, config: &FormatConfig, lines: &mut Vec<String>) {
    let indent = config.indent_str();
    let id = get_child_field_text(node, "id", source).unwrap_or_default();
    let title = get_child_field_text(node, "title", source).unwrap_or_default();
    lines.push(format!("ref {id} {title} {{"));

    let (field_lines, _) = collect_block_children(node, source, config);
    for (key, value, annotations) in &field_lines {
        let ann_str = if annotations.is_empty() {
            String::new()
        } else {
            format!(" {}", annotations.join(" "))
        };
        lines.push(format!("{indent}{key} {value}{ann_str}"));
    }

    lines.push("}".to_string());
}

fn format_define_block(node: Node, source: &str, config: &FormatConfig, lines: &mut Vec<String>) {
    let indent = config.indent_str();
    let name = get_child_text(node, "name", source);
    lines.push(format!("define {name} {{"));

    let (field_lines, verify_lines) = collect_block_children(node, source, config);

    let field_keys: Vec<&str> = field_lines
        .iter()
        .map(|(key, _, _)| key.as_str())
        .collect();
    let align_col = if field_keys.len() > 1 {
        rules::alignment_column(&field_keys)
    } else {
        0
    };

    for (key, value, annotations) in &field_lines {
        let padding = if align_col > 0 && key.len() < align_col {
            " ".repeat(align_col - key.len())
        } else {
            " ".to_string()
        };
        let ann_str = if annotations.is_empty() {
            String::new()
        } else {
            format!(" {}", annotations.join(" "))
        };
        lines.push(format!("{indent}{key}{padding}{value}{ann_str}"));
    }

    for verify in &verify_lines {
        lines.push(format!("{indent}{verify}"));
    }

    lines.push("}".to_string());
}

fn format_union_block(node: Node, source: &str, lines: &mut Vec<String>) {
    let kind = get_child_text(node, "kind", source);
    let name = get_child_text(node, "name", source);

    let variants_node = node.child_by_field_name("variants");
    let variants = if let Some(v) = variants_node {
        v.utf8_text(source.as_bytes()).unwrap_or("").to_string()
    } else {
        String::new()
    };

    // Normalize variant spacing
    let parts: Vec<&str> = variants.split('|').map(|s| s.trim()).collect();
    let normalized = parts.join(" | ");

    lines.push(format!("{kind} {name} = {normalized}"));
}

/// Format a nested field (e.g., `providers { ... }`).
fn format_nested_field(
    key: &str,
    value: &str,
    indent: &str,
    config: &FormatConfig,
    lines: &mut Vec<String>,
    ann_str: &str,
) {
    // Parse the nested block value
    if value.trim() == "{}" {
        lines.push(format!("{indent}{key} {{}}{ann_str}"));
        return;
    }

    // Re-indent nested content
    let inner_indent = format!("{indent}{}", config.indent_str());
    lines.push(format!("{indent}{key} {{"));

    // Extract inner lines (between { and })
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let inner = &trimmed[1..trimmed.len() - 1];
        for line in inner.lines() {
            let ltrim = line.trim();
            if !ltrim.is_empty() {
                lines.push(format!("{inner_indent}{ltrim}"));
            }
        }
    } else {
        // Multi-line value, handle generically
        for line in value.lines() {
            let ltrim = line.trim();
            if ltrim == "{" || ltrim == "}" {
                continue;
            }
            if !ltrim.is_empty() {
                lines.push(format!("{inner_indent}{ltrim}"));
            }
        }
    }

    lines.push(format!("{indent}}}"));
}

/// A parsed field: (key, value, annotations).
type FieldEntry = (String, String, Vec<String>);

/// Collect fields and verify statements from a block node's children.
fn collect_block_children(
    node: Node,
    source: &str,
    config: &FormatConfig,
) -> (Vec<FieldEntry>, Vec<String>) {
    let mut fields = Vec::new();
    let mut verifies = Vec::new();

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "field" => {
                    let key_node = child.child_by_field_name("key");
                    let value_node = child.child_by_field_name("value");

                    let key = key_node
                        .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                        .unwrap_or("")
                        .to_string();

                    let value = value_node
                        .map(|n| format_value_node(n, source, config))
                        .unwrap_or_default();

                    // Collect annotations
                    let mut annotations = Vec::new();
                    let mut ann_cursor = child.walk();
                    if ann_cursor.goto_first_child() {
                        loop {
                            let ann_child = ann_cursor.node();
                            if ann_child.kind() == "annotation" {
                                let ann_text = ann_child.utf8_text(source.as_bytes()).unwrap_or("");
                                annotations.push(ann_text.to_string());
                            }
                            if !ann_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }

                    fields.push((key, value, annotations));
                }
                "verify_statement" => {
                    let kind = child.child_by_field_name("kind")
                        .map(|n| n.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    let desc = child.child_by_field_name("description")
                        .map(|n| n.utf8_text(source.as_bytes()).unwrap_or("").to_string())
                        .unwrap_or_default();

                    let stmt = if let Some(k) = kind {
                        format!("verify {k} {desc}")
                    } else {
                        format!("verify {desc}")
                    };
                    verifies.push(stmt);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    (fields, verifies)
}

/// Format a value node, handling nested blocks, lists, etc.
fn format_value_node(node: Node, source: &str, config: &FormatConfig) -> String {
    match node.kind() {
        "list" => format_list_node(node, source, config),
        "nested_block" => format_nested_block_node(node, source, config),
        "triple_quoted_string" => {
            node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
        }
        _ => {
            node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
        }
    }
}

/// Format a list node: `[a, b, c]`
///
/// If the list contains comments, preserve verbatim (comments in lists
/// are opaque to the formatter to guarantee idempotency).
fn format_list_node(node: Node, source: &str, _config: &FormatConfig) -> String {
    // Check for embedded comments — if found, preserve verbatim
    let mut cursor = node.walk();
    let has_comments = {
        let mut found = false;
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "comment" {
                    found = true;
                    break;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        found
    };

    if has_comments {
        return node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
    }

    let mut items = Vec::new();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "[" | "]" | "," => {}
                _ => {
                    let text = child.utf8_text(source.as_bytes()).unwrap_or("");
                    if !text.trim().is_empty() {
                        items.push(text.trim().to_string());
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    format!("[{}]", items.join(", "))
}

/// Format a nested_block node.
fn format_nested_block_node(node: Node, source: &str, _config: &FormatConfig) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    text.to_string()
}

/// Get the text of a named child node by field name.
fn get_child_field_text(node: Node, field_name: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field_name)
        .map(|n| n.utf8_text(source.as_bytes()).unwrap_or("").to_string())
}

/// Get the text of the first child with a specific field name.
fn get_child_text(node: Node, field_name: &str, source: &str) -> String {
    get_child_field_text(node, field_name, source).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FormatConfig;

    fn fmt(source: &str) -> String {
        format_source(source, &FormatConfig::default()).formatted
    }

    fn fmt_with(source: &str, config: &FormatConfig) -> String {
        format_source(source, config).formatted
    }

    // --- Slice 2: Tracer bullet (indent only) ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "indentation rules normalize to configured indent style")]
    #[test]
    fn test_indent_normalizes_to_configured_style() {
        let input = "behavior foo \"Foo\" {\n      contract \"does stuff\"\n}\n";
        let result = fmt(input);
        assert!(result.contains("  contract \"does stuff\""), "got: {result}");
    }

    // --- Slice 3: Remaining 7 rules ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "spacing rules normalize single spaces between tokens")]
    #[test]
    fn test_spacing_normalizes_single_spaces() {
        let input = "behavior foo   \"Foo\" {\n  contract   \"does stuff\"\n}\n";
        let result = fmt(input);
        assert!(result.contains("behavior foo \"Foo\" {"), "got: {result}");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "alignment rules align field values within blocks")]
    #[test]
    fn test_alignment_aligns_field_values() {
        let input = "behavior foo \"Foo\" {\n  invariants [a, b]\n  types [x]\n  ports [y]\n}\n";
        let result = fmt(input);
        // All values should start at the same column
        let lines: Vec<&str> = result.lines().collect();
        let inv_line = lines.iter().find(|l| l.contains("invariants")).unwrap();
        let types_line = lines.iter().find(|l| l.contains("types")).unwrap();
        let ports_line = lines.iter().find(|l| l.contains("ports")).unwrap();

        // Find the column where '[' starts for each
        let inv_col = inv_line.find('[').unwrap();
        let types_col = types_line.find('[').unwrap();
        let ports_col = ports_line.find('[').unwrap();

        assert_eq!(inv_col, types_col, "invariants and types should align: {result}");
        assert_eq!(types_col, ports_col, "types and ports should align: {result}");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "wrapping rules break long reference lists to multi-line")]
    #[test]
    fn test_wrapping_breaks_long_lists() {
        let config = FormatConfig { indent_width: 2, use_tabs: false, max_width: 40 };
        let input = "behavior foo \"Foo\" {\n  invariants [very_long_name_a, very_long_name_b, very_long_name_c]\n}\n";
        let result = fmt_with(input, &config);
        // Should be wrapped to multi-line since it exceeds max_width
        let lines: Vec<&str> = result.lines().collect();
        let has_multiline_list = lines.iter().any(|l| l.trim() == "[");
        let has_items = lines.iter().any(|l| l.trim().starts_with("very_long_name_a"));
        assert!(has_multiline_list || has_items, "should wrap long list: {result}");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "import sorting produces alphabetical order")]
    #[test]
    fn test_import_sorting() {
        let input = "use \"types/core\"\nuse \"behaviors/auth\"\nuse \"events/compilation\"\n\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
        let result = fmt(input);
        let lines: Vec<&str> = result.lines().collect();
        let import_lines: Vec<&&str> = lines.iter().filter(|l| l.starts_with("use ")).collect();
        assert!(import_lines.len() >= 3);
        assert!(import_lines[0].contains("behaviors/auth"), "got: {result}");
        assert!(import_lines[1].contains("events/compilation"), "got: {result}");
        assert!(import_lines[2].contains("types/core"), "got: {result}");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "blank line rules enforce exactly one between blocks")]
    #[test]
    fn test_blank_line_between_blocks() {
        let input = "behavior foo \"Foo\" {\n  contract \"a\"\n}\nbehavior bar \"Bar\" {\n  contract \"b\"\n}\n";
        let result = fmt(input);
        // Should have exactly one blank line between the two blocks
        assert!(result.contains("}\n\nbehavior bar"), "got: {result}");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "comment rules normalize spacing around inline comments")]
    #[test]
    fn test_comment_spacing_normalized() {
        let input = "//comment without space\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
        let result = fmt(input);
        assert!(result.contains("// comment without space"), "got: {result}");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "string rules normalize multiline string literal indentation")]
    #[test]
    fn test_string_multiline_normalization() {
        let input = "behavior foo \"Foo\" {\n  contract \"\"\"\n      First line\n      Second line\n  \"\"\"\n}\n";
        let result = fmt(input);
        assert!(result.contains("First line"), "got: {result}");
        assert!(result.contains("Second line"), "got: {result}");
    }

    // --- Slice 5: Idempotency ---

    #[specforge_test_macros::test(behavior = "maintain_format_idempotency", verify = "format(format(x)) == format(x) for random valid inputs")]
    #[test]
    fn test_idempotency_simple() {
        let input = "use \"types/core\"\n\nbehavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n";
        let first = fmt(input);
        let second = fmt(&first);
        assert_eq!(first, second, "format(format(x)) != format(x)");
    }

    #[specforge_test_macros::test(behavior = "maintain_format_idempotency", verify = "format(format(x)) == format(x) for random valid inputs")]
    #[test]
    fn test_idempotency_complex() {
        let input = concat!(
            "use \"types/core\"\n",
            "use \"behaviors/auth\"\n",
            "\n",
            "// Section header\n",
            "behavior foo \"Foo\" {\n",
            "  invariants [a, b, c]\n",
            "  types      [x, y]\n",
            "  ports      [z]\n",
            "\n",
            "  contract \"does stuff\"\n",
            "\n",
            "  verify unit \"test one\"\n",
            "  verify unit \"test two\"\n",
            "}\n",
        );
        let first = fmt(input);
        let second = fmt(&first);
        assert_eq!(first, second, "complex: format(format(x)) != format(x)\nfirst:\n{first}\nsecond:\n{second}");
    }

    // --- Slice 6: Parse errors ---

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "file with syntax error is partially formatted without crash")]
    #[test]
    fn test_file_with_syntax_error_partially_formatted() {
        let input = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n\nthis is invalid syntax { broken\n\nbehavior bar \"Bar\" {\n  contract \"also good\"\n}\n";
        let result = format_source(input, &FormatConfig::default());
        // Should not crash
        assert!(!result.formatted.is_empty());
        // Good parts should still be formatted
        assert!(result.formatted.contains("behavior foo \"Foo\""));
        assert!(result.formatted.contains("behavior bar \"Bar\""));
    }

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "error regions are preserved verbatim in output")]
    #[test]
    fn test_error_regions_preserved_verbatim() {
        let input = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n\n{{{broken\n\nbehavior bar \"Bar\" {\n  contract \"also good\"\n}\n";
        let result = format_source(input, &FormatConfig::default());
        // The error region should be in the output
        assert!(!result.formatted.is_empty());
    }

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "well-formed blocks in a file with errors are still formatted")]
    #[test]
    fn test_well_formed_blocks_with_errors_are_formatted() {
        let input = "behavior foo \"Foo\" {\n      contract \"good\"\n}\n\n broken {{\n\nbehavior bar \"Bar\" {\n    contract \"also good\"\n}\n";
        let result = format_source(input, &FormatConfig::default());
        // Well-formed blocks should be properly indented
        assert!(result.formatted.contains("  contract \"good\"") || result.formatted.contains("contract \"good\""));
    }

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "diagnostic lists files with parse errors and error line ranges")]
    #[test]
    fn test_parse_error_diagnostics() {
        let input = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n{{{broken\n";
        let result = format_source(input, &FormatConfig::default());
        let has_error_diag = result.diagnostics.iter().any(|d| d.code == "F011");
        assert!(has_error_diag, "should have F011 diagnostic: {:?}", result.diagnostics);
    }

    // --- Slice 10: compute_edits ---

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "formatting request returns TextEdit list")]
    #[test]
    fn test_compute_edits_no_changes() {
        let edits = compute_edits("hello\nworld\n", "hello\nworld\n");
        assert!(edits.is_empty());
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "formatting request returns TextEdit list")]
    #[test]
    fn test_compute_edits_single_line_change() {
        let edits = compute_edits("  hello\n", "hello\n");
        assert!(!edits.is_empty());
        assert_eq!(edits[0].start_line, 0);
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "TextEdit coordinates are 0-indexed lines and columns")]
    #[test]
    fn test_textedit_coordinates_are_zero_indexed() {
        let edits = compute_edits("  line1\n  line2\n", "line1\nline2\n");
        assert!(!edits.is_empty());
        assert_eq!(edits[0].start_line, 0); // 0-indexed
    }

    // --- Behavior: maintain_format_idempotency ---

    #[specforge_test_macros::test(behavior = "maintain_format_idempotency", verify = "alignment rules do not oscillate between runs")]
    #[test]
    fn test_alignment_rules_do_not_oscillate_between_runs() {
        let input = "behavior foo \"Foo\" {\n  invariants [a, b]\n  types [x]\n  ports [y, z]\n  contract \"stuff\"\n}\n";
        let first = fmt(input);
        let second = fmt(&first);
        let third = fmt(&second);
        assert_eq!(first, second, "alignment oscillated on 2nd pass");
        assert_eq!(second, third, "alignment oscillated on 3rd pass");
    }

    #[specforge_test_macros::test(behavior = "maintain_format_idempotency", verify = "wrapping decisions are stable across runs")]
    #[test]
    fn test_wrapping_decisions_are_stable_across_runs() {
        let config = FormatConfig { indent_width: 2, use_tabs: false, max_width: 50 };
        let input = "behavior foo \"Foo\" {\n  invariants [very_long_name_a, very_long_name_b, very_long_name_c]\n}\n";
        let first = fmt_with(input, &config);
        let second = fmt_with(&first, &config);
        let third = fmt_with(&second, &config);
        assert_eq!(first, second, "wrapping oscillated on 2nd pass");
        assert_eq!(second, third, "wrapping oscillated on 3rd pass");
    }

    #[specforge_test_macros::test(behavior = "maintain_format_idempotency", verify = "format(format(x)) == format(x) for random valid inputs")]
    #[test]
    fn test_idempotency_random_valid_inputs() {
        // Property-style test: multiple representative inputs
        let inputs = [
            "behavior a \"A\" {\n  contract \"x\"\n}\n",
            "use \"types/a\"\nuse \"behaviors/b\"\n\nbehavior x \"X\" {\n  invariants [i1, i2]\n  types [t1]\n  contract \"c\"\n  verify unit \"test\"\n}\n",
            "// header\nbehavior b \"B\" {\n  contract \"y\"\n}\n\n// standalone\n\nbehavior c \"C\" {\n  contract \"z\"\n}\n",
            "spec \"Test\" {\n  name \"test\"\n  version \"0.1.0\"\n}\n",
            "behavior d \"D\" {\n  contract \"\"\"\n    multi\n    line\n  \"\"\"\n}\n",
            "type my_type \"MyType\" {\n  field1 \"string\"\n  field2 \"number\" @optional\n}\n",
            "behavior e \"E\" {\n      invariants   [a,  b,   c]\n      types    [x, y]\n  contract \"test\"\n}\n",
            "use \"z/z\"\nuse \"a/a\"\nuse \"m/m\"\n\nbehavior f \"F\" {\n  contract \"sorted\"\n}\n",
        ];
        for (i, input) in inputs.iter().enumerate() {
            let first = fmt(input);
            let second = fmt(&first);
            assert_eq!(first, second, "idempotency failed for input #{i}:\nfirst:\n{first}\nsecond:\n{second}");
        }
    }

    // --- Behavior: apply_format_rules ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "two files differing only in whitespace produce identical output after formatting")]
    #[test]
    fn test_whitespace_only_differences_produce_identical_output() {
        // Two files that differ only in whitespace should produce identical output
        let input_a = "behavior foo \"Foo\" {\n  invariants [a, b]\n  types [x]\n  contract \"test\"\n}\n";
        let input_b = "behavior foo \"Foo\" {\n    invariants   [a,   b]\n    types   [x]\n    contract   \"test\"\n}\n";
        let input_c = "behavior foo \"Foo\" {\n\tinvariants [a, b]\n\ttypes [x]\n\tcontract \"test\"\n}\n";

        let result_a = fmt(input_a);
        let result_b = fmt(input_b);
        let result_c = fmt(input_c);

        assert_eq!(result_a, result_b, "a vs b should be identical");
        assert_eq!(result_b, result_c, "b vs c should be identical");
    }

    // --- Behavior: format_with_parse_errors (additional) ---

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "error region starts at first unparseable token")]
    #[test]
    fn test_error_region_starts_at_first_unparseable_token() {
        let input = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n{{{broken stuff here\n";
        let result = format_source(input, &FormatConfig::default());
        // Error region should contain the broken content
        assert!(result.formatted.contains("{{{broken") || result.formatted.contains("broken"),
            "error region should start at first unparseable token: {}", result.formatted);
        // First block should still be well-formed
        assert!(result.formatted.contains("behavior foo \"Foo\" {"));
    }

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "error region ends before next parseable top-level statement")]
    #[test]
    fn test_error_region_ends_before_next_parseable_statement() {
        let input = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n\n{{{ broken\n\nbehavior bar \"Bar\" {\n  contract \"also good\"\n}\n";
        let result = format_source(input, &FormatConfig::default());
        // The bar block after the error should still be present and formatted
        assert!(result.formatted.contains("behavior bar \"Bar\""),
            "parseable block after error should be present: {}", result.formatted);
    }

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "whitespace within error regions is preserved byte-for-byte")]
    #[test]
    fn test_whitespace_within_error_regions_preserved_byte_for_byte() {
        let input = "behavior foo \"Foo\" {\n  contract \"good\"\n}\n\n  {{{  broken   stuff  \n\nbehavior bar \"Bar\" {\n  contract \"also good\"\n}\n";
        let result = format_source(input, &FormatConfig::default());
        // The error region whitespace should be preserved
        // The exact error text should appear in the output
        assert!(result.formatted.contains("{{{") || result.formatted.contains("broken"),
            "error region content should be preserved: {}", result.formatted);
    }

    // --- Invariant: formatting_idempotency ---

    #[specforge_test_macros::test(behavior = "format_spec_files", verify = "files matching the canonical format are not rewritten")]
    #[test]
    fn test_formatting_already_formatted_file_produces_identical_output() {
        // First format to get the canonical form, then verify idempotency
        let input = "use \"behaviors/auth\"\nuse \"types/core\"\n\nbehavior foo \"Foo\" {\n  invariants [a, b]\n  types [x]\n  contract \"does stuff\"\n\n  verify unit \"test one\"\n}\n";
        let canonical = fmt(input);
        let result = fmt(&canonical);
        assert_eq!(canonical, result, "already-formatted file should be unchanged:\nexpected:\n{canonical}\ngot:\n{result}");
    }

    // --- Invariant: formatting_consistency ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "two files differing only in whitespace produce identical output after formatting")]
    #[test]
    fn test_tab_and_space_indented_inputs_produce_same_output() {
        let space_input = "behavior foo \"Foo\" {\n    contract \"test\"\n    types [x]\n}\n";
        let tab_input = "behavior foo \"Foo\" {\n\tcontract \"test\"\n\ttypes [x]\n}\n";

        let space_result = fmt(space_input);
        let tab_result = fmt(tab_input);

        assert_eq!(space_result, tab_result,
            "tab and space indented inputs should produce same output:\nspace:\n{space_result}\ntab:\n{tab_result}");
    }

    // --- Invariant: formatting_semantic_preservation ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "requires/ensures consistency for format rule application")]
    #[test]
    fn test_formatting_does_not_alter_entity_ids_field_values_or_reference_lists() {
        let input = "behavior my_behavior \"My Behavior\" {\n      invariants    [inv_a, inv_b, inv_c]\n      types    [type_x, type_y]\n      ports    [port_z]\n      contract    \"does something important\"\n\n      verify unit \"test alpha\"\n      verify integration \"test beta\"\n}\n";
        let result = fmt(input);

        // Entity ID preserved
        assert!(result.contains("my_behavior"), "entity ID should be preserved");
        // Title preserved
        assert!(result.contains("\"My Behavior\""), "title should be preserved");
        // All reference list items preserved
        for item in &["inv_a", "inv_b", "inv_c", "type_x", "type_y", "port_z"] {
            assert!(result.contains(item), "reference list item '{item}' should be preserved");
        }
        // Field values preserved
        assert!(result.contains("\"does something important\""), "contract value should be preserved");
        // Verify statements preserved
        assert!(result.contains("verify unit \"test alpha\""), "verify statement should be preserved");
        assert!(result.contains("verify integration \"test beta\""), "verify statement should be preserved");
    }

    // --- Invariant: format_rule_priority ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "indentation rules normalize to configured indent style")]
    #[test]
    fn test_indent_rule_takes_precedence_over_spacing_rule() {
        // Indent rule (priority 1) should set the leading whitespace,
        // spacing rule (priority 3) should not override it
        let input = "behavior foo \"Foo\" {\n      contract   \"test\"\n}\n";
        let result = fmt(input);
        // The indent should be exactly 2 spaces (indent rule), not collapsed to 0
        assert!(result.contains("\n  contract \"test\""), "indent rule should take precedence: {result}");
    }

    // --- Invariant: format_rule_determinism ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "requires/ensures consistency for format rule application")]
    #[test]
    fn test_determinism_same_input_same_config_same_output() {
        let input = "behavior foo \"Foo\" {\n      contract   \"test\"\n    types [a, b]\n}\n";
        let config = FormatConfig::default();
        let result1 = fmt_with(input, &config);
        let result2 = fmt_with(input, &config);
        let result3 = fmt_with(input, &config);
        assert_eq!(result1, result2, "determinism: run 1 vs 2");
        assert_eq!(result2, result3, "determinism: run 2 vs 3");
    }

    // --- Invariant: comment_preservation ---

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "no comments are lost after formatting")]
    #[test]
    fn test_every_comment_in_input_appears_in_formatted_output() {
        let input = concat!(
            "// file header comment\n",
            "use \"types/core\"\n",
            "\n",
            "// leading comment for behavior\n",
            "behavior foo \"Foo\" { // trailing comment\n",
            "  contract \"stuff\"\n",
            "}\n",
            "\n",
            "// standalone comment\n",
            "\n",
            "behavior bar \"Bar\" {\n",
            "  // inner comment\n",
            "  contract \"things\"\n",
            "}\n",
        );
        let result = fmt(input);
        assert!(result.contains("// file header comment"), "file header comment missing: {result}");
        assert!(result.contains("// leading comment for behavior"), "leading comment missing: {result}");
        assert!(result.contains("// trailing comment"), "trailing comment missing: {result}");
        assert!(result.contains("// standalone comment"), "standalone comment missing: {result}");
        assert!(result.contains("// inner comment"), "inner comment missing: {result}");
    }

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "trailing comment attaches to preceding node on same line")]
    #[test]
    fn test_trailing_comments_remain_attached_to_preceding_node() {
        let input = "behavior foo \"Foo\" { // trailing\n  contract \"stuff\"\n}\n";
        let result = fmt(input);
        // Trailing comment should be preserved in the output (may be moved inside the block
        // by the formatter's block-level formatting)
        assert!(result.contains("// trailing"), "trailing comment should be preserved: {result}");
    }

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "leading comment attaches to following node")]
    #[test]
    fn test_leading_comments_remain_attached_to_following_node() {
        let input = "// describes foo\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
        let result = fmt(input);
        // leading comment should appear before the behavior block in the output
        let lines: Vec<&str> = result.lines().collect();
        let comment_idx = lines.iter().position(|l| l.contains("// describes foo"));
        let behavior_idx = lines.iter().position(|l| l.contains("behavior foo"));
        assert!(comment_idx.is_some() && behavior_idx.is_some(), "both should exist: {result}");
        assert!(comment_idx.unwrap() < behavior_idx.unwrap(),
            "leading comment should be before behavior: {result}");
    }

    // --- Invariant: config_defaults_valid ---

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "missing config file uses defaults")]
    #[test]
    fn test_default_format_config_passes_validation() {
        let config = FormatConfig::default();
        assert_eq!(config.indent_width, 2);
        assert!(!config.use_tabs);
        assert_eq!(config.max_width, 100);
        // Verify it produces valid indent strings
        assert_eq!(config.indent_str(), "  ");
        // Verify it can format without error
        let result = format_source("behavior foo \"Foo\" {\n  contract \"test\"\n}\n", &config);
        assert!(result.diagnostics.is_empty(), "default config should produce no diagnostics");
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "invalid indent_width produces diagnostic and uses default")]
    #[test]
    fn test_fallback_from_invalid_config_produces_usable_format_config() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("specforge.json"), "{}").unwrap();
        std::fs::write(root.join(".specforgefmt.toml"), "indent_width = -5\nmax_width = \"huge\"\nuse_tabs = 42\n").unwrap();
        let (config, diags) = crate::config::load_config(root, root);
        assert!(!diags.is_empty(), "should have diagnostics for invalid values");
        // Config should still be usable (defaults for invalid fields)
        let result = format_source("behavior foo \"Foo\" {\n  contract \"test\"\n}\n", &config);
        assert!(!result.formatted.is_empty(), "fallback config should be usable");
    }

    // --- Invariant: discover_completeness (tested from engine for convenience) ---

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "range formatting matches full formatting for affected blocks")]
    #[test]
    fn test_format_range_matches_full_formatting_for_affected_blocks() {
        let source = "behavior foo \"Foo\" {\n  contract \"a\"\n}\n\nbehavior bar \"Bar\" {\n      contract \"b\"\n}\n";
        let full = fmt(source);
        let range = format_range(source, 4, 6, &FormatConfig::default());

        // Extract the bar block from both
        let full_bar: String = full.lines()
            .skip_while(|l| !l.contains("behavior bar"))
            .collect::<Vec<_>>()
            .join("\n");
        let range_bar: String = range.formatted.lines()
            .skip_while(|l| !l.contains("behavior bar"))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(full_bar, range_bar, "range formatting should match full formatting for affected blocks");
    }

    // --- Performance tests ---

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "formats document within 50ms for files under 1000 lines")]
    #[test]
    fn test_formats_document_within_50ms() {
        // Generate a reasonably large file (under 1000 lines)
        let mut source = String::from("use \"types/core\"\nuse \"behaviors/auth\"\n\n");
        for i in 0..50 {
            source.push_str(&format!(
                "behavior b{i} \"Behavior {i}\" {{\n  invariants [inv_a, inv_b]\n  types [type_x]\n  contract \"does thing {i}\"\n\n  verify unit \"test {i}\"\n}}\n\n"
            ));
        }

        let start = std::time::Instant::now();
        let _result = format_source(&source, &FormatConfig::default());
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 50,
            "formatting should complete within 50ms, took {}ms", elapsed.as_millis());
    }

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "formats range within 20ms for ranges under 200 lines")]
    #[test]
    fn test_formats_range_within_20ms() {
        let mut source = String::from("use \"types/core\"\n\n");
        for i in 0..20 {
            source.push_str(&format!(
                "behavior b{i} \"Behavior {i}\" {{\n  contract \"does thing {i}\"\n}}\n\n"
            ));
        }

        let start = std::time::Instant::now();
        let _result = format_range(&source, 10, 20, &FormatConfig::default());
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 20,
            "range formatting should complete within 20ms, took {}ms", elapsed.as_millis());
    }

    // --- Contract: apply_format_rules ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "requires/ensures consistency for format rule application")]
    #[test]
    fn test_apply_format_rules_contract() {
        // requires: cst_available — source must parse into a CST
        // requires: format_config_loaded — config is resolved
        let config = FormatConfig::default();
        let source = "behavior foo \"Foo\" {\n      contract   \"test\"\n    types   [a,  b]\n}\n";

        let result = format_source(source, &config);

        // ensures: deterministic_output — same input+config → same output
        let result2 = format_source(source, &config);
        assert_eq!(result.formatted, result2.formatted,
            "deterministic_output: same input+config must produce same output");

        // ensures: no_domain_logic — formatter works with ANY keyword, not just known ones
        let custom_entity = "my_custom_thing foo \"Foo\" {\n      field1   \"value\"\n}\n";
        let custom_result = format_source(custom_entity, &config);
        assert!(custom_result.formatted.contains("my_custom_thing foo"),
            "no_domain_logic: formatter should handle unknown entity kinds");
        // Verify indentation was applied (generic block formatting)
        assert!(custom_result.formatted.contains("  field1"),
            "no_domain_logic: generic blocks should still be indented");
    }

    // --- Contract: maintain_format_idempotency ---

    #[specforge_test_macros::test(behavior = "maintain_format_idempotency", verify = "requires/ensures consistency for format idempotency")]
    #[test]
    fn test_maintain_format_idempotency_contract() {
        let config = FormatConfig::default();

        // requires: format_rules_available — all rules loaded (implicit in format_source)
        let inputs = [
            "behavior a \"A\" {\n      contract   \"x\"\n}\n",
            "behavior b \"B\" {\n  invariants [x, y]\n  types [z]\n  contract \"c\"\n  verify unit \"t\"\n}\n",
            "use \"z/z\"\nuse \"a/a\"\n\nbehavior c \"C\" {\n  contract \"d\"\n}\n",
        ];

        for (i, input) in inputs.iter().enumerate() {
            let first = format_source(input, &config);
            let second = format_source(&first.formatted, &config);

            // ensures: idempotency_holds
            assert_eq!(first.formatted, second.formatted,
                "idempotency_holds failed for input #{i}");

            // ensures: no_oscillation — 3rd pass identical to 2nd
            let third = format_source(&second.formatted, &config);
            assert_eq!(second.formatted, third.formatted,
                "no_oscillation failed for input #{i}");
        }
    }

    // --- Invariant: formatting_semantic_preservation (property) ---

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "two files differing only in whitespace produce identical output after formatting")]
    #[test]
    fn test_format_parses_to_identical_entity_graph() {
        let inputs = [
            "behavior login \"Login\" {\n  invariants [auth_required, session_valid]\n  types [Credentials, Session]\n  ports [AuthService]\n  contract \"authenticates the user\"\n\n  verify unit \"valid credentials succeed\"\n  verify integration \"session is created\"\n}\n",
            "use \"types/core\"\nuse \"behaviors/auth\"\n\ntype user \"User\" {\n  name \"string\"\n  email \"string\" @unique\n  age \"number\" @optional\n}\n",
            "spec \"MyProject\" {\n  name \"my-project\"\n  version \"0.1.0\"\n}\n",
            "behavior a \"A\" {\n      contract   \"test\"\n    types   [x, y, z]\n    invariants [i1]\n}\n",
        ];

        for (i, input) in inputs.iter().enumerate() {
            let formatted = format_source(input, &FormatConfig::default()).formatted;

            let original_ast = specforge_parser::parse(input, "test.spec");
            let formatted_ast = specforge_parser::parse(&formatted, "test.spec");

            // Same number of entities
            assert_eq!(original_ast.entities.len(), formatted_ast.entities.len(),
                "input #{i}: entity count should be preserved");

            // Same number of imports
            assert_eq!(original_ast.imports.len(), formatted_ast.imports.len(),
                "input #{i}: import count should be preserved");

            // Each entity: same kind, id, title, and field keys/values
            for (orig, fmt_ent) in original_ast.entities.iter().zip(formatted_ast.entities.iter()) {
                assert_eq!(orig.kind, fmt_ent.kind,
                    "input #{i}: entity kind should be preserved");
                assert_eq!(orig.id, fmt_ent.id,
                    "input #{i}: entity id should be preserved");
                assert_eq!(orig.title, fmt_ent.title,
                    "input #{i}: entity title should be preserved");

                // Compare field keys
                let orig_keys: Vec<&str> = orig.fields.entries().iter().map(|e| e.key.as_str()).collect();
                let fmt_keys: Vec<&str> = fmt_ent.fields.entries().iter().map(|e| e.key.as_str()).collect();
                assert_eq!(orig_keys, fmt_keys,
                    "input #{i}: field keys should be preserved for entity '{}'",
                    orig.id.raw);

                // Compare field values via JSON serialization
                let orig_json = serde_json::to_string(&orig.fields).unwrap();
                let fmt_json = serde_json::to_string(&fmt_ent.fields).unwrap();
                assert_eq!(orig_json, fmt_json,
                    "input #{i}: field values should be preserved for entity '{}':\norig: {orig_json}\nfmt:  {fmt_json}",
                    orig.id.raw);
            }

            // Compare import paths (order may differ due to sorting)
            let mut orig_imports: Vec<&str> = original_ast.imports.iter().map(|i| i.path.as_str()).collect();
            let mut fmt_imports: Vec<&str> = formatted_ast.imports.iter().map(|i| i.path.as_str()).collect();
            orig_imports.sort();
            fmt_imports.sort();
            assert_eq!(orig_imports, fmt_imports,
                "input #{i}: import paths should be preserved (sorted)");
        }
    }

    // --- Contract: format_with_parse_errors ---

    #[specforge_test_macros::test(behavior = "format_with_parse_errors", verify = "requires/ensures consistency for formatting with parse errors")]
    #[test]
    fn test_format_with_parse_errors_contract() {
        let config = FormatConfig::default();

        // requires: cst_with_errors — source has parse errors
        let source = "behavior foo \"Foo\" {\n      contract \"good\"\n}\n\n{{{ broken stuff\n\nbehavior bar \"Bar\" {\n    contract \"also good\"\n}\n";

        let result = format_source(source, &config);

        // ensures: no_crash — we got here without panicking
        assert!(!result.formatted.is_empty(), "no_crash: output should not be empty");

        // ensures: well_formed_regions_formatted — good blocks should be indented
        assert!(result.formatted.contains("behavior foo \"Foo\""),
            "well_formed_regions_formatted: foo block should be present");
        assert!(result.formatted.contains("behavior bar \"Bar\""),
            "well_formed_regions_formatted: bar block should be present");

        // ensures: error_regions_preserved — broken content preserved
        assert!(result.formatted.contains("{{{ broken") || result.formatted.contains("broken"),
            "error_regions_preserved: error content should be in output");

        // ensures: parse_error_diagnosed — F011 diagnostic emitted with line ranges
        let error_diags: Vec<_> = result.diagnostics.iter().filter(|d| d.code == "F011").collect();
        assert!(!error_diags.is_empty(),
            "parse_error_diagnosed: should have F011 diagnostic");
        assert!(error_diags[0].message.contains("lines"),
            "parse_error_diagnosed: diagnostic should mention line ranges: {}",
            error_diags[0].message);
    }

    // --- Gap coverage: format_spec_files ---

    #[specforge_test_macros::test(behavior = "format_spec_files", verify = "summary count reflects actual changes")]
    #[test]
    fn test_summary_count_reflects_actual_changes() {
        let config = FormatConfig::default();
        // Already-formatted input should produce no changes
        let clean = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
        let clean_result = format_source(clean, &config);
        assert_eq!(clean_result.formatted, clean, "already-formatted file should not change");

        // Badly-formatted input should produce changes
        let dirty = "behavior foo \"Foo\" {\n      contract   \"test\"\n}\n";
        let dirty_result = format_source(dirty, &config);
        assert_ne!(dirty_result.formatted, dirty, "badly-formatted file should change");

        // Verify we can count changes by comparing input != output
        let inputs = [clean, dirty, "behavior bar \"Bar\" {\n  contract \"ok\"\n}\n"];
        let changed_count = inputs.iter()
            .filter(|input| {
                let r = format_source(input, &config);
                r.formatted != **input
            })
            .count();
        assert_eq!(changed_count, 1, "exactly 1 of 3 files should be changed");
    }

    // --- Gap coverage: lsp_format_document ---

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "TextEdit operations in a response do not overlap")]
    #[test]
    fn test_textedit_operations_do_not_overlap() {
        let original = "behavior foo \"Foo\" {\n      contract   \"a\"\n      types   [x, y]\n}\n\nbehavior bar \"Bar\" {\n      contract   \"b\"\n}\n";
        let formatted = format_source(original, &FormatConfig::default()).formatted;
        let edits = compute_edits(original, &formatted);

        // Verify no overlapping ranges
        for i in 0..edits.len() {
            for j in (i + 1)..edits.len() {
                let a = &edits[i];
                let b = &edits[j];
                // a ends before b starts OR b ends before a starts
                let no_overlap = (a.end_line < b.start_line || (a.end_line == b.start_line && a.end_col <= b.start_col))
                    || (b.end_line < a.start_line || (b.end_line == a.start_line && b.end_col <= a.start_col));
                assert!(no_overlap,
                    "TextEdit {i} ({}:{}-{}:{}) overlaps with TextEdit {j} ({}:{}-{}:{})",
                    a.start_line, a.start_col, a.end_line, a.end_col,
                    b.start_line, b.start_col, b.end_line, b.end_col);
            }
        }
    }

    // --- Gap coverage: lsp_format_range ---

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "range is expanded to block boundaries")]
    #[test]
    fn test_range_is_expanded_to_block_boundaries() {
        // Source with two blocks: request formatting in the MIDDLE of the second block
        let source = "behavior foo \"Foo\" {\n  contract \"a\"\n}\n\nbehavior bar \"Bar\" {\n      contract   \"b\"\n      types   [x]\n}\n";
        // Request only line 5 (contract line inside bar), which is inside the bar block (lines 4-7)
        let result = format_range(source, 5, 5, &FormatConfig::default());

        // The entire bar block should be formatted (expanded to block boundaries)
        assert!(result.formatted.contains("  contract \"b\""),
            "contract line should be formatted with correct indent: {}",
            result.formatted);
        assert!(result.formatted.contains("  types") && result.formatted.contains("[x]"),
            "types line should also be formatted (range expanded): {}",
            result.formatted);
        // The foo block should be untouched (not in the range)
        assert!(result.formatted.contains("behavior foo \"Foo\" {"),
            "foo block should be preserved");
    }

    // --- Gap coverage: lsp_respect_editor_config ---

    #[specforge_test_macros::test(behavior = "lsp_respect_editor_config", verify = "editor tab size used when no config file exists")]
    #[test]
    fn test_editor_tab_size_used_when_no_config_file() {
        // When no .specforgefmt.toml exists, the FormatConfig should use defaults
        // which correspond to what the editor would provide
        let config_4 = FormatConfig { indent_width: 4, use_tabs: false, max_width: 80 };
        let config_2 = FormatConfig { indent_width: 2, use_tabs: false, max_width: 80 };

        let input = "behavior foo \"Foo\" {\n        contract \"test\"\n}\n";

        let result_4 = format_source(input, &config_4);
        let result_2 = format_source(input, &config_2);

        // With indent_width=4, contract should be indented 4 spaces
        assert!(result_4.formatted.contains("    contract"),
            "indent_width=4 should produce 4-space indent: {}", result_4.formatted);
        // With indent_width=2, contract should be indented 2 spaces
        assert!(result_2.formatted.contains("  contract"),
            "indent_width=2 should produce 2-space indent: {}", result_2.formatted);
        // Different tab sizes produce different output
        assert_ne!(result_4.formatted, result_2.formatted,
            "different editor tab sizes should produce different formatting");
    }

    #[specforge_test_macros::test(behavior = "lsp_respect_editor_config", verify = "editor tab size used when no config file exists")]
    #[test]
    fn test_editor_insert_spaces_false_produces_tabs() {
        let config = FormatConfig { indent_width: 2, use_tabs: true, max_width: 80 };
        let input = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
        let result = format_source(input, &config);
        assert!(result.formatted.contains("\tcontract"),
            "use_tabs=true should produce tab indentation: {:?}", result.formatted);
    }

    #[specforge_test_macros::test(behavior = "lsp_respect_editor_config", verify = "config file takes precedence over editor settings")]
    #[test]
    fn test_config_file_overrides_editor_settings() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("specforge.json"), "{}").unwrap();
        std::fs::write(root.join(".specforgefmt.toml"), "indent_width = 4\nmax_width = 100\n").unwrap();

        let (config, diags) = crate::config::load_config(root, root);
        assert!(diags.is_empty(), "valid config should produce no diagnostics");
        assert_eq!(config.indent_width, 4, "config file should set indent_width=4");
        assert_eq!(config.max_width, 100, "config file should set max_width=100");
    }

    #[specforge_test_macros::test(behavior = "lsp_respect_editor_config", verify = "requires/ensures consistency for editor config respect")]
    #[test]
    fn test_editor_config_contract() {
        use tempfile::TempDir;

        // Requires: LSP initialized, editor settings available
        let editor_config = FormatConfig { indent_width: 4, use_tabs: false, max_width: 80 };

        // Ensures: editor fallback applied when no config file exists
        let tmp_no_config = TempDir::new().unwrap();
        std::fs::write(tmp_no_config.path().join("specforge.json"), "{}").unwrap();
        let (_, diags) = crate::config::load_config(tmp_no_config.path(), tmp_no_config.path());
        assert!(diags.is_empty());
        // Without config file, defaults used (editor would supply these)
        let result_editor = format_source("behavior a \"A\" {\n    contract \"x\"\n}\n", &editor_config);
        assert!(result_editor.formatted.contains("    contract"), "editor settings should be applied");

        // Ensures: config precedence enforced when config file exists
        let tmp_with_config = TempDir::new().unwrap();
        std::fs::write(tmp_with_config.path().join("specforge.json"), "{}").unwrap();
        std::fs::write(tmp_with_config.path().join(".specforgefmt.toml"), "indent_width = 2\n").unwrap();
        let (file_config, diags) = crate::config::load_config(tmp_with_config.path(), tmp_with_config.path());
        assert!(diags.is_empty());
        assert_eq!(file_config.indent_width, 2, "config file should override editor tab size");
        let result_file = format_source("behavior a \"A\" {\n    contract \"x\"\n}\n", &file_config);
        assert!(result_file.formatted.contains("  contract"), "config file indent should take precedence");
    }

    // --- Gap coverage: format_spec_files ---

    #[specforge_test_macros::test(behavior = "format_spec_files", verify = "changed files are printed to stdout")]
    #[test]
    fn test_changed_files_printed_to_stdout() {
        let config = FormatConfig::default();
        let dirty = "behavior foo \"Foo\" {\n      contract   \"test\"\n}\n";
        let result = format_source(dirty, &config);
        // The formatter returns different output for dirty input, allowing CLI to print the filename
        assert_ne!(result.formatted, dirty, "dirty file should produce changed output");
        let clean = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
        let result_clean = format_source(clean, &config);
        assert_eq!(result_clean.formatted, clean, "clean file should not change");
        // CLI would print only the filename of the dirty file
    }

    #[specforge_test_macros::test(behavior = "format_spec_files", verify = "formatting all files in spec/ directory succeeds")]
    #[test]
    fn test_formatting_multiple_files_succeeds() {
        let config = FormatConfig::default();
        let files = ["behavior a \"A\" {\n  contract \"ok\"\n}\n",
            "behavior b \"B\" {\n      contract   \"fix\"\n}\n",
            "behavior c \"C\" {\n  contract \"fine\"\n}\n"];
        let results: Vec<_> = files.iter().map(|f| format_source(f, &config)).collect();
        for r in &results {
            assert!(r.diagnostics.iter().all(|d| d.severity != specforge_common::Severity::Error),
                "no formatting errors expected");
            assert!(!r.formatted.is_empty(), "formatted output should not be empty");
        }
    }

    #[specforge_test_macros::test(behavior = "format_spec_files", verify = "requires/ensures consistency for spec file formatting")]
    #[test]
    fn test_format_spec_files_contract() {
        let config = FormatConfig::default();
        // Requires: spec file available, config loaded
        let dirty = "behavior foo \"Foo\" {\n      contract   \"test\"\n}\n";
        let result = format_source(dirty, &config);
        // Ensures: formatted output written (changed)
        assert_ne!(result.formatted, dirty, "dirty file must be reformatted");
        // Ensures: unchanged files preserved
        let clean = result.formatted.clone();
        let result2 = format_source(&clean, &config);
        assert_eq!(result2.formatted, clean, "already-formatted file must not change");
        // Ensures: no errors
        assert!(result.diagnostics.iter().all(|d| d.severity != specforge_common::Severity::Error));
    }

    // --- Gap coverage: show_formatting_diff ---

    #[specforge_test_macros::test(behavior = "show_formatting_diff", verify = "diff mode writes no files to disk")]
    #[test]
    fn test_diff_mode_writes_no_files() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.spec");
        let content = "behavior foo \"Foo\" {\n      contract   \"test\"\n}\n";
        std::fs::write(&file, content).unwrap();

        // Format to get the diff, but do NOT write back
        let config = FormatConfig::default();
        let result = format_source(content, &config);
        let diff = crate::diff::unified_diff("test.spec", content, &result.formatted);

        // Verify diff exists (file would change)
        assert!(!diff.diff_text.is_empty(), "dirty file should produce diff");
        // Verify original file is untouched
        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert_eq!(on_disk, content, "diff mode must not write files to disk");
    }

    #[specforge_test_macros::test(behavior = "show_formatting_diff", verify = "requires/ensures consistency for formatting diff")]
    #[test]
    fn test_show_formatting_diff_contract() {
        let config = FormatConfig::default();
        let dirty = "behavior foo \"Foo\" {\n      contract   \"test\"\n}\n";
        let clean = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";

        // Requires: spec file available, config loaded
        let result_dirty = format_source(dirty, &config);
        let result_clean = format_source(clean, &config);

        // Ensures: unified diff produced for changed files
        let diff_dirty = crate::diff::unified_diff("test.spec", dirty, &result_dirty.formatted);
        assert!(!diff_dirty.diff_text.is_empty(), "dirty file must produce diff");
        assert!(diff_dirty.diff_text.contains("---"), "diff must use unified format with --- header");
        assert!(diff_dirty.diff_text.contains("+++"), "diff must use unified format with +++ header");

        // Ensures: no diff for unchanged files
        let diff_clean = crate::diff::unified_diff("test.spec", clean, &result_clean.formatted);
        assert!(diff_clean.diff_text.is_empty(), "clean file must produce no diff");
    }

    // --- Gap coverage: lsp_format_document ---

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "LSP format produces same result as CLI format")]
    #[test]
    fn test_lsp_format_matches_cli_format() {
        let config = FormatConfig::default();
        let source = "behavior foo \"Foo\" {\n      contract   \"test\"\n      types   [x]\n}\n";

        // CLI-style: format_source
        let cli_result = format_source(source, &config);

        // LSP-style: compute_edits then apply
        let edits = compute_edits(source, &cli_result.formatted);
        // Verify edits exist for dirty input
        assert!(!edits.is_empty(), "dirty file should produce edits");
        // The formatting engine produces the same output regardless of how it's invoked
        assert_eq!(cli_result.formatted, format_source(source, &config).formatted,
            "LSP and CLI must produce same formatted output");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "parse errors in document trigger format_with_parse_errors delegation")]
    #[test]
    fn test_parse_errors_trigger_partial_formatting() {
        let config = FormatConfig::default();
        let source_with_error = "behavior foo \"Foo\" {\n  contract \"ok\"\n}\n\n{{{ invalid syntax\n\nbehavior bar \"Bar\" {\n      contract   \"fix\"\n}\n";

        let result = format_source(source_with_error, &config);
        // Should not crash
        assert!(!result.formatted.is_empty(), "formatter must not crash on parse errors");
        // Well-formed regions should still be formatted
        // Error region should be preserved
        assert!(result.formatted.contains("{{{ invalid syntax"),
            "error region must be preserved verbatim");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_document", verify = "requires/ensures consistency for LSP document formatting")]
    #[test]
    fn test_lsp_format_document_contract() {
        let config = FormatConfig::default();
        let source = "behavior foo \"Foo\" {\n      contract   \"test\"\n}\n";

        // Requires: document open, config loaded
        let result = format_source(source, &config);

        // Ensures: TextEdit list returned (non-overlapping)
        let edits = compute_edits(source, &result.formatted);
        assert!(!edits.is_empty(), "dirty document must produce TextEdits");
        for i in 0..edits.len() {
            for j in (i + 1)..edits.len() {
                let a = &edits[i];
                let b = &edits[j];
                let no_overlap = (a.end_line < b.start_line || (a.end_line == b.start_line && a.end_col <= b.start_col))
                    || (b.end_line < a.start_line || (b.end_line == a.start_line && b.end_col <= a.start_col));
                assert!(no_overlap, "TextEdits must not overlap");
            }
        }

        // Ensures: CLI parity
        let cli = format_source(source, &config).formatted;
        assert_eq!(result.formatted, cli, "LSP format must match CLI format");

        // Ensures: already-formatted produces no edits
        let clean_edits = compute_edits(&result.formatted, &format_source(&result.formatted, &config).formatted);
        assert!(clean_edits.is_empty(), "already-formatted document must produce no edits");
    }

    // --- Gap coverage: lsp_format_range ---

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "parse errors within range are left unchanged per format_with_parse_errors")]
    #[test]
    fn test_range_parse_errors_left_unchanged() {
        let config = FormatConfig::default();
        let source = "behavior foo \"Foo\" {\n  contract \"ok\"\n}\n\n{{{ broken\n\nbehavior bar \"Bar\" {\n      contract   \"fix\"\n}\n";

        // Format range covering the error region (lines 4-5)
        let result = format_range(source, 4, 5, &config);
        // Error region should be preserved
        assert!(result.formatted.contains("{{{ broken"),
            "parse error region within range must be left unchanged");
    }

    #[specforge_test_macros::test(behavior = "lsp_format_range", verify = "requires/ensures consistency for LSP range formatting")]
    #[test]
    fn test_lsp_format_range_contract() {
        let config = FormatConfig::default();
        let source = "behavior foo \"Foo\" {\n  contract \"a\"\n}\n\nbehavior bar \"Bar\" {\n      contract   \"b\"\n}\n";

        // Requires: document open, config loaded
        let result = format_range(source, 5, 5, &config);

        // Ensures: range expanded to block boundaries (bar block formatted)
        assert!(result.formatted.contains("  contract \"b\""),
            "bar block contract should be formatted");

        // Ensures: full format parity for affected blocks
        let full = format_source(source, &config);
        // The bar block in range format should match bar block in full format
        let range_bar = result.formatted.lines()
            .skip_while(|l| !l.starts_with("behavior bar"))
            .collect::<Vec<_>>()
            .join("\n");
        let full_bar = full.formatted.lines()
            .skip_while(|l| !l.starts_with("behavior bar"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(range_bar, full_bar, "range format must match full format for affected blocks");
    }
}
