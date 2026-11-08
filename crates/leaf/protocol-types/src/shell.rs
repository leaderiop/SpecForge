//! A shell tool's command line as words (ADR 0292), read by POSIX rules for what a hook needs: single quotes are
//! literal, double quotes keep their text but for `\"`, `\\`, `\$` and `` \` ``, a backslash escapes the next
//! character, and an unquoted `;`, `&`, `|`, `(`, `)` or newline ends a command. An unquoted redirect (`>`, `>>`, `<`,
//! `<<`, `<<<`, `2>&1`, `>&2`, `&>`, `&>>`, `>|`, `<>`) ends the word before it and is no part of the command's words:
//! a file target is kept beside them, and digits are its file descriptor only as a word of their own (`2>x`, not `a2>x`).
//! No expansion is done (`$VAR` stays as written); a hook reads what was typed, not what the shell would run. A command
//! substitution (`$(…)`, `` `…` ``, bare or in double quotes) stays in its word as written, and runs, so its text is kept
//! beside the word and [`commands`] reads it.
//!
//! The one reader of host and guest: the host hands a hook the commands of its line (ADR 0292 D4), and a guest that holds
//! a command as text reads it here.

type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

/// A control operator: what comes before a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    And,
    Or,
    Semi,
    Newline,
    Pipe,
    Background,
    Open,
    Close,
}

/// One command of a line, and the operator before it. Two operators in a row (`(a) && b`) leave a segment with no words
/// between them, so none is lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The command's words, without its redirects, wherever they stood.
    pub words: Vec<String>,
    /// Where it redirects to or from, as written: the target of `>x` or `<x`, the `1` of `2>&1`. A heredoc's tag and a
    /// here-string's text name no file and are not kept.
    pub redirects: Vec<String>,
    /// The text of each `$(…)` and `` `…` `` in this command's words: they run before it.
    pub substitutions: Vec<String>,
    pub after: Option<Op>,
}

/// How deep `$(…)` nests before it is no longer read: real commands stay below it, and the bound keeps a guest's stack
/// and a hostile line's work (each level reads its text once more).
const MAX_NEST: usize = 16;

/// What the word being read is for.
#[derive(Clone, Copy, PartialEq)]
enum Next {
    Word,
    Redirect,
    /// A heredoc's tag or a here-string's text.
    Text,
}

/// The command being read.
struct Reading {
    segments: Vec<Segment>,
    after: Option<Op>,
    words: Vec<String>,
    redirects: Vec<String>,
    substitutions: Vec<String>,
    /// A `$(` found no `)` to the end of the line: the later ones read as text, so a long line takes one such scan.
    unclosed: bool,
    word: String,
    started: bool,
    /// Nothing of the word was quoted: bare digits are a file descriptor, a quoted `2` is an argument.
    bare: bool,
    next: Next,
}

impl Reading {
    fn begin_bare(&mut self, c: char) {
        if !self.started {
            self.bare = true;
        }
        self.started = true;
        self.word.push(c);
    }

    fn begin_quoted(&mut self) {
        self.started = true;
        self.bare = false;
    }

    /// A `` ` `` was read: the word keeps the substitution as written, its text is kept beside.
    fn backtick(&mut self, chars: &mut Chars<'_>) -> Result<(), String> {
        let body = backtick_text(chars)?;
        self.word.push('`');
        self.word.push_str(&body);
        self.word.push('`');
        self.substitutions.push(body);
        Ok(())
    }

    /// A `$` before a `(` was read and is in the word. A `(` no `)` closes is left to whoever reads on.
    fn dollar_paren(&mut self, chars: &mut Chars<'_>) {
        if self.unclosed {
            return;
        }
        let mut ahead = chars.clone();
        ahead.next();
        match dollar_paren(&mut ahead, 0) {
            Some(body) => {
                *chars = ahead;
                self.word.push('(');
                self.word.push_str(&body);
                self.word.push(')');
                if !hands_over_a_text(&body) {
                    self.substitutions.push(body);
                }
            }
            None => self.unclosed = true,
        }
    }

