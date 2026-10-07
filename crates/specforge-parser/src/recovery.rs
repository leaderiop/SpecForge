//! Recovery from unclosed strings.
//!
//! Both string kinds may span lines, so an unclosed `"` or `"""` would run
//! to the end of the file, or pair with the next string's opening quote and
//! shift every later pairing, swallowing the blocks after it. The string
//! that was never closed is ended just before the next line that starts a
//! top-level block, and parsing resumes there.
//!
//! The scan runs only on files the grammar already rejected, and only acts
//! on a string left open at the end of the file or a multi-line string
//! whose closing quote runs straight into text (the shifted pairing): a
//! properly closed multi-line string whose content looks like a block start
//! is never split.

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

/// One string literal as the grammar's tokens would lex it.
struct StringToken {
    open: usize,
    delim: &'static str,
    /// Byte offset of the closing delimiter; `None` when unclosed at EOF.
    close: Option<usize>,
    /// Indentation of the top-level line the string sits under.
    top_indent: usize,
}

/// Find the strings to end early, in source order. Empty when no string
/// was left unclosed.
pub(crate) fn unclosed_strings(source: &str) -> Vec<UnclosedString> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut pos = 0;
    loop {
        let tokens = lex_strings(source, pos);
        // An unclosed regular string pairs with the next string's opening
        // quote, which shifts every later pairing: the first multi-line
        // "string" whose closing quote runs straight into text
        // (`"Broken {...behavior b "B`) is the one never closed. Failing
        // that, the string left open at the end of the file.
        let shifted = tokens.iter().find(|t| {
            t.close.is_some_and(|close| {
                bytes[t.open..close].contains(&b'\n')
                    && runs_into_text(bytes, close + t.delim.len())
            })
        });
        let Some(culprit) = shifted.or(tokens.last().filter(|t| t.close.is_none())) else {
            break;
        };
        let end = next_block_start(source, culprit).unwrap_or(source.len());
        found.push(UnclosedString {
            open: culprit.open,
            delim: culprit.delim,
            end,
        });
        if end == source.len() {
            break;
        }
        pos = end;
    }
    found
}

/// Whether the byte after a closing delimiter continues a token, which a
/// real closing quote never does: the grammar follows a string with
/// whitespace, a bracket, `,`, `|`, a comment, or the end of the file.
fn runs_into_text(bytes: &[u8], after: usize) -> bool {
    bytes.get(after).is_some_and(|&c| {
        !(c.is_ascii_whitespace()
            || matches!(
                c,
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b',' | b'|' | b'/'
            ))
    })
}

/// Lex `source[start..]` for string literals, skipping comments and
/// scheme-ref IDs (whose `//` is not a comment). `start` is a line start
/// at brace depth 0.
fn lex_strings(source: &str, start: usize) -> Vec<StringToken> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut depth = 0usize;
    let mut top_indent = 0usize;
    let mut line_start = start;
    let mut line_has_token = false;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            line_start = i + 1;
            line_has_token = false;
            i += 1;
            continue;
        }
        if b == b' ' || b == b'\t' || b == b'\r' {
            i += 1;
            continue;
        }
        if !line_has_token {
            line_has_token = true;
            if depth == 0 && !bytes[i..].starts_with(b"//") {
                top_indent = i - line_start;
            }
        }
        if bytes[i..].starts_with(b"//") {
            i = memchr(b'\n', bytes, i).unwrap_or(bytes.len());
        } else if bytes[i..].starts_with(b"\"\"\"") {
            let close = find(bytes, i + 3, b"\"\"\"");
            tokens.push(StringToken {
                open: i,
                delim: "\"\"\"",
                close,
                top_indent,
            });
            let Some(close) = close else { break };
            (line_start, line_has_token) = after_string(bytes, i, close, line_start);
            i = close + 3;
        } else if b == b'"' {
            let close = regular_close(bytes, i + 1);
            tokens.push(StringToken {
                open: i,
                delim: "\"",
                close,
                top_indent,
            });
            let Some(close) = close else { break };
            (line_start, line_has_token) = after_string(bytes, i, close, line_start);
            i = close + 1;
        } else if b == b'{' {
            depth += 1;
            i += 1;
        } else if b == b'}' {
            depth = depth.saturating_sub(1);
            i += 1;
        } else if is_ident_start(b) {
            i = skip_word(bytes, i);
        } else {
            i += 1;
        }
    }
    tokens
}

/// Line bookkeeping after a string spanning `open..close`.
fn after_string(bytes: &[u8], open: usize, close: usize, line_start: usize) -> (usize, bool) {
    match bytes[open..close].iter().rposition(|&c| c == b'\n') {
        Some(nl) => (open + nl + 1, true),
        None => (line_start, true),
    }
}

/// Skip an identifier, or a whole scheme-ref ID (`scheme.kind:rest`).
fn skip_word(bytes: &[u8], start: usize) -> usize {
    let ident_end = |from: usize| {
        let mut j = from;
        while j < bytes.len() && is_ident_char(bytes[j]) {
            j += 1;
        }
        j
    };
    let end = ident_end(start);
    if bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(|&c| is_ident_start(c)) {
        let kind_end = ident_end(end + 1);
        if bytes.get(kind_end) == Some(&b':') {
            let mut j = kind_end + 1;
            while j < bytes.len() && !is_ref_stop(bytes[j]) {
                j += 1;
            }
            if j > kind_end + 1 {
                return j;
            }
        }
    }
    end
}

