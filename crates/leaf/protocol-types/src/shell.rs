//! A shell tool's command line as words (ADR 0292), read by POSIX rules for what a hook needs: single quotes are
//! literal, double quotes keep their text but for `\"`, `\\`, `\$` and `` \` ``, a backslash escapes the next
//! character, and an unquoted `;`, `&`, `|`, `(`, `)` or newline ends a command. An unquoted redirect (`>`, `>>`, `<`,
//! `<<`, `<<<`, `2>&1`, `>&2`, `&>`, `&>>`, `>|`, `<>`) ends the word before it and is no part of the command's words:
//! a file target is kept beside them, and digits are its file descriptor only as a word of their own (`2>x`, not `a2>x`).
//! No expansion is done (`$VAR` stays as written); a hook reads what was typed, not what the shell would run.
//!
//! The one reader of host and guest: the host hands a hook the commands of its line (ADR 0292 D4), and a guest that holds
//! a command as text reads it here.

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
    pub after: Option<Op>,
}

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
        if !self.words.is_empty() || !self.redirects.is_empty() || self.after.is_some() {
            self.segments.push(Segment {
                words: std::mem::take(&mut self.words),
                redirects: std::mem::take(&mut self.redirects),
                after: self.after,
            });
        }
    }
}

/// The segments of `line`. An unclosed quote or a trailing backslash is an `Err` naming it.
pub fn segments(line: &str) -> Result<Vec<Segment>, String> {
    let mut reading = Reading {
        segments: Vec::new(),
        after: None,
        words: Vec::new(),
        redirects: Vec::new(),
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
            c if c.is_whitespace() => reading.end_word(),
            c => reading.begin_bare(c),
        }
    }
    reading.end_command();
    Ok(reading.segments)
}

/// The commands of `line`: each is its words, split at the control operators. An unclosed quote or a trailing
/// backslash is an `Err` naming it.
pub fn commands(line: &str) -> Result<Vec<Vec<String>>, String> {
    Ok(segments(line)?
        .into_iter()
        .map(|segment| segment.words)
        .filter(|words| !words.is_empty())
        .collect())
}

/// Every word of `line`, in order, the commands one after the other.
pub fn words(line: &str) -> Result<Vec<String>, String> {
    Ok(commands(line)?.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests;
