//! The lexemes of a `.spec` text, read without parsing so a half-typed
//! document lexes too: identifiers, scheme ref IDs, numbers, strings,
//! comments and single punctuation characters, as tree-sitter's grammar
//! (`crates/tree-sitter-specforge/grammar.js`) tokenizes them; the test
//! `lexer_agrees_with_the_grammar` checks it over the repository's spec.
//! One divergence: a `"…"` string ends at its line's end (the grammar lets
//! it run on, no spec in the repository writes one), so an unclosed quote
//! never swallows the rest of a document being typed.

/// What a lexeme is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexemeKind {
    /// Letters, digits and `_`, not starting with a digit; any non-ASCII
    /// byte continues it, so a word is never split.
    Ident,
    /// `scheme.kind:id` (`gh.issue:42`, the grammar's `scheme_ref_id`): a
    /// ref's ID, one lexeme.
    RefId,
    /// A word starting with a digit, with an optional `.digits` part:
    /// `42`, `10ms`, `1.5`. A leading `-` is its own `Punct`.
    Number,
    /// `"…"` (with `\` escapes) or `"""…"""`, quotes included.
    Str,
    /// `//` to the end of its line, the newline excluded.
    Comment,
    /// One ASCII punctuation character.
    Punct(char),
}

/// One lexeme, as a byte range of the text it was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lexeme {
    pub start: usize,
    pub end: usize,
    pub kind: LexemeKind,
}

impl Lexeme {
    /// An identifier or a scheme ref ID: what can name an entity.
    pub fn is_name(&self) -> bool {
        matches!(self.kind, LexemeKind::Ident | LexemeKind::RefId)
    }

    /// Its text in `text`, the text it was lexed from.
    pub fn text<'t>(&self, text: &'t str) -> &'t str {
        &text[self.start..self.end]
    }

    /// Whether it is the punctuation character `c`.
    pub fn is_punct(&self, c: char) -> bool {
        self.kind == LexemeKind::Punct(c)
    }
}

/// A byte that continues an identifier: what the grammar's identifiers are
/// made of, and any non-ASCII byte, so a lexeme never splits a word.
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// A byte that starts an identifier.
fn starts_ident(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
}

/// A byte of a scheme ref ID's last part: the grammar's
/// `/[^\s"{}()\[\],]+/`.
fn is_ref_id_byte(byte: u8) -> bool {
    !byte.is_ascii_whitespace()
        && !matches!(byte, b'"' | b'{' | b'}' | b'(' | b')' | b'[' | b']' | b',')
}

/// The end of the grammar's `identifier` (`[a-zA-Z_][a-zA-Z0-9_]*`)
/// starting at `at`, when one starts there.
fn ascii_ident_end(bytes: &[u8], at: usize) -> Option<usize> {
    let first = *bytes.get(at)?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut end = at + 1;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    Some(end)
}

/// The end of a scheme ref ID starting at `at` (`scheme.kind:id`, as the
/// grammar's `scheme_ref_id` token reads it), when one starts there.
fn ref_id_end(bytes: &[u8], at: usize) -> Option<usize> {
    let scheme = ascii_ident_end(bytes, at)?;
    if bytes.get(scheme) != Some(&b'.') {
        return None;
    }
    let kind = ascii_ident_end(bytes, scheme + 1)?;
    if bytes.get(kind) != Some(&b':') {
        return None;
    }
    let mut end = kind + 1;
    while end < bytes.len() && is_ref_id_byte(bytes[end]) {
        end += 1;
    }
    (end > kind + 1).then_some(end)
}

/// The lexemes of `text`, in order, whitespace skipped. Text that starts
/// inside a string is read as code: callers lex from a statement's start.
/// An unclosed `"…"` ends at its line's end; an unclosed `"""…"""` at the
/// text's end.
pub fn lex(text: &str) -> Vec<Lexeme> {
    let bytes = text.as_bytes();
    let find = |from: usize, needle: &[u8]| {
        bytes[from.min(bytes.len())..]
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|at| from + at)
    };
    let mut lexemes = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        let start = i;
        let kind = if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if bytes[i..].starts_with(b"\"\"\"") {
            i = find(i + 3, b"\"\"\"").map_or(bytes.len(), |end| end + 3);
            LexemeKind::Str
        } else if byte == b'"' {
            // `"`, `\` and `\n` are ASCII, so no byte of a wider
            // character is ever taken for one of them.
            i += 1;
            loop {
                match bytes.get(i) {
                    // Unclosed: the string ends at its line's end.
                    None | Some(b'\n') => break,
                    Some(b'"') => {
                        i += 1;
                        break;
                    }
                    Some(b'\\') if bytes.get(i + 1).is_some_and(|b| *b != b'\n') => i += 2,
                    Some(_) => i += 1,
                }
            }
            LexemeKind::Str
        } else if bytes[i..].starts_with(b"//") {
            i = find(i, b"\n").unwrap_or(bytes.len());
            LexemeKind::Comment
        } else if let Some(end) = ref_id_end(bytes, i) {
            i = end;
            LexemeKind::RefId
        } else if starts_ident(byte) {
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            LexemeKind::Ident
        } else if byte.is_ascii_digit() {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'.') && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            LexemeKind::Number
        } else {
            // Every non-ASCII byte is a word byte: punctuation is ASCII.
            i += 1;
            LexemeKind::Punct(char::from(byte))
        };
        lexemes.push(Lexeme {
            start,
            end: i,
            kind,
        });
    }
    lexemes
}