    fn end_word(&mut self) {
        if std::mem::take(&mut self.started) {
            let word = std::mem::take(&mut self.word);
            match std::mem::replace(&mut self.next, Next::Word) {
                Next::Word => self.words.push(word),
                Next::Redirect => self.redirects.push(word),
                Next::Text => {}
            }
        }
    }

    /// A redirect's `>` or `<` was read: bare digits, or bash's `{name}`, right before it are its file descriptor.
    fn end_word_before_redirect(&mut self) {
        let fd = self.started
            && self.bare
            && (self.word.bytes().all(|b| b.is_ascii_digit())
                || self
                    .word
                    .strip_prefix('{')
                    .and_then(|w| w.strip_suffix('}'))
                    .is_some_and(|name| {
                        !name.is_empty()
                            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    }));
        if fd {
            self.word.clear();
            self.started = false;
        } else {
            self.end_word();
        }
    }

    fn end_command(&mut self) {
        self.end_word();
        self.next = Next::Word;
        if !self.words.is_empty()
            || !self.redirects.is_empty()
            || !self.substitutions.is_empty()
            || self.after.is_some()
        {
            self.segments.push(Segment {
                words: std::mem::take(&mut self.words),
                redirects: std::mem::take(&mut self.redirects),
                substitutions: std::mem::take(&mut self.substitutions),
                after: self.after,
            });
        }
    }
}

/// Reads a `$(…)` from after its `(`: its text, or `None` when no `)` closes it (a heredoc's lone `'` can hide it, and
/// the caller then reads the characters as plain text of its double quotes).
fn dollar_paren(chars: &mut Chars<'_>, depth: usize) -> Option<String> {
    if depth > MAX_NEST {
        return None;
    }
    let mut body = String::new();
    let mut open = 1;
    while let Some(c) = chars.next() {
        match c {
            '(' => open += 1,
            ')' => {
                open -= 1;
                if open == 0 {
                    return Some(body);
                }
            }
            '\\' => {
                body.push(c);
                body.push(chars.next()?);
                continue;
            }
            '\'' => {
                body.push(c);
                loop {
                    let c = chars.next()?;
                    body.push(c);
                    if c == '\'' {
                        break;
                    }
                }
                continue;
            }
            '"' => {
                body.push(c);
                loop {
                    let c = chars.next()?;
                    body.push(c);
                    match c {
                        '"' => break,
                        '\\' => body.push(chars.next()?),
                        '$' if chars.next_if_eq(&'(').is_some() => {
                            body.push('(');
                            body.push_str(&dollar_paren(chars, depth + 1)?);
                            body.push(')');
                        }
                        _ => {}
                    }
                }
                continue;
            }
            _ => {}
        }
        body.push(c);
    }
    None
}

/// Reads a `` `…` `` from after its opening backtick: its text with the escapes of the backtick form undone.
fn backtick_text(chars: &mut Chars<'_>) -> Result<String, String> {
    let mut body = String::new();
    loop {
        match chars.next() {
            Some('`') => return Ok(body),
            Some('\\') => match chars.next() {
                Some(c @ ('`' | '\\' | '$')) => body.push(c),
                Some(c) => {
                    body.push('\\');
                    body.push(c);
                }
                None => return Err("a backtick is not closed".into()),
            },
            Some(c) => body.push(c),
            None => return Err("a backtick is not closed".into()),
        }
    }
}

/// `$(cat <<TAG` and nothing else on its first line: the idiom that hands a text to a command, whose lines run nothing.
fn hands_over_a_text(body: &str) -> bool {
    let first = body.trim_start().lines().next().unwrap_or_default();
    first
        .strip_prefix("cat")
        .map(str::trim_start)
        .and_then(|rest| rest.strip_prefix("<<"))
        .is_some_and(|tag| {
            let tag = tag.trim();
            !tag.is_empty() && !tag.contains(|c: char| c.is_whitespace() || ";&|()<>`$".contains(c))
        })
}

