//! A shell tool's command line as words (ADR 0292), read by POSIX rules for what a hook needs: single quotes are
//! literal, double quotes keep their text but for `\"`, `\\`, `\$` and `` \` ``, a backslash escapes the next
//! character, and an unquoted `;`, `&`, `|`, `(`, `)` or newline ends a command. No expansion is done (`$VAR` stays
//! as written); a hook reads what was typed, not what the shell would run.

/// The commands of `line`: each is its words, split at the control operators. An unclosed quote or a trailing
/// backslash is an `Err` naming it.
pub fn commands(line: &str) -> Result<Vec<Vec<String>>, String> {
    let mut commands: Vec<Vec<String>> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut started = false;
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
                    word.push(c);
                }
                None => return Err("a backslash ends the line".into()),
            },
            ';' | '&' | '|' | '(' | ')' | '\n' => {
                end(&mut words, &mut word, &mut started);
                if !words.is_empty() {
                    commands.push(std::mem::take(&mut words));
                }
            }
            c if c.is_whitespace() => end(&mut words, &mut word, &mut started),
            c => {
                started = true;
                word.push(c);
            }
        }
    }
    end(&mut words, &mut word, &mut started);
    if !words.is_empty() {
        commands.push(words);
    }
    Ok(commands)
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
    fn shell_refuses_an_unclosed_quote() {
        assert!(commands("echo 'a").is_err());
        assert!(commands("echo \"a").is_err());
        assert!(commands("echo a\\").is_err());
    }
}
