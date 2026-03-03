//! Tree-sitter grammar for the SpecForge `.spec` DSL.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_specforge() -> *const ();
}

/// Returns the tree-sitter [`LanguageFn`] for the SpecForge grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_specforge) };

/// Convenience: the node-types JSON (embedded at compile time).
pub const NODE_TYPES: &str = include_str!("node-types.json");

#[cfg(test)]
mod tests {
    use super::*;

    fn make_parser() -> tree_sitter::Parser {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&LANGUAGE.into())
            .expect("failed to set language");
        parser
    }

    #[test]
    fn can_load_language() {
        let _lang = LANGUAGE.into_raw();
    }

    #[test]
    fn parse_simple_invariant() {
        let mut parser = make_parser();

        let source = r#"invariant data_persistence "Test" {
  guarantee """
    Must be true.
  """
  risk high
}"#;

        let tree = parser.parse(source, None).expect("parse failed");
        let root = tree.root_node();
        assert_eq!(root.kind(), "source_file");
        assert!(!root.has_error());
        assert_eq!(root.child_count(), 1);

        let invariant = root.child(0).unwrap();
        assert_eq!(invariant.kind(), "invariant_block");
    }

    #[test]
    fn parse_use_import() {
        let mut parser = make_parser();

        let source = "use invariants/core\nuse types/errors";
        let tree = parser.parse(source, None).unwrap();
        assert!(!tree.root_node().has_error());
        assert_eq!(tree.root_node().named_child_count(), 2);
    }

    // ── Query file tests ───────────────────────────────────────

    #[test]
    fn query_files_parse_without_errors() {
        let lang: tree_sitter::Language = LANGUAGE.into();
        let queries = [
            ("highlights.scm", include_str!("../queries/highlights.scm")),
            ("folds.scm", include_str!("../queries/folds.scm")),
            ("indents.scm", include_str!("../queries/indents.scm")),
            ("injections.scm", include_str!("../queries/injections.scm")),
        ];
        for (name, source) in queries {
            tree_sitter::Query::new(&lang, source)
                .unwrap_or_else(|e| panic!("{name} failed to parse: {e}"));
        }
    }

    #[test]
    fn folds_capture_all_block_types() {
        use streaming_iterator::StreamingIterator;

        let mut parser = make_parser();
        let lang: tree_sitter::Language = LANGUAGE.into();

        // One instance of each of the 16 block types.
        // failure_mode includes post_mitigation to produce a nested_block too.
        let source = r#"
spec "test" { version "1.0" }
invariant data_persistence "T" { risk high }
behavior validate_input "T" { contract """x""" }
feature input_validation "T" { behaviors [validate_input] }
event file_parsed "T" { channel "c" }
type User { id string }
port FileSystem { direction outbound }
ref figma.frame:abc "T" { status reviewed }
capability user_management "T" { persona developer }
deliverable cli_binary "T" { type cli }
roadmap foundation "T" { status done }
library core_lib "T" { family core }
glossary { term "x" { definition """d""" } }
decision parser_strategy "T" { status accepted }
constraint response_time "T" { category performance }
failure_mode data_corruption "T" { severity 8 post_mitigation { severity 2 } }
"#;

        let tree = parser.parse(source, None).expect("parse failed");
        assert!(!tree.root_node().has_error(), "source has parse errors");

        let query = tree_sitter::Query::new(&lang, include_str!("../queries/folds.scm"))
            .expect("folds.scm failed to parse");
        let fold_idx = query
            .capture_index_for_name("fold")
            .expect("no @fold capture");

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut match_count = 0u32;
        while let Some(m) = matches.next() {
            assert!(
                m.captures.iter().any(|c| c.index == fold_idx),
                "match without @fold capture"
            );
            match_count += 1;
        }

        // 16 block types + nested_block + term_def = at least 18 folds
        assert!(
            match_count >= 18,
            "expected >= 18 fold matches, got {match_count}"
        );
    }

    #[test]
    fn indents_capture_brackets() {
        use streaming_iterator::StreamingIterator;

        let mut parser = make_parser();
        let lang: tree_sitter::Language = LANGUAGE.into();

        let source = r#"spec "test" {
  version "1.0"
  tags ["a", "b"]
}"#;

        let tree = parser.parse(source, None).expect("parse failed");
        assert!(!tree.root_node().has_error());

        let query = tree_sitter::Query::new(&lang, include_str!("../queries/indents.scm"))
            .expect("indents.scm failed to parse");
        let indent_idx = query
            .capture_index_for_name("indent")
            .expect("no @indent capture");
        let dedent_idx = query
            .capture_index_for_name("dedent")
            .expect("no @dedent capture");

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut indent_count = 0u32;
        let mut dedent_count = 0u32;
        while let Some(m) = matches.next() {
            for c in m.captures {
                if c.index == indent_idx {
                    indent_count += 1;
                } else if c.index == dedent_idx {
                    dedent_count += 1;
                }
            }
        }

        // source has: 1 `{`, 1 `[`, 1 `]`, 1 `}` → 2 indents, 2 dedents
        assert_eq!(indent_count, 2, "expected 2 @indent captures");
        assert_eq!(dedent_count, 2, "expected 2 @dedent captures");
        assert_eq!(indent_count, dedent_count, "indent/dedent count mismatch");
    }

    #[test]
    fn injections_capture_triple_quoted_strings() {
        use streaming_iterator::StreamingIterator;

        let mut parser = make_parser();
        let lang: tree_sitter::Language = LANGUAGE.into();

        let source = r#"invariant data_persistence "T" {
  guarantee """
    Must hold.
  """
  risk high
}
feature input_validation "T" {
  problem """
    Something.
  """
  solution """
    Fix it.
  """
  behaviors [validate_input]
}"#;

        let tree = parser.parse(source, None).expect("parse failed");
        assert!(!tree.root_node().has_error());

        let query = tree_sitter::Query::new(&lang, include_str!("../queries/injections.scm"))
            .expect("injections.scm failed to parse");
        let content_idx = query
            .capture_index_for_name("injection.content")
            .expect("no @injection.content capture");

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut match_count = 0u32;
        while let Some(m) = matches.next() {
            let cap = m
                .captures
                .iter()
                .find(|c| c.index == content_idx)
                .expect("match missing @injection.content");
            assert_eq!(
                cap.node.kind(),
                "triple_quoted_string",
                "injection capture should be a triple_quoted_string node"
            );
            match_count += 1;
        }

        // 3 triple-quoted strings in source
        assert_eq!(
            match_count, 3,
            "expected 3 injection matches, got {match_count}"
        );
    }
}
