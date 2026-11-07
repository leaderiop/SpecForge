//! A shell tool's command line as words (ADR 0292), read by POSIX rules for what a hook needs: single quotes are
//! literal, double quotes keep their text but for `\"`, `\\`, `\$` and `` \` ``, a backslash escapes the next
//! character, and an unquoted `;`, `&`, `|`, `(`, `)` or newline ends a command, but for the `&` of a redirect (`2>&1`,
//! `&>x`). No expansion is done (`$VAR` stays as written); a hook reads what was typed, not what the shell would run.
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
    pub words: Vec<String>,
    pub after: Option<Op>,
}

/// The segments of `line`. An unclosed quote or a trailing backslash is an `Err` naming it.
pub fn segments(line: &str) -> Result<Vec<Segment>, String> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut after = None;
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut started = false;
    // The word's last character was written bare: a quoted `>` opens no redirect.
    let mut bare = false;
    let mut chars = line.chars().peekable();
    let end = |words: &mut Vec<String>, word: &mut String, started: &mut bool| {
        if std::mem::take(started) {
            words.push(std::mem::take(word));
        }
    };
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                started = true;
                bare = false;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => word.push(c),
                        None => return Err("a single quote is not closed".into()),
                    }
                }
            }
            '"' => {
                started = true;
                bare = false;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\' | '$' | '`')) => word.push(c),
                            Some('\n') => {}
                            Some(c) => {
                                word.push('\\');
                                word.push(c);
                            }
                            None => return Err("a double quote is not closed".into()),
                        },
                        Some(c) => word.push(c),
                        None => return Err("a double quote is not closed".into()),
                    }
                }
            }
            '\\' => match chars.next() {
                Some('\n') => {}
                Some(c) => {
                    started = true;
                    bare = false;
                    word.push(c);
                }
                None => return Err("a backslash ends the line".into()),
            },
            '&' if (bare && word.ends_with(['>', '<'])) || chars.peek() == Some(&'>') => {
                started = true;
                word.push(c);
            }
            ';' | '&' | '|' | '(' | ')' | '\n' => {
                end(&mut words, &mut word, &mut started);
                if !words.is_empty() || after.is_some() {
                    segments.push(Segment {
                        words: std::mem::take(&mut words),
                        after,
                    });
                }
                after = Some(match c {
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
            c if c.is_whitespace() => end(&mut words, &mut word, &mut started),
            c => {
                started = true;
                bare = true;
                word.push(c);
            }
        }
    }
    end(&mut words, &mut word, &mut started);
    if !words.is_empty() || after.is_some() {
        segments.push(Segment { words, after });
    }
    Ok(segments)
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