/// The closing quote of a regular string whose content starts at `from`.
fn regular_close(bytes: &[u8], from: usize) -> Option<usize> {
    let mut j = from;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            b'"' => return Some(j),
            _ => j += 1,
        }
    }
    None
}

/// The first line after the string's opening that starts a top-level block.
fn next_block_start(source: &str, t: &StringToken) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = t.open + t.delim.len();
    while let Some(nl) = memchr(b'\n', bytes, i) {
        let line = nl + 1;
        if line >= bytes.len() {
            return None;
        }
        let line_end = memchr(b'\n', bytes, line).unwrap_or(bytes.len());
        if is_block_start(&source[line..line_end], t.top_indent) {
            return Some(line);
        }
        i = line;
    }
    None
}

/// Whether `line` opens a top-level block at indentation no deeper than
/// `max_indent`: `keyword id ["Title"] {`, `spec "Title" {`,
/// `define name {`, `ref scheme.kind:id "Title"`, or a `use` import.
fn is_block_start(line: &str, max_indent: usize) -> bool {
    let rest = line.trim_start_matches([' ', '\t']);
    if line.len() - rest.len() > max_indent {
        return false;
    }
    let mut cur = Cursor(rest);
    let Some(keyword) = cur.ident() else {
        return false;
    };
    match keyword {
        "use" => cur.ws() && cur.starts_import(),
        "pub" => cur.ws() && cur.ident() == Some("use") && cur.ws() && cur.starts_import(),
        "spec" => cur.ws() && cur.string() && cur.brace(),
        "ref" => cur.ws() && cur.scheme_ref() && cur.ws() && cur.string(),
        _ => {
            if !(cur.ws() && cur.ident().is_some()) {
                return false;
            }
            let before_title = cur.0;
            if !(cur.ws() && cur.string()) {
                cur.0 = before_title;
            }
            cur.brace()
        }
    }
}

/// A forward-only scanner over one line.
struct Cursor<'a>(&'a str);

impl<'a> Cursor<'a> {
    /// Consume at least one space or tab.
    fn ws(&mut self) -> bool {
        let rest = self.0.trim_start_matches([' ', '\t']);
        let moved = rest.len() < self.0.len();
        self.0 = rest;
        moved
    }

    fn ident(&mut self) -> Option<&'a str> {
        let bytes = self.0.as_bytes();
        if !bytes.first().is_some_and(|&c| is_ident_start(c)) {
            return None;
        }
        let end = bytes
            .iter()
            .position(|&c| !is_ident_char(c))
            .unwrap_or(bytes.len());
        let (word, rest) = self.0.split_at(end);
        self.0 = rest;
        Some(word)
    }

    /// A string literal closed on this line.
    fn string(&mut self) -> bool {
        let bytes = self.0.as_bytes();
        if bytes.first() != Some(&b'"') {
            return false;
        }
        match regular_close(bytes, 1) {
            Some(close) => {
                self.0 = &self.0[close + 1..];
                true
            }
            None => false,
        }
    }

    fn scheme_ref(&mut self) -> bool {
        let bytes = self.0.as_bytes();
        if !bytes.first().is_some_and(|&c| is_ident_start(c)) {
            return false;
        }
        let end = skip_word(bytes, 0);
        let is_ref = self.0[..end].contains(':');
        self.0 = &self.0[end..];
        is_ref
    }

    /// Optional spaces, then `{`.
    fn brace(&mut self) -> bool {
        self.ws();
        self.0.starts_with('{')
    }

    /// The start of an import's target: a path, bindings or `* as`.
    fn starts_import(&self) -> bool {
        self.0.starts_with(['"', '{', '*'])
    }
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Characters that end a scheme-ref ID (the grammar's `[^\s"{}()\[\],]+`).
fn is_ref_stop(c: u8) -> bool {
    c.is_ascii_whitespace() || matches!(c, b'"' | b'{' | b'}' | b'(' | b')' | b'[' | b']' | b',')
}

fn memchr(needle: u8, bytes: &[u8], from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .iter()
        .position(|&c| c == needle)
        .map(|p| from + p)
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| from + p)
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
    fn the_lexer_agrees_with_the_grammar_on_the_corpora() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        for corpus in ["spec", "integrations/rust/spec", "examples"] {
            collect_specs(&root.join(corpus), &mut files);
        }
        assert!(files.len() > 20, "found only {} corpus files", files.len());
        for file in files {
            let src = std::fs::read_to_string(&file).unwrap();
            for t in lex_strings(&src, 0) {
                let close = t.close.unwrap_or_else(|| {
                    panic!("{}: string at byte {} unclosed", file.display(), t.open)
                });
                assert!(
                    !runs_into_text(src.as_bytes(), close + t.delim.len()),
                    "{}: string at byte {} runs into text",
                    file.display(),
                    t.open
                );
            }
        }
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
    }

    #[test]
    fn slashes_in_a_scheme_ref_are_not_a_comment() {
        let src = "ref web.page:https://x.io \"T\"\nbehavior a \"A\" {\n}\n";
        assert!(unclosed_strings(src).is_empty());
    }

    // Pin: flipped by plan 15 T5.
    #[test]
    fn the_corpora_s_unions_are_no_resume_points_today() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        for corpus in ["spec", "integrations/rust/spec", "examples"] {
            collect_specs(&root.join(corpus), &mut files);
        }
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
                    missed.insert(node.kind());
                }
            }
        }
        assert_eq!(missed, std::collections::BTreeSet::from(["union_block"]));
    }
}
