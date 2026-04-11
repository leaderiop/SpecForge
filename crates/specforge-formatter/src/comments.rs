use std::collections::HashMap;
use tree_sitter::Node;

/// Classification of a comment's attachment to surrounding nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentKind {
    /// Comment on its own line(s) immediately before a node.
    Leading,
    /// Comment at end of same line as a node.
    Trailing,
    /// Standalone comment block separated by blank lines from both adjacent blocks.
    Standalone,
}

/// A comment with its text, position, and attachment classification.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AttachedComment {
    pub text: String,
    pub start_row: usize,
    pub end_row: usize,
    pub kind: CommentKind,
    /// The node ID this comment is attached to (if Leading or Trailing).
    pub attached_to: Option<usize>,
}

/// Map from node IDs to their attached comments.
pub struct CommentMap {
    /// Leading comments keyed by the node they precede.
    pub leading: HashMap<usize, Vec<AttachedComment>>,
    /// Trailing comments keyed by the node they follow.
    pub trailing: HashMap<usize, Vec<AttachedComment>>,
    /// Standalone comments not attached to any specific node.
    pub standalone: Vec<AttachedComment>,
}

impl CommentMap {
    pub fn new() -> Self {
        Self {
            leading: HashMap::new(),
            trailing: HashMap::new(),
            standalone: Vec::new(),
        }
    }
}

/// Build a comment map from a tree-sitter CST.
///
/// Algorithm:
/// 1. Collect all comment nodes and non-comment sibling nodes.
/// 2. For each comment:
///    - If on the same line as a preceding non-comment token → Trailing
///    - If followed by a non-comment node on the next line(s) → Leading
///    - Otherwise → Standalone
pub fn build_comment_map(root: Node, source: &str) -> CommentMap {
    let mut map = CommentMap::new();
    let mut comments: Vec<(Node, String)> = Vec::new();
    let mut non_comments: Vec<Node> = Vec::new();

    collect_nodes(root, source, &mut comments, &mut non_comments);

    for (comment_node, text) in &comments {
        let comment_row = comment_node.start_position().row;

        // Check if trailing: is there a non-comment node ending on the same line before this comment?
        let is_trailing = non_comments.iter().any(|n| {
            n.end_position().row == comment_row
                && n.end_byte() <= comment_node.start_byte()
        });

        if is_trailing {
            // Attach as trailing to the last non-comment node on same line before this comment
            if let Some(prev) = non_comments
                .iter()
                .rfind(|n| {
                    n.end_position().row == comment_row
                        && n.end_byte() <= comment_node.start_byte()
                })
            {
                let ac = AttachedComment {
                    text: text.clone(),
                    start_row: comment_row,
                    end_row: comment_node.end_position().row,
                    kind: CommentKind::Trailing,
                    attached_to: Some(prev.id()),
                };
                map.trailing.entry(prev.id()).or_default().push(ac);
            }
            continue;
        }

        // Check if leading: is there a non-comment node on a subsequent line?
        let next_node = non_comments
            .iter()
            .find(|n| n.start_position().row > comment_row);

        if let Some(next) = next_node {
            // Check if there's a blank line gap between comment and next node
            let gap = next.start_position().row.saturating_sub(comment_node.end_position().row);

            // If there's a blank line gap AND a preceding node also with a gap, it's standalone
            let prev_node = non_comments
                .iter()
                .rfind(|n| n.end_position().row < comment_row);

            let is_standalone = if let Some(prev) = prev_node {
                let gap_before = comment_row.saturating_sub(prev.end_position().row);
                gap_before > 1 && gap > 1
            } else {
                false
            };

            if is_standalone {
                map.standalone.push(AttachedComment {
                    text: text.clone(),
                    start_row: comment_row,
                    end_row: comment_node.end_position().row,
                    kind: CommentKind::Standalone,
                    attached_to: None,
                });
            } else {
                let ac = AttachedComment {
                    text: text.clone(),
                    start_row: comment_row,
                    end_row: comment_node.end_position().row,
                    kind: CommentKind::Leading,
                    attached_to: Some(next.id()),
                };
                map.leading.entry(next.id()).or_default().push(ac);
            }
        } else {
            // No following node — standalone
            map.standalone.push(AttachedComment {
                text: text.clone(),
                start_row: comment_row,
                end_row: comment_node.end_position().row,
                kind: CommentKind::Standalone,
                attached_to: None,
            });
        }
    }

    map
}

/// Collect comment and non-comment top-level/field-level nodes.
fn collect_nodes<'a>(
    root: Node<'a>,
    source: &str,
    comments: &mut Vec<(Node<'a>, String)>,
    non_comments: &mut Vec<Node<'a>>,
) {
    let mut cursor = root.walk();
    collect_recursive(root, &mut cursor, source, comments, non_comments);
}

