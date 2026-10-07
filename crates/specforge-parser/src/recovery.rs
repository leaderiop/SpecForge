//! Recovery from unclosed strings.
//!
//! Both string kinds may span lines, so an unclosed `"` or `"""` would run
//! to the end of the file, or pair with the next string's opening quote and
//! shift every later pairing, swallowing the blocks after it. The string
//! that was never closed is ended just before the next line that starts a
//! top-level form of the grammar, and parsing resumes there.
//!
//! The text is read through the language's one lexer (`crate::lex`), whose
//! strings are the grammar's: the culprit is the first string it reads as
//! unclosed (no closing quote, a `\` before a line break, or a multi-line
//! pairing whose closing quote runs into text), or a multi-line `"""…"""`
//! whose closing quotes run into text. The scan runs only on files the
//! grammar already rejected; a properly closed multi-line string whose
//! content looks like a block start is never split. A test holds the resume
//! lines to the top-level forms the grammar reads in the repository's spec.

use crate::lex::{self, LexemeKind, lex};

const TRIPLE: &str = "\"\"\"";

/// A string the recovery ends early.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnclosedString {
    /// Byte offset of the opening quote.
    pub open: usize,
    /// The delimiter: `"` or `"""`.
    pub delim: &'static str,
    /// Byte offset where the string is ended: the start of the next
    /// block-start line, or the end of the source when none follows.
    pub end: usize,
}

/// Find the strings to end early, in source order. Empty when no string
/// was left unclosed.
pub(crate) fn unclosed_strings(source: &str) -> Vec<UnclosedString> {
    let mut found = Vec::new();
    let mut pos = 0;
    while let Some((open, delim, top_indent)) = culprit(source, pos) {
        let end = next_block_start(source, open + delim.len(), top_indent).unwrap_or(source.len());
        found.push(UnclosedString { open, delim, end });
        if end == source.len() {
            break;
        }
        pos = end;
    }
    found
}

/// The first string of `source[from..]` the grammar does not close, as (the
/// opening quote's offset in `source`, its delimiter, the indentation of the
/// top-level line it sits under). `from` is a line start at brace depth 0.
fn culprit(source: &str, from: usize) -> Option<(usize, &'static str, usize)> {
    let text = &source[from..];
    let mut depth = 0usize;
    let mut top_indent = 0usize;
    let mut previous_end: Option<usize> = None;
    for lexeme in lex(text) {
        let first_on_line = previous_end.is_none_or(|end| text[end..lexeme.start].contains('\n'));
        if first_on_line && depth == 0 && lexeme.kind != LexemeKind::Comment {
            let line_start = text[..lexeme.start].rfind('\n').map_or(0, |nl| nl + 1);
            top_indent = lexeme.start - line_start;
        }
        match lexeme.kind {
            LexemeKind::Punct('{') => depth += 1,
            LexemeKind::Punct('}') => depth = depth.saturating_sub(1),
            LexemeKind::Str { closed } => {
                let string = lexeme.text(text);
                let delim = if string.starts_with(TRIPLE) {
                    TRIPLE
                } else {
                    "\""
                };
                // An unclosed `"""` pairs with the next one's opening quotes:
                // the multi-line "string" whose closing quotes run straight
                // into text is the one never closed.
                let shifted = closed
                    && delim == TRIPLE
                    && string.contains('\n')
                    && lex::runs_into_text(text, lexeme.end);
                if !closed || shifted {
                    return Some((from + lexeme.start, delim, top_indent));
                }
            }
            _ => {}
        }
        previous_end = Some(lexeme.end);
    }
    None
}

/// The first line after `from` that starts a top-level form at indentation
/// no deeper than `max_indent`.
fn next_block_start(source: &str, from: usize, max_indent: usize) -> Option<usize> {
    let mut at = from;
    while let Some(nl) = source[at..].find('\n') {
        let line = at + nl + 1;
        if line >= source.len() {
            return None;
        }
        let line_end = source[line..]
            .find('\n')
            .map_or(source.len(), |nl| line + nl);
        if is_block_start(&source[line..line_end], max_indent) {
            return Some(line);
        }
        at = line;
    }
    None
}

