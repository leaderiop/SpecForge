//! The lexemes of a `.spec` text, read without parsing so a half-typed
//! document lexes too: identifiers, scheme ref IDs, numbers, strings,
//! comments and single punctuation characters, as tree-sitter's grammar
//! (`crates/kernel/tree-sitter/grammar.js`) tokenizes them; the test
//! `lexer_agrees_with_the_grammar` checks it over the repository's spec and
//! a fixture of the forms the spec does not write. A string is read the
//! grammar's way, across lines. One the grammar would not close is marked
//! (`Str { closed: false }`); a `"…"` one ends at its first line's end and a
//! `"""…"""` one before the next line that starts a top-level form, so an
//! unclosed quote never swallows the rest of a document being typed.
//! The parser's recovery from unclosed strings, the LSP's document, navigation
//! and the formatter read text through this module and scan none themselves
//! (ADR 0023, ADR 0038). Where a top-level form starts is this module's too
//! (`starts_top_level_form`): recovery and the LSP's document both ask it. Which
//! statement a line of a block's body starts is this module's too
//! (`body_statement_at`): the LSP's document asks it.
//!
//! This is the second reader of the language's text beside the grammar, and
//! `expr::tokenize` is a third that is not built on it. `tokenize` reads the
//! expression sub-language (`latency < 100ms`, an `expr { }` group, or a prose
//! line the prove pass reads), whose lexical rules are not the host's:
//! identifiers are `[a-z_][a-z0-9_]*` (here any ASCII letter and any non-ASCII
//! byte), a number's unit is alphabetic only and a trailing `.` belongs to the
//! number (`1.`, `1.ms`; here `.digits` only, then any word byte), `<=`, `>=`,
//! `==` and `!=` are one token (here two puncts), columns count characters
//! (here bytes), and any character that is no token is an error with a
//! position (here there are no errors). A tokenizer over these lexemes would
//! re-read every `Ident`, `Number`, `RefId`, string and comment lexeme
//! character by character to split and reject it, which is the old tokenizer
//! behind a layer, and would change the error position of an input such as
//! `Foo`, `abcÉ`, `10ms2` or `a.b:c`. So the two stay apart, and a test
//! (`expr::tests::the_tokenizer_agrees_with_the_lexer_and_the_grammar_on_the_corpus`)
//! pins what they share: on every expression of the repository's spec, the
//! tokens `tokenize` cuts are the lexemes `lex` cuts (the four two-character
//! operators joined), and what `parse_expression` reads from an `expr { }`
//! group's text is what the grammar read.

mod slots;

/// What a lexeme is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexemeKind {
    /// Letters, digits and `_`, not starting with a digit; any non-ASCII
    /// byte continues it, so a word is never split. Where the grammar reads
    /// a bare word (a value, a list item, an entity's name or a `use`
    /// binding, ADR 0283) it is the whole word, `AUTH-01`, `03`, `v1.0`,
    /// unless the word has the shape of a number or a date.
    Ident,
    /// `scheme.kind:id` (`gh.issue:42`, the grammar's `scheme_ref_id`): a
    /// ref's ID, one lexeme.
    RefId,
    /// A word starting with a digit, with an optional `.digits` part:
    /// `42`, `10ms`, `1.5`; a leading `-` is its own `Punct`. In a value or list item
    /// slot (ADR 0283) it is a bare word with the shape of an integer or a date
    /// (`42`, `-5`, `007`, `2026-10-26`, [`is_number_word`]), whole, sign included.
    Number,
    /// `"…"` (with `\` escapes) or `"""…"""`, quotes included, read as the
    /// grammar's `string` and `triple_quoted_string` tokens read it: across
    /// lines. `closed` is false for one the grammar would not close. Such a
    /// `"…"` (no closing quote; a `\` before a line break, which is no
    /// escape; or, spanning lines, a closing quote that runs straight into
    /// text, the pairing an unclosed quote shifts) ends at its first line's
    /// end. Such a `"""…"""` (no closing `"""`, or, spanning lines, closing
    /// quotes that run straight into text) ends at the end of the line
    /// before the next line that starts a top-level form, where the parser's
    /// recovery resumes, else at the text's end.
    Str { closed: bool },
    /// `//` to the end of its line, the newline excluded.
    Comment,
    /// One ASCII punctuation character.
    Punct(char),
}