fn collect_recursive<'a>(
    node: Node<'a>,
    cursor: &mut tree_sitter::TreeCursor<'a>,
    source: &str,
    comments: &mut Vec<(Node<'a>, String)>,
    non_comments: &mut Vec<Node<'a>>,
) {
    if node.kind() == "comment" {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
        comments.push((node, text));
        return;
    }

    if node.kind() != "source_file" {
        non_comments.push(node);
    }

    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let mut child_cursor = child.walk();
            collect_recursive(child, &mut child_cursor, source, comments, non_comments);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_source(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_specforge::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "leading comment attaches to following node")]
    #[test]
    fn test_leading_comment_attaches_to_following_node() {
        let source = "// This describes foo\nbehavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n";
        let tree = parse_source(source);
        let map = build_comment_map(tree.root_node(), source);

        assert_eq!(map.standalone.len(), 0);
        assert!(!map.leading.is_empty(), "should have a leading comment");
        let leading_comments: Vec<_> = map.leading.values().flatten().collect();
        assert_eq!(leading_comments.len(), 1);
        assert_eq!(leading_comments[0].text, "// This describes foo");
        assert_eq!(leading_comments[0].kind, CommentKind::Leading);
    }

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "trailing comment attaches to preceding node on same line")]
    #[test]
    fn test_trailing_comment_attaches_to_preceding_node_on_same_line() {
        let source = "behavior foo \"Foo\" { // inline comment\n  contract \"does stuff\"\n}\n";
        let tree = parse_source(source);
        let map = build_comment_map(tree.root_node(), source);

        let trailing_comments: Vec<_> = map.trailing.values().flatten().collect();
        assert!(!trailing_comments.is_empty(), "should have a trailing comment");
        assert_eq!(trailing_comments[0].text, "// inline comment");
        assert_eq!(trailing_comments[0].kind, CommentKind::Trailing);
    }

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "standalone comment block between blocks is preserved")]
    #[test]
    fn test_standalone_comment_block_between_blocks_is_preserved() {
        let source = concat!(
            "behavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n",
            "\n\n",
            "// standalone note\n",
            "\n\n",
            "behavior bar \"Bar\" {\n  contract \"does things\"\n}\n",
        );
        let tree = parse_source(source);
        let map = build_comment_map(tree.root_node(), source);

        assert!(!map.standalone.is_empty(), "should have a standalone comment");
        assert_eq!(map.standalone[0].text, "// standalone note");
        assert_eq!(map.standalone[0].kind, CommentKind::Standalone);
    }

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "section header comment attaches to next block group")]
    #[test]
    fn test_section_header_comment_attaches_to_next_block_group() {
        let source = "// Section: behaviors\nbehavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n";
        let tree = parse_source(source);
        let map = build_comment_map(tree.root_node(), source);

        // Section header is a leading comment (no blank line gap on both sides)
        let leading_comments: Vec<_> = map.leading.values().flatten().collect();
        assert!(!leading_comments.is_empty());
        assert_eq!(leading_comments[0].kind, CommentKind::Leading);
    }

    // --- Property: no comments lost after formatting (comment map level) ---

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "no comments are lost after formatting")]
    #[test]
    fn test_no_comments_lost_in_comment_map() {
        // All comment tokens should be captured by the comment map
        let source = concat!(
            "// header comment\n",
            "behavior foo \"Foo\" { // trailing comment\n",
            "  contract \"stuff\"\n",
            "}\n",
            "\n",
            "\n",
            "// standalone comment\n",
            "\n",
            "\n",
            "// leading comment for bar\n",
            "behavior bar \"Bar\" {\n",
            "  contract \"things\"\n",
            "}\n",
        );

        let tree = parse_source(source);
        let map = build_comment_map(tree.root_node(), source);

        let total_leading: usize = map.leading.values().map(|v| v.len()).sum();
        let total_trailing: usize = map.trailing.values().map(|v| v.len()).sum();
        let total_standalone = map.standalone.len();
        let total = total_leading + total_trailing + total_standalone;

        // Count actual comment tokens in source
        let comment_count = source.lines().filter(|l| {
            let t = l.trim();
            t.starts_with("//")
        }).count();
        // Also count inline comments (trailing on same line as code)
        let inline_count = source.lines().filter(|l| {
            let t = l.trim();
            !t.starts_with("//") && t.contains("//")
        }).count();

        assert_eq!(total, comment_count + inline_count,
            "all comments should be captured: got {total} (leading={total_leading}, trailing={total_trailing}, standalone={total_standalone}), expected {}",
            comment_count + inline_count);
    }

    // --- Contract: preserve_comments ---

    #[specforge_test_macros::test(behavior = "preserve_comments", verify = "requires/ensures consistency for comment preservation")]
    #[test]
    fn test_preserve_comments_contract() {
        // requires: cst_available (we parse the source)
        let source = "// leading\nbehavior foo \"Foo\" { // trailing\n  contract \"stuff\"\n}\n";
        let tree = parse_source(source);
        let map = build_comment_map(tree.root_node(), source);

        // ensures: all_comments_attached
        let total_leading: usize = map.leading.values().map(|v| v.len()).sum();
        let total_trailing: usize = map.trailing.values().map(|v| v.len()).sum();
        assert!(total_leading + total_trailing > 0, "comments should be attached");

        // ensures: no_comments_lost
        let leading_texts: Vec<&str> = map.leading.values().flatten().map(|c| c.text.as_str()).collect();
        let trailing_texts: Vec<&str> = map.trailing.values().flatten().map(|c| c.text.as_str()).collect();
        assert!(leading_texts.contains(&"// leading"), "leading comment should be preserved");
        assert!(trailing_texts.contains(&"// trailing"), "trailing comment should be preserved");
    }
}
