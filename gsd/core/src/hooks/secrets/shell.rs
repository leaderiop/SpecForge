//! Ported from gsd-core v1.19.0 hooks/gsd-secret-read-guard.js (`tokenize` and its heredoc and substitution readers), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! A shell command line as tokens: words, separators, redirection operators and heredoc bodies, each with the index
//! of the simple command (`seg`) it belongs to, and the command substitutions found inside it.

use std::cell::Cell;

/// Which reading of a heredoc the scan makes (gsd-core v1.19.0, #5513). A command that opens a heredoc is scanned
/// under every reading and denied if any finds a read, so a change of reading can add a denial, never remove one.
#[derive(Clone, Copy)]
pub(super) struct Reading {
    /// bash 5 skips the bodies of heredocs opened inside `$( )` when it looks for the closing `)`; bash 3.2 (macOS
    /// `/bin/bash`) does not, so a `)` on a body line closes the substitution.
    pub bodies_skipped_in_spans: bool,
    /// An unquoted tag is a bash word (a CR and a substitution belong to it); false is the earlier reading, where
    /// the tag stopped at any whitespace and at `(`.
    pub tags_as_words: bool,
}

impl Reading {
    pub const DEFAULT: Self = Self {
        bodies_skipped_in_spans: true,
        tags_as_words: true,
    };
}

// shortcut: one thread-local reading, as upstream's module flags; thread it as a parameter if the scan ever runs on
// more than one thread.
thread_local!(static READING: Cell<Reading> = const { Cell::new(Reading::DEFAULT) });

pub(super) fn reading() -> Reading {
    READING.with(Cell::get)
}