/// Whether `line` opens a top-level form of the grammar (a child of
/// `source_file`) at indentation no deeper than `max_indent`: `use …`,
/// `pub use …`, `spec "Title" {`, `ref scheme.kind:id "Title"`,
/// `kind name ["Title"] {` (a define block too) or `kind name =` (a union).
fn is_block_start(line: &str, max_indent: usize) -> bool {
    let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
    if indent > max_indent {
        return false;
    }
    let lexemes = lex(line);
    let kind = |i: usize| lexemes.get(i).map(|l| l.kind);
    let word = |i: usize| {
        lexemes
            .get(i)
            .filter(|l| l.kind == LexemeKind::Ident)
            .map(|l| l.text(line))
    };
    // A header's title: a `"…"` closed on this line.
    let title = |i: usize| {
        lexemes.get(i).is_some_and(|l| {
            l.kind == LexemeKind::Str { closed: true } && !l.text(line).starts_with(TRIPLE)
        })
    };
    let punct = |i: usize, c: char| kind(i) == Some(LexemeKind::Punct(c));
    // An import's target: a path, bindings or `* as`.
    let import =
        |i: usize| kind(i).is_some_and(LexemeKind::is_str) || punct(i, '{') || punct(i, '*');
    match word(0) {
        Some("use") => import(1),
        Some("pub") => word(1) == Some("use") && import(2),
        Some("spec") => title(1) && punct(2, '{'),
        Some("ref") => kind(1) == Some(LexemeKind::RefId) && title(2),
        Some(_) if word(1).is_some() => punct(if title(2) { 3 } else { 2 }, '{') || punct(2, '='),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_file_needs_no_recovery() {
        let src = "behavior a \"A\" {\n  contract \"\"\"\nbehavior b \"B\" {\n  \"\"\"\n}\n";
        assert!(unclosed_strings(src).is_empty());
    }

    #[test]
    fn an_unclosed_triple_string_ends_before_the_next_block() {
        let src = "behavior a \"A\" {\n  contract \"\"\"\n  text\n}\n\nbehavior b \"B\" {\n}\n";
        let open = src.find("\"\"\"").unwrap();
        let end = src.find("behavior b").unwrap();
        assert_eq!(
            unclosed_strings(src),
            vec![UnclosedString {
                open,
                delim: "\"\"\"",
                end
            }]
        );
    }

    #[test]
    fn a_shifted_title_pairing_is_blamed_on_the_first_string() {
        let src = "behavior a \"A {\n}\n\nbehavior b \"B\" {\n  status done\n}\n";
        let end = src.find("behavior b").unwrap();
        assert_eq!(
            unclosed_strings(src),
            vec![UnclosedString {
                open: 11,
                delim: "\"",
                end
            }]
        );
    }

    #[test]
    fn a_later_string_does_not_take_the_blame() {
        let src = "behavior a \"A {\n  contract \"Oops.\"\n}\n\nbehavior b \"B\" {\n}\n";
        let end = src.find("behavior b").unwrap();
        assert_eq!(
            unclosed_strings(src),
            vec![UnclosedString {
                open: 11,
                delim: "\"",
                end
            }]
        );
    }

    #[test]
    fn text_after_a_one_line_string_is_not_an_unclosed_string() {
        let src = "behavior a \"A\" {\n  status \"done\"x\n}\n\nbehavior b \"B\" {\n}\n";
        assert!(unclosed_strings(src).is_empty());
    }

    #[test]
    fn each_unclosed_string_is_ended() {
        let src = "a x \"X {\n}\nb y \"Y\" {\n  c \"\"\"\n  text\n}\nc z \"Z\" {\n}\n";
        let found = unclosed_strings(src);
        let ends: Vec<usize> = found.iter().map(|u| u.end).collect();
        assert_eq!(
            ends,
            vec![src.find("b y").unwrap(), src.find("c z").unwrap()]
        );
        assert_eq!(found[1].delim, "\"\"\"");
    }

    /// Every corpus file lexes the way the grammar does: all strings closed,
    /// none running into text. Recovery never fires on a valid file anyway
    /// (it waits for a grammar error), but its blame rule relies on this.
    #[test]
    fn no_string_of_the_corpora_runs_into_text() {
        let mut files = Vec::new();
        for corpus in CORPORA {
            collect_specs(&root().join(corpus), &mut files);
        }
        assert!(files.len() > 20, "found only {} corpus files", files.len());
        for file in files {
            let src = std::fs::read_to_string(&file).unwrap();
            for lexeme in lex(&src) {
                let LexemeKind::Str { closed } = lexeme.kind else {
                    continue;
                };
                assert!(
                    closed,
                    "{}: string at byte {} unclosed",
                    file.display(),
                    lexeme.start
                );
                assert!(
                    !lex::runs_into_text(&src, lexeme.end),
                    "{}: string at byte {} runs into text",
                    file.display(),
                    lexeme.start
                );
            }
        }
    }

    const CORPORA: [&str; 3] = ["spec", "integrations/rust/spec", "examples"];

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn collect_specs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_specs(&path, out);
            } else if path.extension().is_some_and(|e| e == "spec") {
                out.push(path);
            }
        }
    }

    #[test]
    fn with_no_block_after_it_the_string_runs_to_the_end() {
        let src = "behavior a \"A\" {\n  contract \"oops\n}\n";
        assert_eq!(
            unclosed_strings(src),
            vec![UnclosedString {
                open: 28,
                delim: "\"",
                end: src.len()
            }]
        );
    }

    #[test]
    fn block_start_lines() {
        assert!(is_block_start("behavior b \"B\" {", 0));
        assert!(is_block_start("type t {", 0));
        assert!(is_block_start("spec \"S\" {", 0));
        assert!(is_block_start("ref github.issue:org/repo#1 \"T\"", 0));
        assert!(is_block_start("use \"./a.spec\"", 0));
        assert!(is_block_start("pub use { A } from \"./a.spec\"", 0));
        assert!(!is_block_start("  behavior b \"B\" {", 0));
        assert!(is_block_start("  behavior b \"B\" {", 2));
        assert!(!is_block_start("the quick brown fox", 0));
        assert!(!is_block_start("status done", 0));
        assert!(!is_block_start("behavior b \"B", 0));
        assert!(is_block_start("type status = \"a\" | \"b\"", 0));
        assert!(is_block_start("type t =", 0));
        assert!(is_block_start("use\"./a.spec\"", 0));
    }

    #[test]
    fn slashes_in_a_scheme_ref_are_not_a_comment() {
        let src = "ref web.page:https://x.io \"T\"\nbehavior a \"A\" {\n}\n";
        assert!(unclosed_strings(src).is_empty());
    }

    #[specforge_test_macros::test(
        behavior = "recover_from_syntax_errors",
        verify = "recovery resumes at every top-level form the grammar reads, a union block included"
    )]
    fn every_top_level_form_of_the_corpora_is_a_resume_point() {
        let mut files = Vec::new();
        for corpus in CORPORA {
            collect_specs(&root().join(corpus), &mut files);
        }
        files.push(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lexemes.spec"),
        );
        let mut missed = std::collections::BTreeSet::new();
        for file in files {
            let src = std::fs::read_to_string(&file).unwrap();
            let (_, tree) = crate::parse_incremental(&src, &file.to_string_lossy(), None);
            let tree = tree.unwrap();
            let root = tree.root_node();
            let mut cursor = root.walk();
            for node in root.named_children(&mut cursor) {
                if node.kind() == tree_sitter_specforge::kind::COMMENT {
                    continue;
                }
                let line = src.lines().nth(node.start_position().row).unwrap();
                let column = node.start_position().column;
                assert!(
                    line[..column].trim().is_empty(),
                    "{}: two forms on one line",
                    file.display()
                );
                if !is_block_start(line, column) {
                    missed.insert(format!("{}: {}", node.kind(), line));
                }
            }
        }
        assert!(missed.is_empty(), "not resume points: {missed:#?}");
    }
}
