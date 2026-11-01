//! Whether a line runs a tool: the tool stands as a command word, not as an argument (`command -v jq`, gh's `--jq`)
//! or inside a quoted string (`echo "jq"`). The scans that keep a tool out of the shell (`port_tools`' jq and the
//! rendered lines') share it.
//!
//! A command word is at the line's start or after `|`, `;`, `&`, `(` (and `$(`), `{`, `!` or a backtick, after a shell
//! keyword (`if`, `then`, `else`, `elif`, `do`, `while`, `until`), or after a prefix that runs its argument (`xargs`,
//! `env`, `command` without `-v`/`-V`, `time`), with that prefix's options and `NAME=value` words between. The body
//! of `sh -c '…'` / `bash -c "…"` is a line of its own and is scanned the same way.

/// The shell keywords a command follows.
const KEYWORDS: &[&str] = &["if", "then", "else", "elif", "do", "while", "until"];

/// The commands that run their argument as a command.
const PREFIXES: &[&str] = &["xargs", "env", "command", "time"];

/// Whether `c` continues a word, so a tool's name next to it is part of a longer word (`jq-1`, `--jq`, `pjq`).
fn joins(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/'
}

/// Whether the text before a word, `before`, puts that word in command position.
fn in_command_position(before: &[char]) -> bool {
    let text: String = before.iter().collect();
    let text = text.trim_end();
    let Some(last) = text.chars().next_back() else {
        return true;
    };
    if "|;&({!`".contains(last) {
        return true;
    }
    // the words before, last first: options and `NAME=value` words are skipped back to the word that decides
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut skipped_options = Vec::new();
    let mut assignments = false;
    for (n, word) in words.iter().enumerate().rev() {
        if word.starts_with('-') {
            skipped_options.push(*word);
            continue;
        }
        if word.contains('=') && !word.starts_with('=') {
            assignments = true;
            continue;
        }
        let head = &before[..word_start(before, n)];
        if KEYWORDS.contains(word) {
            return skipped_options.is_empty();
        }
        if PREFIXES.contains(word) {
            // `command -v jq` looks jq up, it does not run it
            let looks_up =
                *word == "command" && skipped_options.iter().any(|o| *o == "-v" || *o == "-V");
            return !looks_up && in_command_position(head);
        }
        // `FOO=1 jq`: assignments alone before the word, the word is the command
        return false;
    }
    // only options and assignments: a command when no option came first
    skipped_options.is_empty() && assignments
}

/// The char index in `before` where its `n`th whitespace-separated word starts.
fn word_start(before: &[char], n: usize) -> usize {
    let mut seen = 0;
    let mut in_word = false;
    for (i, c) in before.iter().enumerate() {
        if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            if seen == n {
                return i;
            }
            seen += 1;
            in_word = true;
        }
    }
    before.len()
}

/// Each place `tool` stands as a command word in `code` (already masked): the char index after its name.
fn command_words(code: &[char], tool: &str) -> Vec<usize> {
    let name: Vec<char> = tool.chars().collect();
    let mut out = Vec::new();
    for at in 0..(code.len() + 1).saturating_sub(name.len()) {
        let end = at + name.len();
        if code[at..end] != name[..]
            || (at > 0 && joins(code[at - 1]))
            || code.get(end).is_some_and(|c| joins(*c))
        {
            continue;
        }
        if in_command_position(&code[..at]) {
            out.push(end);
        }
    }
    out
}

/// The bodies of the `sh -c '…'` and `bash -c "…"` runs of `line` (`masked` is its mask), unquoted.
fn shell_bodies(line: &[char], masked: &[char]) -> Vec<String> {
    let mut out = Vec::new();
    for shell in ["sh", "bash"] {
        for end in command_words(masked, shell) {
            let rest: String = line[end..].iter().collect();
            let rest = rest.trim_start();
            let Some(flags) = rest.split_whitespace().next() else {
                continue;
            };
            if !(flags.starts_with('-') && !flags.starts_with("--") && flags.ends_with('c')) {
                continue;
            }
            let quoted = rest[flags.len()..].trim_start();
            let mut chars = quoted.chars();
            let body = match chars.next() {
                Some('\'') => chars.take_while(|c| *c != '\'').collect(),
                Some('"') => {
                    let mut body = String::new();
                    while let Some(c) = chars.next() {
                        match c {
                            '\\' => body.extend(chars.next()),
                            '"' => break,
                            _ => body.push(c),
                        }
                    }
                    body
                }
                _ => continue,
            };
            out.push(body);
        }
    }
    out
}

/// Whether the shell line `line` runs `tool`: a command word outside the quoted text, or in a `sh -c` body.
pub fn runs(line: &str, tool: &str) -> bool {
    let raw: Vec<char> = line.chars().collect();
    let masked: Vec<char> = mask(line).chars().collect();
    !command_words(&masked, tool).is_empty()
        || shell_bodies(&raw, &masked)
            .iter()
            .any(|body| runs(body, tool))
}

