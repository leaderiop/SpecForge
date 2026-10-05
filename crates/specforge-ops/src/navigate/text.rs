//! A spec file's text, addressed in `SourceSpan` positions (1-based lines,
//! 1-based byte columns, end exclusive), and the lexing navigation needs:
//! the identifier tokens of a range, outside strings and comments.

use specforge_common::{SourceSpan, Sym};

/// A file's text and where each of its lines starts.
#[derive(Debug)]
pub(crate) struct SourceText {
    text: String,
    /// Byte offset of each line's first byte.
    line_starts: Vec<usize>,
}

impl SourceText {
    pub(crate) fn new(text: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        SourceText { text, line_starts }
    }

    /// The byte offset of 1-based (`line`, `col`); `None` past the line's
    /// end (its newline excluded) or the file's.
    pub(crate) fn offset(&self, line: usize, col: usize) -> Option<usize> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let end = self.line_end(line)?;
        let offset = start + col.checked_sub(1)?;
        (offset <= end).then_some(offset)
    }

    /// The byte offset just past 1-based `line`'s text (before its
    /// newline).
    fn line_end(&self, line: usize) -> Option<usize> {
        let next = self.line_starts.get(line).copied();
        Some(match next {
            Some(next) => next - 1,
            None => {
                self.line_starts.get(line.checked_sub(1)?)?;
                self.text.len()
            }
        })
    }

    /// The 1-based (line, column) of byte `offset`.
    pub(crate) fn position(&self, offset: usize) -> (usize, usize) {
        let index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index - 1,
        };
        (index + 1, offset - self.line_starts[index] + 1)
    }

    /// The span of bytes `start..end` in `file`.
    pub(crate) fn span(&self, file: Sym, start: usize, end: usize) -> SourceSpan {
        let (start_line, start_col) = self.position(start);
        let (end_line, end_col) = self.position(end);
        SourceSpan {
            file,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// The byte range `span` covers, when it lies in the text.
    pub(crate) fn range(&self, span: &SourceSpan) -> Option<std::ops::Range<usize>> {
        let start = self.offset(span.start_line, span.start_col)?;
        let end = self.offset(span.end_line, span.end_col)?;
        (start <= end).then_some(start..end)
    }

    /// The bytes of 1-based lines `first..=last`, newlines excluded at
    /// the end.
    pub(crate) fn lines(&self, first: usize, last: usize) -> Option<std::ops::Range<usize>> {
        let start = *self.line_starts.get(first.checked_sub(1)?)?;
        let end = self.line_end(last)?;
        (start <= end).then_some(start..end)
    }

    /// The text of bytes `start..end`.
    pub(crate) fn slice(&self, start: usize, end: usize) -> Option<&str> {
        self.text.get(start..end)
    }

    /// The end of the text, as an empty span: where an insertion at the
    /// end of the file goes.
    pub(crate) fn end(&self, file: Sym) -> SourceSpan {
        self.span(file, self.text.len(), self.text.len())
    }

    /// Whether the text at `span` is exactly `word`.
    pub(crate) fn spells(&self, span: &SourceSpan, word: &str) -> bool {
        self.range(span)
            .is_some_and(|range| self.text.get(range) == Some(word))
    }

    /// The identifier tokens of `span`, outside strings and comments, as
    /// byte ranges of the file.
    pub(crate) fn tokens(&self, span: &SourceSpan) -> Vec<Token> {
        let Some(range) = self.range(span) else {
            return Vec::new();
        };
        lex(&self.text[range.clone()])
            .into_iter()
            .map(|t| Token {
                start: t.start + range.start,
                end: t.end + range.start,
                kind: t.kind,
            })
            .collect()
    }

    /// The text of `token`.
    pub(crate) fn token_text(&self, token: &Token) -> &str {
        &self.text[token.start..token.end]
    }
}

/// What a lexeme is: an identifier (letters, digits, `_`), or one
/// punctuation character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident,
    Punct(char),
}

/// One lexeme, as a byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: TokenKind,
}

/// A byte that continues an identifier: what the grammar's identifiers are
/// made of, and any non-ASCII byte, so a token never splits a word.
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// The lexemes of `text`: identifiers and punctuation, skipping
/// whitespace, `"…"` strings (with `\` escapes), `"""…"""` strings and `//`
/// comments to the end of the line. Text that starts inside a string is
/// read as code: callers lex from an entity, field value or method start.
pub(crate) fn lex(text: &str) -> Vec<Token> {
    let bytes = text.as_bytes();
    let find = |from: usize, needle: &[u8]| {
        bytes[from.min(bytes.len())..]
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|at| from + at)
    };
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte.is_ascii_whitespace() {
            i += 1;
        } else if bytes[i..].starts_with(b"\"\"\"") {
            i = find(i + 3, b"\"\"\"").map_or(bytes.len(), |end| end + 3);
        } else if byte == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\n' {
                i += if bytes[i] == b'\\' { 2 } else { 1 };
            }
            i = (i + 1).min(bytes.len());
        } else if bytes[i..].starts_with(b"//") {
            i = find(i, b"\n").unwrap_or(bytes.len());
        } else if is_word(byte) {
            let start = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            tokens.push(Token {
                start,
                end: i,
                kind: TokenKind::Ident,
            });
        } else {
            // Every non-ASCII byte is a word byte: punctuation is ASCII.
            tokens.push(Token {
                start: i,
                end: i + 1,
                kind: TokenKind::Punct(char::from(byte)),
            });
            i += 1;
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idents(text: &str) -> Vec<&str> {
        lex(text)
            .into_iter()
            .filter(|t| t.kind == TokenKind::Ident)
            .map(|t| &text[t.start..t.end])
            .collect()
    }

    #[test]
    fn strings_and_comments_hold_no_tokens() {
        let text =
            "behavior a \"a b\" { // a c\n  x [a] \"\"\"a\n\"q\" a\"\"\" y \"esc \\\" a\" z\n}";
        assert_eq!(idents(text), ["behavior", "a", "x", "a", "y", "z"]);
    }

    #[test]
    fn a_word_is_never_split() {
        assert_eq!(
            idents("x_sesion_limit é→a sesion"),
            ["x_sesion_limit", "é→a", "sesion"]
        );
    }

    #[test]
    fn positions_are_one_based_byte_columns() {
        let text = SourceText::new("ab\n\"é\" cd\n".to_string());
        assert_eq!(text.offset(1, 1), Some(0));
        // Line 2 starts at byte 3; "é" is 2 bytes, so column 6 is byte 8.
        assert_eq!(text.offset(2, 6), Some(8));
        assert_eq!(text.position(8), (2, 6));
        assert_eq!(text.offset(1, 4), None, "past the line's end");
        let span = text.span(Sym::new("f"), 8, 10);
        assert!(text.spells(&span, "cd"));
    }
}
