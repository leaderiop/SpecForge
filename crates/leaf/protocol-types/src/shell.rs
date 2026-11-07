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
mod tests {
    use super::*;

    fn one(line: &str) -> Vec<String> {
        let mut c = commands(line).unwrap();
        assert_eq!(c.len(), 1, "{c:?}");
        c.remove(0)
    }

    #[test]
    fn shell_quotes_group_words_and_escapes_keep_characters() {
        assert_eq!(
            one(r#"git commit -m "feat(03): a \"b\"" 'it''s'"#),
            ["git", "commit", "-m", "feat(03): a \"b\"", "its"]
        );
        assert_eq!(one(r"a\ b c"), ["a b", "c"]);
        assert_eq!(one(r#"echo "$HOME" ''"#), ["echo", "$HOME", ""]);
    }

    #[test]
    fn shell_operators_end_a_command() {
        assert_eq!(
            commands("cd x && git commit -m y; echo done | cat").unwrap(),
            [
                vec!["cd", "x"],
                vec!["git", "commit", "-m", "y"],
                vec!["echo", "done"],
                vec!["cat"]
            ]
        );
        assert_eq!(commands("echo 'a;b'").unwrap(), [vec!["echo", "a;b"]]);
    }

    #[test]
    fn shell_segments_name_the_operator_before_each_command() {
        let after: Vec<Option<Op>> = segments("a && b || c; d | e & f (g) h")
            .unwrap()
            .into_iter()
            .map(|segment| segment.after)
            .collect();
        assert_eq!(
            after,
            [
                None,
                Some(Op::And),
                Some(Op::Or),
                Some(Op::Semi),
                Some(Op::Pipe),
                Some(Op::Background),
                Some(Op::Open),
                Some(Op::Close),
            ]
        );
        let quoted = segments(r#"echo 'a && b' "c || d""#).unwrap();
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].words, ["echo", "a && b", "c || d"]);
        let newline = segments("a\nb").unwrap();
        assert_eq!(newline[1].after, Some(Op::Newline));
    }

    #[test]
    fn a_redirect_is_no_boundary_and_two_operators_keep_their_order() {
        assert_eq!(commands("make 2>&1").unwrap(), [vec!["make", "2>&1"]]);
        assert_eq!(
            commands("make &>log; ls").unwrap(),
            [vec!["make", "&>log"], vec!["ls"]]
        );
        assert_eq!(commands("make '>'&").unwrap(), [vec!["make", ">"]]);
        let after: Vec<Option<Op>> = segments("(a) && b")
            .unwrap()
            .into_iter()
            .map(|segment| segment.after)
            .collect();
        assert_eq!(after, [Some(Op::Open), Some(Op::Close), Some(Op::And)]);
    }

    #[test]
    fn shell_refuses_an_unclosed_quote() {
        assert!(commands("echo 'a").is_err());
        assert!(commands("echo \"a").is_err());
        assert!(commands("echo a\\").is_err());
    }
}