/// The segments of `line`. An unclosed quote or a trailing backslash is an `Err` naming it.
pub fn segments(line: &str) -> Result<Vec<Segment>, String> {
    let mut reading = Reading {
        segments: Vec::new(),
        after: None,
        words: Vec::new(),
        redirects: Vec::new(),
        substitutions: Vec::new(),
        unclosed: false,
        word: String::new(),
        started: false,
        bare: false,
        next: Next::Word,
    };
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                reading.begin_quoted();
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => reading.word.push(c),
                        None => return Err("a single quote is not closed".into()),
                    }
                }
            }
            '"' => {
                reading.begin_quoted();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\' | '$' | '`')) => reading.word.push(c),
                            Some('\n') => {}
                            Some(c) => {
                                reading.word.push('\\');
                                reading.word.push(c);
                            }
                            None => return Err("a double quote is not closed".into()),
                        },
                        Some('`') => reading.backtick(&mut chars)?,
                        Some('$') if chars.peek() == Some(&'(') => {
                            reading.word.push('$');
                            reading.dollar_paren(&mut chars);
                        }
                        Some(c) => reading.word.push(c),
                        None => return Err("a double quote is not closed".into()),
                    }
                }
            }
            '\\' => match chars.next() {
                Some('\n') => {}
                Some(c) => {
                    reading.begin_quoted();
                    reading.word.push(c);
                }
                None => return Err("a backslash ends the line".into()),
            },
            '&' if chars.peek() == Some(&'>') => {
                reading.end_word();
                chars.next();
                chars.next_if_eq(&'>');
                reading.next = Next::Redirect;
            }
            '>' => {
                reading.end_word_before_redirect();
                chars.next_if_eq(&'>');
                let _ = chars.next_if_eq(&'&').or_else(|| chars.next_if_eq(&'|'));
                reading.next = Next::Redirect;
            }
            '<' => {
                reading.end_word_before_redirect();
                reading.next = if chars.next_if_eq(&'<').is_some() {
                    let _ = chars.next_if_eq(&'<').or_else(|| chars.next_if_eq(&'-'));
                    Next::Text
                } else {
                    let _ = chars.next_if_eq(&'&').or_else(|| chars.next_if_eq(&'>'));
                    Next::Redirect
                };
            }
            ';' | '&' | '|' | '(' | ')' | '\n' => {
                reading.end_command();
                reading.after = Some(match c {
                    ';' => Op::Semi,
                    '\n' => Op::Newline,
                    '(' => Op::Open,
                    ')' => Op::Close,
                    '&' if chars.next_if_eq(&'&').is_some() => Op::And,
                    '&' => Op::Background,
                    _ if chars.next_if_eq(&'|').is_some() => Op::Or,
                    _ => {
                        chars.next_if_eq(&'&');
                        Op::Pipe
                    }
                });
            }
            '`' => {
                if !reading.started {
                    reading.bare = false;
                }
                reading.started = true;
                reading.backtick(&mut chars)?;
            }
            '$' if chars.peek() == Some(&'(') => {
                reading.begin_bare('$');
                reading.bare = false;
                reading.dollar_paren(&mut chars);
            }
            c if c.is_whitespace() => reading.end_word(),
            c => reading.begin_bare(c),
        }
    }
    reading.end_command();
    Ok(reading.segments)
}

/// The commands of `line`, each its words, split at the control operators; a command substitution's come first, as
/// they run first. An unclosed quote, a trailing backslash or a nest past [`MAX_NEST`] is an `Err` naming it.
pub fn commands(line: &str) -> Result<Vec<Vec<String>>, String> {
    fn read(line: &str, depth: usize, out: &mut Vec<Vec<String>>) -> Result<(), String> {
        if depth > MAX_NEST {
            return Err("command substitutions nest too deep".into());
        }
        for segment in segments(line)? {
            for text in &segment.substitutions {
                read(text, depth + 1, out)?;
            }
            if !segment.words.is_empty() {
                out.push(segment.words);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    read(line, 0, &mut out)?;
    Ok(out)
}

/// Every word of `line`, in order, the commands one after the other.
pub fn words(line: &str) -> Result<Vec<String>, String> {
    Ok(commands(line)?.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests;