impl LexemeKind {
    /// A string, closed or not.
    pub fn is_str(self) -> bool {
        matches!(self, LexemeKind::Str { .. })
    }
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

/// `"""`: what opens and closes a triple-quoted string.
pub(crate) const TRIPLE: &str = "\"\"\"";

/// The indentation of the line holding byte `at` of `text`, in bytes, when
/// nothing but whitespace precedes `at` on that line: what a top-level
/// form's line is measured by. `None` when a lexeme (or a string's tail)
/// precedes it on its line.
pub fn line_indent(text: &str, at: usize) -> Option<usize> {
    let line_start = text[..at].rfind('\n').map_or(0, |nl| nl + 1);
    text.as_bytes()[line_start..at]
        .iter()
        .all(u8::is_ascii_whitespace)
        .then_some(at - line_start)
}

/// Whether the first of `lexemes` (lexemes of `text`, in order; any after
/// its line are ignored) starts a line that opens a top-level form of the
/// grammar, a child of `source_file`: it is the first lexeme of its line,
/// that line is indented by at most `max_indent` bytes (`line_indent`), and
/// the line's lexemes read `use` then a path, `{` or `*`; `pub use` and the
/// same; `spec "Title" {`; `ref scheme.kind:id "Title"`; `kind name
/// ["Title"] {` (a define block too); or `kind name =` (a union). A title
/// is a `"…"` closed on that line.
///
/// The one rule of where a top-level form starts (ADR 0038 D6). The lexer
/// ends an unclosed `"""` before such a line, passing each line it scans
/// lexed alone; `form_starts` asks it over a whole document, for the parser's
/// recovery and the LSP's document. A test holds it to the grammar's
/// top-level nodes in the repository's spec, both ways.
pub fn starts_top_level_form(text: &str, lexemes: &[Lexeme], max_indent: usize) -> bool {
    let Some(first) = lexemes.first() else {
        return false;
    };
    if line_indent(text, first.start).is_none_or(|indent| indent > max_indent) {
        return false;
    }
    let line_end = text[first.start..]
        .find('\n')
        .map_or(text.len(), |nl| first.start + nl);
    let line = &lexemes[..lexemes.partition_point(|l| l.start < line_end)];
    let kind = |i: usize| line.get(i).map(|l| l.kind);
    let word = |i: usize| {
        line.get(i)
            .filter(|l| l.kind == LexemeKind::Ident)
            .map(|l| l.text(text))
    };
    // A header's title: a `"…"` closed on this line.
    let title = |i: usize| {
        line.get(i).is_some_and(|l| {
            l.kind == LexemeKind::Str { closed: true }
                && l.end <= line_end
                && !l.text(text).starts_with(TRIPLE)
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
        Some(_) => {
            let Some(name) = name_len(text, line, 1) else {
                return false;
            };
            let open = if title(1 + name) { 2 + name } else { 1 + name };
            punct(open, '{') || punct(open, '=')
        }
        None => false,
    }
}

/// The lexemes of `text` (indices into `lexemes`, its `lex`, in order) that
/// start a top-level form as a document reads it: each first on its line,
/// whose line reads one of the grammar's top-level shapes
/// (`starts_top_level_form`) indented no deeper than the last line begun
/// with no bracket open (`{`, `[` or `(`). Such a line closes every bracket
/// open before it: a body, block, list or parameter list left unclosed ends
/// there. Comments neither start a form nor measure a line.
///
/// The one reading of where a document's forms start (ADR 0063 D3). The
/// parser's recovery cuts the source at each one the grammar's tree did not
/// start a form at (`cst`), and the LSP's document closes its frames at each.
pub fn form_starts(text: &str, lexemes: &[Lexeme]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut depth = 0usize;
    let mut top_indent = 0usize;
    let mut previous_end: Option<usize> = None;
    for (i, lexeme) in lexemes.iter().enumerate() {
        if lexeme.kind == LexemeKind::Comment {
            continue;
        }
        let first_on_line = previous_end.is_none_or(|end| text[end..lexeme.start].contains('\n'));
        previous_end = Some(lexeme.end);
        if first_on_line {
            let indent = line_indent(text, lexeme.start);
            if depth == 0
                && let Some(indent) = indent
            {
                top_indent = indent;
            }
            if starts_top_level_form(text, &lexemes[i..], top_indent) {
                starts.push(i);
                depth = 0;
                top_indent = indent.unwrap_or(top_indent);
            }
        }
        match lexeme.kind {
            LexemeKind::Punct('{' | '[' | '(') => depth += 1,
            LexemeKind::Punct('}' | ']' | ')') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    starts
}

/// Which statement a line of a block's body starts (ADR 0241): what
/// `crates/kernel/tree-sitter/grammar.js`'s `field`, `verify_statement` and
/// `method_statement` read, from lexemes, on a half-typed line too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyStatement {
    /// `verify [kind] "description"`: a `"..."` string (not `"""..."""`, closed or
    /// not) follows `verify` directly, or after one identifier.
    Verify,
    /// `verify`, or `verify` and one identifier, with nothing after it on its
    /// line and no `"..."` string next: a verify line being typed. The grammar
    /// reads a field named verify (`verify string`).
    VerifyTyped,
    /// `method name(...)`: an identifier follows `method` on its line, then `(`
    /// or nothing yet.
    Method,
    /// `verdict <rule> <outcome> {` on one line (ADR 0290): the rule a word or names joined by adjacent `/`, the
    /// outcome an identifier. With less after it, `verdict` is a field's key (`verdict ValidatorVerdictKind`).
    Verdict,
    /// A field's key, or a line the grammar reads as no statement.
    Field,
}

/// The statement a body line whose first lexeme is `lexemes[i]` (of `text`)
/// starts. The one reading of it: the LSP's document asks it, and a test holds
/// it to the grammar's `verify_statement` and `method_statement` nodes. Only
/// body statements are asked: in a nested block `verify` and `method` are
/// always keys, which the caller knows from its frames.
///
/// A statement's kind is read on its line, where the grammar reads across line
/// breaks: a bare `verify` above `contract "c"` is a verify line being typed
/// here, while the grammar takes the next line as its kind and description.
pub fn body_statement_at(text: &str, lexemes: &[Lexeme], i: usize) -> BodyStatement {
    let Some(first) = lexemes.get(i) else {
        return BodyStatement::Field;
    };
    let line_end = text[first.end..]
        .find('\n')
        .map_or(text.len(), |nl| first.end + nl);
    let mut next = lexemes[i + 1..]
        .iter()
        .filter(|l| l.kind != LexemeKind::Comment);
    let (a, b) = (next.next(), next.next());
    let (a_on, b_on) = (
        a.filter(|l| l.start < line_end),
        b.filter(|l| l.start < line_end),
    );
    let ident = |l: Option<&Lexeme>| l.is_some_and(|l| l.kind == LexemeKind::Ident);
    // The grammar's `string`: a triple-quoted one is a field's value, never a description.
    let regular = |l: Option<&Lexeme>| {
        l.is_some_and(|l| l.kind.is_str() && !l.text(text).starts_with(TRIPLE))
    };
    match first.text(text) {
        "verify" if regular(a) || (ident(a_on) && regular(b)) => BodyStatement::Verify,
        "verify" if a_on.is_none() || (ident(a_on) && b_on.is_none()) => BodyStatement::VerifyTyped,
        "verdict" if verdict_head(text, lexemes, i, line_end) => BodyStatement::Verdict,
        "method" if ident(a_on) && (b_on.is_none() || b.is_some_and(|b| b.is_punct('('))) => {
            BodyStatement::Method
        }
        _ => BodyStatement::Field,
    }
}

/// Whether the `verdict` at `lexemes[i]` is followed, before `line_end`, by a rule, an outcome and `{`: the rule one
/// word, or names joined by adjacent `/` (the grammar's `reference_path`), the outcome one identifier.
fn verdict_head(text: &str, lexemes: &[Lexeme], i: usize, line_end: usize) -> bool {
    let mut at = i + 1;
    let code = |at: usize| {
        lexemes
            .get(at)
            .filter(|l| l.start < line_end && l.kind != LexemeKind::Comment)
    };
    let Some((len, _)) = word_run(text, lexemes, at, false) else {
        return false;
    };
    at += len;
    // `/name` joined with no gap, as many as there are.
    while let (Some(slash), Some(next)) = (lexemes.get(at), lexemes.get(at + 1))
        && slash.kind == LexemeKind::Punct('/')
        && lexemes[at - 1].end == slash.start
        && next.start == slash.end
    {
        let Some((len, _)) = word_run(text, lexemes, at + 1, false) else {
            return false;
        };
        at += 1 + len;
    }
    let outcome = code(at).is_some_and(|l| l.kind == LexemeKind::Ident);
    outcome && code(at + 1).is_some_and(|l| l.kind == LexemeKind::Punct('{'))
}

/// The start of the first line after the one holding byte `from` of `text`
/// that starts a top-level form indented by no more than `max_indent` bytes
/// (`starts_top_level_form`, each line lexed alone: the text after a string
/// the grammar would not close cannot be lexed whole). Where such a string
/// ends: the lexer ends an unclosed `"""…"""` at the end of the line before
/// it (ADR 0038).
fn next_form_line(text: &str, from: usize, max_indent: usize) -> Option<usize> {
    let mut at = from;
    while let Some(nl) = text[at..].find('\n') {
        let line = at + nl + 1;
        if line >= text.len() {
            return None;
        }
        let line_end = text[line..].find('\n').map_or(text.len(), |nl| line + nl);
        let line_text = &text[line..line_end];
        if starts_top_level_form(line_text, &lex(line_text), max_indent) {
            return Some(line);
        }
        at = line;
    }
    None
}

/// The end of the bare word starting at `at`, the grammar's `_bare` (`-?[A-Za-z0-9_]+([.-][A-Za-z0-9_]+)*`,
/// ADR 0283), or, with `minus` false, its `_name` (no leading `-`). `None` when none starts there. ASCII only:
/// a non-ASCII byte ends it, and the caller sees the lexeme it cuts.
pub(crate) fn word_end(bytes: &[u8], at: usize, minus: bool) -> Option<usize> {
    let word = |b: Option<&u8>| b.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_');
    let mut end = at;
    if minus && bytes.get(end) == Some(&b'-') {
        end += 1;
    }
    if !word(bytes.get(end)) {
        return None;
    }
    loop {
        while word(bytes.get(end)) {
            end += 1;
        }
        if matches!(bytes.get(end), Some(b'.' | b'-')) && word(bytes.get(end + 1)) {
            end += 1;
        } else {
            return Some(end);
        }
    }
}

/// Whether `text` is one name as the grammar reads one where an entity is named (`_name`, ADR 0283): word
/// characters in runs joined by a single `-` or `.`, never starting with `-` (`AUTH-01`, `03`, `v1.0`).
pub fn is_name(text: &str) -> bool {
    word_end(text.as_bytes(), 0, false) == Some(text.len())
}

/// Whether `text` is a path as the grammar reads one (`reference_path`, ADR 0284): names joined by `/`, with a
/// leading `/` rooting it at the project (`03/D-01`, `/D-09`). One name is a path of one segment.
pub fn is_path(text: &str) -> bool {
    text.strip_prefix('/')
        .unwrap_or(text)
        .split('/')
        .all(is_name)
}

/// Whether `text` is one bare value word as the grammar reads one (`_bare`, ADR 0283): a name, with one leading
/// `-` allowed (`-5`).
pub fn is_bare_word(text: &str) -> bool {
    word_end(text.as_bytes(), 0, true) == Some(text.len())
}

/// What a bare word is by its shape alone: the parser's tag, which loses nothing (ADR 0283). The field's declared
/// type, which the parser never reads, decides what a word means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralShape {
    /// A canonical integer, `-?(0|[1-9][0-9]*)` within `i64`, not `-0`.
    Integer(i64),
    /// `true` or `false`.
    Boolean(bool),
    /// `YYYY-MM-DD` as written; whether the day exists is a `date` field's to say.
    Date,
    /// Any other word: `007`, `2-1`, `v1.0`, `AUTH-01`, `1.5`, an integer too large for `i64`.
    Other,
}

/// The [`LiteralShape`] of `word`, a bare word. The one function the parser's value tag, its list items' form,
/// the lexer and their agreement test ask.
pub fn literal_shape(word: &str) -> LiteralShape {
    let bytes = word.as_bytes();
    let digits = |b: &[u8]| !b.is_empty() && b.iter().all(u8::is_ascii_digit);
    match word {
        "true" => return LiteralShape::Boolean(true),
        "false" => return LiteralShape::Boolean(false),
        _ => {}
    }
    if bytes.len() == 10
        && digits(&bytes[..4])
        && bytes[4] == b'-'
        && digits(&bytes[5..7])
        && bytes[7] == b'-'
        && digits(&bytes[8..])
    {
        return LiteralShape::Date;
    }
    let magnitude = word.strip_prefix('-').unwrap_or(word);
    let canonical =
        digits(magnitude.as_bytes()) && (magnitude == "0" || !magnitude.starts_with('0'));
    if canonical
        && word != "-0"
        && let Ok(n) = word.parse::<i64>()
    {
        return LiteralShape::Integer(n);
    }
    LiteralShape::Other
}

/// Whether `word` has the shape of a number or a date (`-?[0-9]+`, or `YYYY-MM-DD`): what the lexer reads as a
/// `Number` where a bare word is expected, and what a list item writes as a literal.
pub fn is_number_word(word: &str) -> bool {
    let magnitude = word.strip_prefix('-').unwrap_or(word);
    (!magnitude.is_empty() && magnitude.bytes().all(|b| b.is_ascii_digit()))
        || literal_shape(word) == LiteralShape::Date
}

/// The number of lexemes from `lexemes[at]` that spell one entity name: a run of adjacent lexemes whose bytes are
/// a `_name` word, or a single identifier (which may hold non-ASCII bytes). `None` when none starts there.
pub(crate) fn name_len(text: &str, lexemes: &[Lexeme], at: usize) -> Option<usize> {
    let first = lexemes.get(at)?;
    if let Some((len, _)) = word_run(text, lexemes, at, false) {
        return Some(len);
    }
    (first.kind == LexemeKind::Ident).then_some(1)
}

/// The lexemes from `lexemes[at]` that are, byte for byte and with no gap, one bare word
/// ([`word_end`]): their count and the word's end. `None` when no word starts there, or when it ends inside a
/// lexeme (a non-ASCII byte or a scheme ref ID's `:` continues it).
pub(crate) fn word_run(
    text: &str,
    lexemes: &[Lexeme],
    at: usize,
    minus: bool,
) -> Option<(usize, usize)> {
    let first = lexemes.get(at)?;
    let end = word_end(text.as_bytes(), first.start, minus)?;
    let mut position = first.start;
    let mut len = 0;
    for lexeme in &lexemes[at..] {
        if lexeme.start >= end {
            break;
        }
        let part = matches!(
            lexeme.kind,
            LexemeKind::Ident | LexemeKind::Number | LexemeKind::Punct('-' | '.')
        );
        if !part || lexeme.start != position {
            return None;
        }
        position = lexeme.end;
        len += 1;
    }
    (position == end).then_some((len, end))
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

/// Whether the byte at `at` of `text` continues a token: what the closing
/// quote of a string spanning lines is never followed by in the repository's
/// spec. Whitespace, a bracket, `,`, `|`, `/` (a comment) and the end of the
/// text do not continue one.
pub fn runs_into_text(text: &str, at: usize) -> bool {
    follows_text(text.as_bytes(), at)
}

/// `runs_into_text` over bytes.
fn follows_text(bytes: &[u8], at: usize) -> bool {
    bytes.get(at).is_some_and(|&c| {
        !(c.is_ascii_whitespace()
            || matches!(
                c,
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b',' | b'|' | b'/'
            ))
    })
}

/// Whether `text` is a scheme ref ID cut short: `scheme`, `scheme.`,
/// `scheme.kind`, `scheme.kind:` or a whole `scheme.kind:id` (the grammar's
/// `scheme_ref_id`, typed so far).
pub fn is_ref_id_prefix(text: &str) -> bool {
    let bytes = text.as_bytes();
    let Some(scheme) = ascii_ident_end(bytes, 0) else {
        return false;
    };
    match bytes.get(scheme) {
        None => return true,
        Some(b'.') => {}
        Some(_) => return false,
    }
    if scheme + 1 == bytes.len() {
        return true;
    }
    let Some(kind) = ascii_ident_end(bytes, scheme + 1) else {
        return false;
    };
    match bytes.get(kind) {
        None => true,
        Some(b':') => bytes[kind + 1..].iter().all(|&b| is_ref_id_byte(b)),
        Some(_) => false,
    }
}

/// Whether `text` is the grammar's `identifier` (`[a-zA-Z_][a-zA-Z0-9_]*`), whole.
pub fn is_identifier(text: &str) -> bool {
    ascii_ident_end(text.as_bytes(), 0) == Some(text.len())
}

/// Whether `text` is one entity ID as the grammar reads one, whole: a name ([`is_name`]: an `identifier`, or
/// a word holding `-` or `.`, `AUTH-01`, `03`) or a `scheme_ref_id` (`scheme.kind:id`, `gh.issue:42`).
pub fn is_entity_id(text: &str) -> bool {
    is_path(text) || ref_id_end(text.as_bytes(), 0) == Some(text.len())
}

/// The end of the `"…"` string opening at `open`, and whether the grammar
/// closes it: at its closing quote when the grammar's token ends there and,
/// if it spans lines, nothing runs straight into that quote; else at its
/// first line's end. `"`, `\` and `\n` are ASCII, so no byte of a wider
/// character is ever taken for one of them.
fn regular_string(bytes: &[u8], open: usize) -> (usize, bool) {
    let line_end = bytes[open..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |at| open + at);
    let mut i = open + 1;
    let close = loop {
        match bytes.get(i) {
            None => break None,
            Some(b'"') => break Some(i),
            // The grammar's escape is `\` and any character but a CR or a LF.
            Some(b'\\') => match bytes.get(i + 1) {
                None | Some(b'\n' | b'\r') => break None,
                Some(_) => i += 2,
            },
            Some(_) => i += 1,
        }
    };
    match close {
        Some(close) if close < line_end || !follows_text(bytes, close + 1) => (close + 1, true),
        _ => (line_end, false),
    }
}

/// The end of the `"""…"""` string opening at `open`, and whether the
/// grammar closes it: after the first `"""` that follows, unless the string
/// spans lines and that `"""` runs straight into text (the pairing an
/// unclosed `"""` shifts). One the grammar would not close ends at the end
/// of the line before the next line that starts a top-level form
/// (`next_form_line`, indented no deeper than `top_indent`, the top-level
/// line it sits under), else at the text's end.
fn triple_string(text: &str, open: usize, top_indent: usize) -> (usize, bool) {
    let body = open + TRIPLE.len();
    let close = text[body..].find(TRIPLE).map(|at| body + at + TRIPLE.len());
    match close {
        Some(end) if !text[open..end].contains('\n') || !follows_text(text.as_bytes(), end) => {
            (end, true)
        }
        _ => (
            next_form_line(text, body, top_indent).map_or(text.len(), |line| line - 1),
            false,
        ),
    }
}

/// The deepest the grammar's brackets may nest: `(`, `[` and `{` counted together, outside strings and
/// comments. The repository's 235 `.spec` files nest at most 3; the parser, the grammar and every walk of
/// their trees recurse once per level, and an editor's thread has a 2 MiB stack.
pub const MAX_NESTING: usize = 32;

/// The byte offset of the first opening bracket of `text` that nests deeper than [`MAX_NESTING`], read
/// through [`lex`]; `None` when every bracket is within it. A closer never takes the depth below zero.
pub fn nested_too_deep(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for lexeme in lex(text) {
        match lexeme.kind {
            LexemeKind::Punct('(' | '[' | '{') => {
                depth = depth.saturating_add(1);
                if depth > MAX_NESTING {
                    return Some(lexeme.start);
                }
            }
            LexemeKind::Punct(')' | ']' | '}') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

/// The lexemes of `text`, in order, whitespace skipped. Text that starts
/// inside a string is read as code: callers lex from a statement's start.
/// Where the grammar reads a bare word (a field's value, a list item, an entity's name, a selective
/// `use`'s binding), the word is one lexeme (ADR 0283); elsewhere `a-b` is `a`, `-`, `b`.
/// A string the grammar would not close is marked `Str { closed: false }`:
/// a `"…"` ends at its first line's end, a `"""…"""` at the end of the line
/// before the next line that starts a top-level form, else at the text's end.
pub fn lex(text: &str) -> Vec<Lexeme> {
    slots::read_words(text, lex_base(text))
}

/// The lexemes as the lexer cuts them without knowing where it is: an identifier, a number or a punctuation
/// character at a time. [`slots::read_words`] joins the ones a word slot spans.
fn lex_base(text: &str) -> Vec<Lexeme> {
    let bytes = text.as_bytes();
    let find = |from: usize, needle: &[u8]| {
        bytes[from.min(bytes.len())..]
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|at| from + at)
    };
    let mut lexemes = Vec::new();
    // The brace depth, and the indentation of the last line begun at depth
    // 0: a top-level form after an unclosed `"""` is indented no deeper.
    let mut depth = 0usize;
    let mut top_indent = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        let start = i;
        if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if depth == 0
            && !bytes[i..].starts_with(b"//")
            && let Some(indent) = line_indent(text, i)
        {
            top_indent = indent;
        }
        let kind = if bytes[i..].starts_with(b"\"\"\"") {
            let (end, closed) = triple_string(text, i, top_indent);
            i = end;
            LexemeKind::Str { closed }
        } else if byte == b'"' {
            let (end, closed) = regular_string(bytes, i);
            i = end;
            LexemeKind::Str { closed }
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
            match byte {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
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