/// Whether a run of prose, `text`, runs `tool`. Prose names the tool as often as it runs it, so a command word counts
/// when an argument that only a run has follows it (an option, a quoted or `.` filter, a `$` variable, `[`, `{` or
/// `@`), or when it ends a pipe (`… | jq`). Quoted text is not masked: an apostrophe in prose is not a quote. A
/// `sh -c` body is a shell line.
///
/// shortcut: a run whose first argument is a bare word (`jq length f`) is read as prose; mask the run when a
/// prompt starts carrying such filters.
pub fn runs_in_prose(text: &str, tool: &str) -> bool {
    let raw: Vec<char> = text.chars().collect();
    let argument = |end: usize| {
        let rest: String = raw[end..].iter().collect();
        let after = rest.trim_start();
        match after.chars().next() {
            Some(c) if "-.'\"$[{@".contains(c) => true,
            None | Some(')' | ';' | '|' | '`') => {
                let before: String = raw[..end - tool.chars().count()].iter().collect();
                before.trim_end().ends_with('|')
            }
            _ => false,
        }
    };
    command_words(&raw, tool).into_iter().any(argument)
        || shell_bodies(&raw, &mask(text).chars().collect::<Vec<_>>())
            .iter()
            .any(|body| runs(body, tool))
}

/// `line` with what the shell does not run masked, char for char: the text of a quoted string (a `$(…)` or backtick
/// inside double quotes stays, it runs) becomes `_`, and an unquoted `#` that starts a word ends the line.
pub fn mask(line: &str) -> String {
    // the open contexts, innermost last: `'"'` a double-quoted string, `'('` a `$(…)` or `(`, `'`'` a backtick
    let mut open: Vec<char> = Vec::new();
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut prev = ' ';
    while let Some(c) = chars.next() {
        let quoted = open.last() == Some(&'"');
        match c {
            '\\' => {
                out.push(if quoted { '_' } else { c });
                if let Some(n) = chars.next() {
                    out.push(if quoted { '_' } else { n });
                }
            }
            '\'' if !quoted => {
                out.push('_');
                for n in chars.by_ref() {
                    out.push('_');
                    if n == '\'' {
                        break;
                    }
                }
            }
            '"' if quoted => {
                open.pop();
                out.push('_');
            }
            '"' => {
                open.push('"');
                out.push('_');
            }
            '$' if chars.peek() == Some(&'(') => {
                chars.next();
                open.push('(');
                out.push_str("$(");
            }
            '`' if open.last() == Some(&'`') => {
                open.pop();
                out.push(c);
            }
            '`' => {
                open.push('`');
                out.push(c);
            }
            _ if quoted => out.push('_'),
            '(' => {
                open.push('(');
                out.push(c);
            }
            ')' => {
                if open.last() == Some(&'(') {
                    open.pop();
                }
                out.push(c);
            }
            '#' if prev.is_whitespace() || out.is_empty() => break,
            _ => out.push(c),
        }
        prev = c;
    }
    out
}

#[test]
fn a_command_word_runs_the_tool() {
    for (line, runs_jq) in [
        // the separators and keywords
        ("x | jq -r .a", true),
        ("X=$(jq -r .a f)", true),
        ("if ! jq -e . f; then", true),
        ("a && jq . f", true),
        ("done | jq -sc .", true),
        ("else jq -rs . f; fi", true),
        ("{ jq .a f; }", true),
        ("(jq .a f)", true),
        ("`jq .a f`", true),
        ("X=$(a | jq .b)", true),
        (r#"X="v $(a | jq -r .b)""#, true),
        ("FOO=1 jq .a f", true),
        // the prefixes, with their options and assignments
        ("ls | xargs jq .a", true),
        ("ls | xargs -0 -I{} jq .a {}", true),
        ("env FOO=1 jq .a f", true),
        ("env -i jq .a f", true),
        ("command jq .a f", true),
        ("time jq .a f", true),
        ("command -v jq >/dev/null 2>&1", false),
        ("command -V jq", false),
        // a shell's -c body
        ("sh -c 'x | jq .a'", true),
        (r#"bash -c "jq -r .a f""#, true),
        (r#"bash -ec "y; jq -r \".a\" f""#, true),
        ("sh -c 'echo jq'", false),
        ("bash script.sh jq", false),
        // not a command word
        ("# jq -r .a", false),
        (r#"echo "jq""#, false),
        ("- [e.g. jq -r '.a' from state]", false),
        (r#"echo "a | jq""#, false),
        ("foo # x; jq", false),
        ("gh pr view 1 --json state --jq .state", false),
        ("ls x/jq ./jq jq-1", false),
        ("apt install jq", false),
        ("foo -x jq", false),
    ] {
        assert_eq!(runs(line, "jq"), runs_jq, "{line}");
    }
}

#[test]
fn prose_runs_the_tool_when_a_run_s_argument_follows() {
    for (text, runs_jq) in [
        ("jq", false),
        ("jq -r .a f", true),
        ("jq -n", true),
        ("specforge export | jq '.nodes'", true),
        ("gh pr view --jq .a", false),
        ("x | jq", true),
        ("(x | jq)", true),
        ("(jq .a f)", true),
        ("{ jq .a f; }", true),
        ("if jq -e . f; then", true),
        ("X=$(jq -r .a f)", true),
        ("with jq and git (jq is a requirement", false),
        ("if jq is missing, install it", false),
        ("don't run jq here", false),
        ("sh -c 'x | jq .a'", true),
    ] {
        assert_eq!(runs_in_prose(text, "jq"), runs_jq, "{text}");
    }
}