/// Runs `f` under reading `r`, and restores the reading before it even if `f` panics.
pub(super) fn with_reading<T>(r: Reading, f: impl FnOnce() -> T) -> T {
    struct Restore(Reading);
    impl Drop for Restore {
        fn drop(&mut self) {
            READING.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(READING.with(|c| c.replace(r)));
    f()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Word,
    Sep,
    Op,
    Heredoc,
}

#[derive(Debug, Clone)]
pub(super) struct Token {
    pub kind: Kind,
    pub text: String,
    /// A heredoc's tag was quoted: its body is not expanded.
    pub quoted: bool,
    pub seg: usize,
    /// An operator that reads its target (`<`, `0<`), as opposed to writing it.
    pub read: bool,
    /// A descriptor duplication (`2>&1`), which names no file.
    pub dup: bool,
    /// A process substitution's inner command (`<(…)`).
    pub procsub: Option<String>,
}

fn token(kind: Kind, text: String, seg: usize) -> Token {
    Token {
        kind,
        text,
        quoted: false,
        seg,
        read: false,
        dup: false,
        procsub: None,
    }
}

fn find(s: &[char], c: char, from: usize) -> Option<usize> {
    (from..s.len()).find(|&i| s[i] == c)
}

fn sub(s: &[char], a: usize, b: usize) -> String {
    s[a.min(s.len())..b.clamp(a.min(s.len()), s.len())]
        .iter()
        .collect()
}

#[derive(Clone)]
struct Tag {
    tag: String,
    quoted: bool,
    strip_tabs: bool,
    seg: usize,
    end: usize,
}

fn read_heredoc_tag(s: &[char], mut i: usize) -> Tag {
    let strip_tabs = s.get(i) == Some(&'-');
    if strip_tabs {
        i += 1;
    }
    while matches!(s.get(i), Some(' ' | '\t')) {
        i += 1;
    }
    let (mut tag, mut quoted) = (String::new(), false);
    if let Some(&q @ ('\'' | '"')) = s.get(i) {
        let j = find(s, q, i + 1);
        tag = sub(s, i + 1, j.unwrap_or(s.len()));
        quoted = true;
        i = j.map_or(s.len(), |j| j + 1);
    } else {
        if s.get(i) == Some(&'\\') {
            quoted = true;
            i += 1;
        }
        let words = reading().tags_as_words;
        while let Some(&c) = s.get(i) {
            if if words {
                " \t\n;&|<>()".contains(c)
            } else {
                c.is_whitespace() || ";&|<>()".contains(c)
            } {
                break;
            }
            // A substitution inside the word belongs to the tag, taken literally (`E$(x)F` waits for `E$(x)F`).
            let end = match (words, c, s.get(i + 1)) {
                (true, '$', Some('(')) => Some(paren_close(s, i + 2)),
                (true, '$', Some('{')) => Some(find(s, '}', i).unwrap_or(s.len() - 1)),
                (true, '`', _) => Some(find(s, '`', i + 1).unwrap_or(s.len() - 1)),
                _ => None,
            };
            if let Some(e) = end {
                tag += &sub(s, i, e + 1);
                i = e + 1;
            } else {
                tag.push(c);
                i += 1;
            }
        }
    }
    Tag {
        tag,
        quoted,
        strip_tabs,
        seg: 0,
        end: i,
    }
}

/// The bodies of the heredocs `tags` opened, read line by line from `i`; the index after the last one read.
fn heredoc_bodies(s: &[char], mut i: usize, tags: &[Tag]) -> (Vec<(String, bool, usize)>, usize) {
    let mut bodies = Vec::new();
    for t in tags {
        let (mut body, mut terminated) = (String::new(), false);
        while i < s.len() {
            let nl = find(s, '\n', i);
            let line = sub(s, i, nl.unwrap_or(s.len()));
            i = nl.map_or(s.len(), |n| n + 1);
            let probe = if t.strip_tabs {
                line.trim_start_matches('\t')
            } else {
                &line
            };
            if probe == t.tag {
                terminated = true;
                break;
            }
            body += &line;
            body.push('\n');
        }
        bodies.push((body, t.quoted, t.seg));
        if !terminated {
            break;
        }
    }
    (bodies, i)
}

/// The index of the `)` that closes a `$(` whose body starts at `i`, or the length.
fn paren_close(s: &[char], mut i: usize) -> usize {
    let mut depth = 1;
    let mut tags: Vec<Tag> = Vec::new();
    while i < s.len() {
        let ch = s[i];
        if ch == '\\' {
            i += 2;
        } else if ch == '\'' {
            i = find(s, '\'', i + 1).map_or(s.len(), |j| j + 1);
        } else if ch == '"' {
            i += 1;
            while i < s.len() && s[i] != '"' {
                if s[i] == '\\' {
                    i += 2;
                } else if s[i] == '$' && s.get(i + 1) == Some(&'(') {
                    i = paren_close(s, i + 2) + 1;
                } else if s[i] == '`' {
                    i = find(s, '`', i + 1).map_or(s.len(), |j| j + 1);
                } else {
                    i += 1;
                }
            }
            i += 1;
        } else if ch == '`' {
            i = find(s, '`', i + 1).map_or(s.len(), |j| j + 1);
        } else if ch == '<'
            && reading().bodies_skipped_in_spans
            && s.get(i + 1) == Some(&'<')
            && s.get(i + 2) != Some(&'<')
        {
            let tag = read_heredoc_tag(s, i + 2);
            i = tag.end;
            tags.push(tag);
        } else if ch == '\n' && !tags.is_empty() {
            i = heredoc_bodies(s, i + 1, &tags).1;
            tags.clear();
        } else {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            i += 1;
        }
    }
    s.len()
}

/// The `$(…)` and backtick bodies of an unquoted heredoc body.
fn substitutions(body: &str, nested: &mut Vec<String>) {
    let s: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < s.len() {
        if s[i] == '$' && s.get(i + 1) == Some(&'(') {
            let e = paren_close(&s, i + 2);
            nested.push(sub(&s, i + 2, e));
            i = e + 1;
        } else if s[i] == '`' {
            let e = find(&s, '`', i + 1).unwrap_or(s.len());
            nested.push(sub(&s, i + 1, e));
            i = e + 1;
        } else {
            i += 1;
        }
    }
}

struct Lexer {
    tokens: Vec<Token>,
    nested: Vec<String>,
    buf: String,
    quoted: bool,
    has_word: bool,
    seg: usize,
    heredocs: Vec<Tag>,
    expect_tag: Option<bool>,
}

impl Lexer {
    fn flush(&mut self) {
        if !self.has_word {
            return;
        }
        if let Some(strip_tabs) = self.expect_tag.take() {
            self.heredocs.push(Tag {
                tag: std::mem::take(&mut self.buf),
                quoted: self.quoted,
                strip_tabs,
                seg: self.seg,
                end: 0,
            });
        } else {
            let mut t = token(Kind::Word, std::mem::take(&mut self.buf), self.seg);
            t.quoted = self.quoted;
            self.tokens.push(t);
        }
        self.buf.clear();
        self.quoted = false;
        self.has_word = false;
    }

    /// A substitution `s[a..b]` in a heredoc tag is part of the tag, taken literally; it is sliced only while a tag is
    /// expected, as upstream does.
    fn literal_in_tag(&mut self, s: &[char], a: usize, b: usize) {
        if self.expect_tag.is_some() && reading().tags_as_words {
            self.buf += &sub(s, a, b);
        }
    }

    fn sep(&mut self, text: &str) {
        self.flush();
        self.tokens.push(token(Kind::Sep, text.into(), self.seg));
        self.seg += 1;
    }
}

/// Tokenizes `src`: the tokens in order, and the bodies of the command substitutions to scan on their own.
pub(super) fn tokenize(src: &str) -> (Vec<Token>, Vec<String>) {
    let s: Vec<char> = src.chars().collect();
    let mut lx = Lexer {
        tokens: Vec::new(),
        nested: Vec::new(),
        buf: String::new(),
        quoted: false,
        has_word: false,
        seg: 0,
        heredocs: Vec::new(),
        expect_tag: None,
    };
    let at = |i: usize| s.get(i).copied();
    let starts = |i: usize, p: &str| p.chars().enumerate().all(|(k, c)| at(i + k) == Some(c));
    let mut i = 0;
    while i < s.len() {
        let ch = s[i];
        match ch {
            '\'' => {
                lx.has_word = true;
                lx.quoted = true;
                let end = find(&s, '\'', i + 1).unwrap_or(s.len());
                lx.buf += &sub(&s, i + 1, end);
                i = end + 1;
            }
            '"' => {
                lx.has_word = true;
                lx.quoted = true;
                i += 1;
                while i < s.len() && s[i] != '"' {
                    let c = s[i];
                    if c == '\\' && at(i + 1).is_some_and(|n| "\"\\$`\n".contains(n)) {
                        if s[i + 1] != '\n' {
                            lx.buf.push(s[i + 1]);
                        }
                        i += 2;
                    } else if c == '$' && at(i + 1) == Some('(') {
                        let e = paren_close(&s, i + 2);
                        lx.nested.push(sub(&s, i + 2, e));
                        i = e + 1;
                    } else if c == '`' {
                        let e = find(&s, '`', i + 1).unwrap_or(s.len());
                        lx.nested.push(sub(&s, i + 1, e));
                        i = e + 1;
                    } else {
                        lx.buf.push(c);
                        i += 1;
                    }
                }
                i += 1;
            }
            '\\' => {
                if at(i + 1) == Some('\n') {
                    i += 2;
                    continue;
                }
                lx.has_word = true;
                if let Some(n) = at(i + 1) {
                    lx.buf.push(n);
                }
                i += 2;
            }
            '$' if at(i + 1) == Some('(') => {
                lx.has_word = true;
                let e = paren_close(&s, i + 2);
                lx.nested.push(sub(&s, i + 2, e));
                lx.literal_in_tag(&s, i, e + 1);
                i = e + 1;
            }
            '$' if at(i + 1) == Some('{') => {
                lx.has_word = true;
                let e = find(&s, '}', i).unwrap_or(s.len().saturating_sub(1));
                lx.buf += &sub(&s, i, e + 1);
                i = e + 1;
            }
            '`' => {
                lx.has_word = true;
                let e = find(&s, '`', i + 1).unwrap_or(s.len());
                lx.nested.push(sub(&s, i + 1, e));
                lx.literal_in_tag(&s, i, e + 1);
                i = e + 1;
            }
            '<' | '>' if at(i + 1) == Some('(') => {
                lx.flush();
                let e = paren_close(&s, i + 2);
                let inner = sub(&s, i + 2, e);
                lx.nested.push(inner.clone());
                let mut t = token(Kind::Word, sub(&s, i, e + 1), lx.seg);
                t.procsub = Some(inner);
                lx.tokens.push(t);
                i = e + 1;
            }
            '\n' => {
                lx.sep("\n");
                i += 1;
                if !lx.heredocs.is_empty() {
                    let (bodies, end) = heredoc_bodies(&s, i, &lx.heredocs);
                    for (body, quoted, seg) in bodies {
                        if !quoted {
                            substitutions(&body, &mut lx.nested);
                        }
                        let mut t = token(Kind::Heredoc, body, seg);
                        t.quoted = quoted;
                        lx.tokens.push(t);
                    }
                    lx.heredocs.clear();
                    i = end;
                }
            }
            // A CR is not a bash metacharacter: inside a heredoc tag it is part of the word (`EOF` + CR).
            '\r' if lx.expect_tag.is_some() && lx.has_word && reading().tags_as_words => {
                lx.buf.push('\r');
                i += 1;
            }
            ' ' | '\t' | '\r' => {
                lx.flush();
                i += 1;
            }
            '#' if !lx.has_word => i = find(&s, '\n', i).unwrap_or(s.len()),
            '<' | '>' | '&' if ch != '&' || at(i + 1) == Some('>') => {
                let mut fd = String::new();
                if lx.has_word
                    && !lx.quoted
                    && !lx.buf.is_empty()
                    && lx.buf.chars().all(|c| c.is_ascii_digit())
                {
                    fd = std::mem::take(&mut lx.buf);
                    lx.has_word = false;
                } else {
                    lx.flush();
                }
                let (text, len) = [
                    ("<<<", 3),
                    ("<<-", 3),
                    ("<<", 2),
                    ("&>>", 3),
                    ("&>", 2),
                    (">>", 2),
                    (">|", 2),
                ]
                .into_iter()
                .find(|(p, _)| starts(i, p))
                .map_or((ch.to_string(), 1), |(p, n)| (p.to_string(), n));
                let mut j = i + len;
                if text == "<<" || text == "<<-" {
                    lx.expect_tag = Some(text == "<<-");
                    i = j;
                    continue;
                }
                let mut text = text;
                let mut dup = false;
                if (text == "<" || text == ">")
                    && at(j) == Some('&')
                    && at(j + 1).is_some_and(|c| c.is_ascii_digit() || c == '-')
                {
                    let mut k = j + 1;
                    while at(k).is_some_and(|c| c.is_ascii_digit() || c == '-') {
                        k += 1;
                    }
                    text += &sub(&s, j, k);
                    j = k;
                    dup = true;
                }
                let mut t = token(Kind::Op, format!("{fd}{text}"), lx.seg);
                t.read = text.starts_with('<') && text != "<<<";
                t.dup = dup;
                lx.tokens.push(t);
                i = j;
            }
            ';' => {
                let two = at(i + 1) == Some(';');
                lx.sep(if two { ";;" } else { ";" });
                i += if two { 2 } else { 1 };
            }
            '|' => {
                let (text, n) = match at(i + 1) {
                    Some('|') => ("||", 2),
                    Some('&') => ("|&", 2),
                    _ => ("|", 1),
                };
                lx.sep(text);
                i += n;
            }
            '&' => {
                let two = at(i + 1) == Some('&');
                lx.sep(if two { "&&" } else { "&" });
                i += if two { 2 } else { 1 };
            }
            '(' | ')' => {
                lx.sep(&ch.to_string());
                i += 1;
            }
            '{' | '}'
                if !lx.has_word
                    && at(i + 1).is_none_or(|n| n.is_whitespace() || ";&|)".contains(n)) =>
            {
                lx.sep(&ch.to_string());
                i += 1;
            }
            _ => {
                lx.has_word = true;
                lx.buf.push(ch);
                i += 1;
            }
        }
    }
    lx.flush();
    (lx.tokens, lx.nested)
}
