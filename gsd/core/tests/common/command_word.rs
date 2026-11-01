//! Which commands a line runs: a word stands as a command, not as an argument (`command -v jq`, gh's `--jq`) or inside
//! a quoted string (`echo "jq"`). The scans that keep a tool out of the shell (`port_tools`' jq and specforge-only
//! scans, and the rendered lines') share it.
//!
//! A command word is at the line's start or after `|`, `;`, `&`, `(` (and `$(`), `{`, `!` or a backtick, after a shell
//! keyword (`if`, `then`, `else`, `elif`, `do`, `while`, `until`), after a case arm's pattern (`(x) cmd`, `x) cmd`), or
//! after a prefix that runs its argument (`xargs`, `env`, `command` without `-v`/`-V`, `time`, `timeout`, `nohup`,
//! `exec`, `nice`, `sudo`), with that prefix's options, their number or `{}` values, and `NAME=value` words between. A
//! command given by path (`/usr/bin/jq`) is its last component. The body of `sh -c '…'` / `bash -c "…"` (`/bin/sh` too)
//! is a line of its own and is scanned the same way.

/// The shell keywords a command follows.
const KEYWORDS: &[&str] = &["if", "then", "else", "elif", "do", "while", "until"];

/// The commands that run their argument as a command.
const PREFIXES: &[&str] = &[
    "xargs", "env", "command", "time", "timeout", "nohup", "exec", "nice", "sudo",
];

/// Whether `c` continues a word, so a tool's name next to it is part of a longer word (`jq-1`, `--jq`, `pjq`).
fn joins(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/'
}

/// Whether `word` is an option's value a prefix takes: a number or duration (`-n 1`, `timeout 5`, `30s`) or `{}`.
fn is_value(word: &str) -> bool {
    word == "{}"
        || word
            .trim_end_matches(['s', 'm', 'h', 'd'])
            .parse::<f64>()
            .is_ok()
}

/// Whether the text before a word, `before`, puts that word in command position.
fn in_command_position(before: &[char]) -> bool {
    let text: String = before.iter().collect();
    let text = text.trim_end();
    let Some(last) = text.chars().next_back() else {
        return true;
    };
    // `X=( a b )` lists words, it runs none
    if last == '(' && text[..text.len() - 1].ends_with('=') {
        return false;
    }
    if "|;&({!`".contains(last) {
        return true;
    }
    if last == ')' {
        // a case arm's pattern, `(x)` or `x)`, stands where a command would: `$(…)` and `(…)` that ran end here too
        let pattern = text.rsplit(char::is_whitespace).next().unwrap_or(text);
        if pattern.contains("$(")
            || pattern.contains('`')
            || pattern[..pattern.len() - 1].contains(')')
        {
            return false;
        }
        let head: Vec<char> = text[..text.len() - pattern.len()].chars().collect();
        return arm_starts(&head);
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
        // `X=$(git rev-parse`: an assignment whose `$(` is still open is the line before the command, not a word of it
        let open = word.matches('(').count() > word.matches(')').count();
        if word.contains('=') && !word.starts_with('=') && !open {
            assignments = true;
            continue;
        }
        // `xargs -n 1 jq`, `timeout 5 jq`: the value of an option or of the prefix itself
        if is_value(word)
            && n > 0
            && (words[n - 1].starts_with('-') || PREFIXES.contains(&words[n - 1]))
        {
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

/// Whether a case arm's pattern can start after `before`: at the line's start, after `;;` or after `in`.
fn arm_starts(before: &[char]) -> bool {
    let text: String = before.iter().collect();
    let text = text.trim_end();
    text.is_empty() || text.ends_with(";;") || text == "in" || text.ends_with(" in")
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

/// Each command word of `code` (already masked), by its name (a path's last component), with the char index after it.
/// A variable (`$X`, `${X}`), an arithmetic `$((…))`, an assignment's name (`X=1`), an option and a number are not one.
fn command_words(code: &[char]) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < code.len() {
        if !joins(code[at]) || (at > 0 && joins(code[at - 1])) {
            at += 1;
            continue;
        }
        let end = (at..code.len())
            .find(|&i| !joins(code[i]))
            .unwrap_or(code.len());
        let word: String = code[at..end].iter().collect();
        // A command word follows a blank or an operator: not `$X`, `${s:i}`, a value (`X=none`, `"auto"` in prose) or a
        // placeholder's member (`a|b`, `{name}`); and it is not an assignment's name or a word glued to what follows
        // (`f(`, `a[0]`, `T-{n}`, `x$Y`, `x@y`, `x:`).
        let separated = at == 0 || code[at - 1].is_whitespace() || "|;&(`!".contains(code[at - 1]);
        let variable =
            !separated || (at > 1 && code[at - 1] == '|' && !code[at - 2].is_whitespace());
        let assignment = matches!(
            code.get(end),
            Some('=' | '(' | '[' | '{' | '|' | '$' | '@' | ':')
        ) || code[end..].starts_with(&['+', '=']);
        let name = word.rsplit('/').next().unwrap_or("").to_string();
        // `/gsd-plan-phase` is a slash command, not a program at the root
        let slash = word.starts_with('/') && word.matches('/').count() == 1;
        // a masked quote is `_`s: a word of a quoted string is no command
        let plain = name.chars().any(|c| c.is_ascii_alphabetic()) && !word.starts_with('-');
        // `none)` / `(a|b)` is a case arm's pattern
        let start = (0..at)
            .rev()
            .find(|&k| code[k].is_whitespace())
            .map_or(0, |k| k + 1);
        let stop = (end..code.len())
            .find(|&k| code[k].is_whitespace())
            .unwrap_or(code.len());
        let token: String = code[start..stop].iter().collect();
        let opens = token.matches('(').count();
        let pattern = token.ends_with(')')
            && !token.contains(['$', '`'])
            && (opens == 0 || (opens == 1 && token.starts_with('(')))
            && arm_starts(&code[..start]);
        if plain
            && !slash
            && !variable
            && !assignment
            && !pattern
            && in_command_position(&code[..at])
        {
            out.push((name, end));
        }
        at = end;
    }
    out
}

/// The bodies of the `sh -c '…'` and `bash -c "…"` runs of `line` (`masked` is its mask), unquoted.
fn shell_bodies(line: &[char], masked: &[char]) -> Vec<String> {
    let mut out = Vec::new();
    for (_, end) in command_words(masked)
        .into_iter()
        .filter(|(name, _)| name == "sh" || name == "bash")
    {
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
    out
}

/// The commands the shell line `line` runs: its command words outside the quoted text, and those of its `sh -c` bodies.
pub fn commands(line: &str) -> Vec<String> {
    block_commands(line)
        .into_iter()
        .map(|(_, name)| name)
        .collect()
}

/// Whether the shell line `line` runs `tool`.
pub fn runs(line: &str, tool: &str) -> bool {
    commands(line).iter().any(|c| c == tool)
}

/// The commands a run of prose, `text`, runs. Prose names a tool as often as it runs it, so a command word counts when
/// an argument that only a run has follows it (an option, a quoted or `.` filter, a `$` variable, `[`, `{` or `@`), or
/// when it ends a pipe (`… | jq`). Quoted text is not masked: an apostrophe in prose is not a quote. A `sh -c` body is
/// a shell line.
///
/// shortcut: a run whose first argument is a bare word (`jq length f`) is read as prose, as English reads the same
/// (`if jq is missing`); mask the run when a prompt starts carrying such filters.
pub fn commands_in_prose(text: &str) -> Vec<String> {
    let raw: Vec<char> = text.chars().collect();
    let argument = |(name, end): &(String, usize)| {
        // an argument is a word of its own: `Claude's`, `jq.` and `jq,` are prose
        if raw
            .get(*end)
            .is_some_and(|c| !c.is_whitespace() && !")|;`".contains(*c))
        {
            return false;
        }
        let rest: String = raw[*end..].iter().collect();
        let after = rest.trim_start();
        match after.chars().next() {
            Some(c) if "-.'\"$[{@".contains(c) => true,
            None | Some(')' | ';' | '|' | '`') => {
                let before: String = raw[..*end].iter().collect();
                let before = before.trim_end();
                // the word as written: a path's components before its name
                let word = before.rsplit(|c: char| !joins(c)).next().unwrap_or(name);
                before[..before.len() - word.len()]
                    .trim_end()
                    .ends_with('|')
            }
            _ => false,
        }
    };
    // `passed | gaps_found | human_needed` lists values: each side of each `|` is one bare word
    let listed = text.contains(" | ")
        && text.split('|').all(|side| {
            let side = side.trim();
            !side.is_empty() && side.chars().all(|c| joins(c) && c != '/')
        });
    let mut out: Vec<String> = command_words(&raw)
        .into_iter()
        .filter(|w| !listed && argument(w))
        .map(|(name, _)| name)
        .collect();
    for body in shell_bodies(&raw, &mask(text).chars().collect::<Vec<_>>()) {
        out.extend(commands(&body));
    }
    out
}

/// Whether a run of prose, `text`, runs `tool` ([`commands_in_prose`]).
pub fn runs_in_prose(text: &str, tool: &str) -> bool {
    commands_in_prose(text).iter().any(|c| c == tool)
}

/// `text` with what the shell does not run masked, char for char and line for line: the text of a quoted string (a
/// `$(…)` or backtick inside double quotes stays, it runs) becomes `_`, its newlines stay, an unquoted `#` that starts a
/// word ends its line, and the body of a heredoc (`<<TAG`, `<<-'TAG'`) is masked whole, its closing line too.
///
/// shortcut: an unquoted heredoc's `$(…)` runs, but is masked with the body; read it when a block starts using one.
pub fn mask(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    // the open contexts, innermost last: `'"'` a double-quoted string, `'('` a `$(…)` or `(`, `'`'` a backtick
    let mut open: Vec<char> = Vec::new();
    let mut heredocs: Vec<String> = Vec::new();
    let mut out = String::with_capacity(text.len());
    let blank = |x: char| if x == '\n' { '\n' } else { '_' };
    let mut prev = ' ';
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        let quoted = open.last() == Some(&'"');
        i += 1;
        match ch {
            '\n' if !quoted && !heredocs.is_empty() => {
                out.push('\n');
                for tag in std::mem::take(&mut heredocs) {
                    while i < c.len() {
                        let end = (i..c.len()).find(|&k| c[k] == '\n').unwrap_or(c.len());
                        let line: String = c[i..end].iter().collect();
                        out.extend(c[i..(end + 1).min(c.len())].iter().map(|x| blank(*x)));
                        i = end + 1;
                        if line.trim() == tag {
                            break;
                        }
                    }
                }
            }
            '\\' => {
                out.push(if quoted { '_' } else { ch });
                if let Some(&n) = c.get(i) {
                    out.push(if quoted || n == '\n' { blank(n) } else { n });
                    i += 1;
                }
            }
            '\'' if !quoted => {
                out.push('_');
                while let Some(&n) = c.get(i) {
                    out.push(blank(n));
                    i += 1;
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
            // `$((…))` and `((…))` are arithmetic: their words are numbers and variables, not commands
            '$' | '('
                if (ch == '$' && c[i..].starts_with(&['(', '(']))
                    || (!quoted && c[i - 1..].starts_with(&['(', '('])) =>
            {
                let from = if ch == '$' { i + 2 } else { i + 1 };
                out.extend(&c[i - 1..from]);
                let mut depth = 0;
                i = from;
                while i < c.len() && !(depth == 0 && c[i..].starts_with(&[')', ')'])) {
                    depth += i32::from(c[i] == '(') - i32::from(c[i] == ')');
                    out.push(blank(c[i]));
                    i += 1;
                }
                out.extend(c.get(i..(i + 2).min(c.len())).into_iter().flatten());
                i = (i + 2).min(c.len());
            }
            '$' if c.get(i) == Some(&'(') => {
                i += 1;
                open.push('(');
                out.push_str("$(");
            }
            '`' if open.last() == Some(&'`') => {
                open.pop();
                out.push(ch);
            }
            '`' => {
                open.push('`');
                out.push(ch);
            }
            _ if quoted => out.push(blank(ch)),
            '(' => {
                open.push('(');
                out.push(ch);
            }
            ')' => {
                if open.last() == Some(&'(') {
                    open.pop();
                }
                out.push(ch);
            }
            '#' if prev.is_whitespace() || out.is_empty() => {
                while c.get(i).is_some_and(|n| *n != '\n') {
                    i += 1;
                }
            }
            '<' if c.get(i) == Some(&'<') && c.get(i + 1) != Some(&'<') && prev != '<' => {
                // `<<-'TAG'`: the tag, its quotes and dash written as code, the body masked from the next line
                let mut j = i + 1;
                j += usize::from(c.get(j) == Some(&'-'));
                while c.get(j).is_some_and(|x| *x == ' ') {
                    j += 1;
                }
                let quote = c.get(j).filter(|x| **x == '\'' || **x == '"').is_some();
                let from = j + usize::from(quote);
                let to = (from..c.len())
                    .find(|&k| !(c[k].is_alphanumeric() || c[k] == '_'))
                    .unwrap_or(c.len());
                if to > from {
                    heredocs.push(c[from..to].iter().collect());
                    let end = (to + usize::from(quote)).min(c.len());
                    out.push(ch);
                    out.extend(
                        c[i..end]
                            .iter()
                            .map(|x| if "'\"".contains(*x) { '_' } else { *x }),
                    );
                    i = end;
                } else {
                    out.push(ch);
                }
            }
            _ => out.push(ch),
        }
        prev = ch;
    }
    out
}

/// The commands each line of a shell block (its lines joined by `\n`) runs, as `(line index, command)`: the block is
/// masked whole, so a string or heredoc that spans lines runs nothing.
pub fn block_commands(block: &str) -> Vec<(usize, String)> {
    let masked = mask(block);
    let mut out = Vec::new();
    // a line that ends with `\` goes on in the next: the logical line is read whole, each word kept on its own line
    let (mut raw, mut code, mut line_of) = (Vec::new(), Vec::new(), Vec::new());
    let lines: Vec<(&str, &str)> = block.split('\n').zip(masked.split('\n')).collect();
    for (n, (r, m)) in lines.iter().enumerate() {
        let (r, m): (Vec<char>, Vec<char>) = (r.chars().collect(), m.chars().collect());
        let goes_on = m.last() == Some(&'\\') && n + 1 < lines.len();
        let keep = if goes_on { m.len() - 1 } else { m.len() };
        // the raw text is cut where the masked line is (a comment), so the two stay char for char
        raw.extend(r.iter().take(keep).copied().chain([' ']));
        code.extend(m[..keep].iter().copied().chain([' ']));
        line_of.extend(std::iter::repeat_n(n, keep + 1));
        if goes_on {
            continue;
        }
        out.extend(
            command_words(&code)
                .into_iter()
                .map(|(name, end)| (line_of[end - 1], name)),
        );
        for body in shell_bodies(&raw, &code) {
            out.extend(commands(&body).into_iter().map(|name| (n, name)));
        }
        (raw, code, line_of) = (Vec::new(), Vec::new(), Vec::new());
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
        // a case arm's pattern, with or without its `(`
        ("(json) jq -r .a f ;;", true),
        ("json) jq -r .a f ;;", true),
        ("case \"$F\" in (json) jq .a f ;; esac", true),
        ("a) x ;; b) jq .a f ;;", true),
        ("X=$(foo) jq", false),
        ("echo $(foo) jq", false),
        // a prefix's values, and the prefixes that run their argument
        ("timeout 5 jq .a f", true),
        ("timeout 30s jq .a f", true),
        ("xargs -n 1 jq .a", true),
        ("xargs -I {} jq .a {}", true),
        ("nohup jq .a f &", true),
        ("exec jq .a f", true),
        ("nice -n 10 jq .a f", true),
        ("sudo jq .a f", true),
        ("grep -m 1 jq f", false),
        // by path, and a shell by path
        ("/usr/bin/jq .a f", true),
        ("x | /usr/local/bin/jq .a", true),
        ("/bin/sh -c 'x | jq .a'", true),
        (r#"/bin/bash -c "jq -r .a f""#, true),
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
        ("ls x | jq", true),
        ("(ls x | jq)", true),
        ("passed | gaps_found | jq", false),
        ("(jq .a f)", true),
        ("{ jq .a f; }", true),
        ("if jq -e . f; then", true),
        ("X=$(jq -r .a f)", true),
        ("with jq and git (jq is a requirement", false),
        ("if jq is missing, install it", false),
        ("don't run jq here", false),
        ("sh -c 'x | jq .a'", true),
        ("/usr/bin/jq -r .a f", true),
        ("x | /usr/bin/jq", true),
        // shortcut: a bare first argument reads as English
        ("jq length f", false),
        // a slash command, a possessive, an enumeration of values
        ("/gsd-plan-phase --gaps", false),
        ("jq's -r flag", false),
        ("passed | gaps_found | human_needed", false),
    ] {
        assert_eq!(runs_in_prose(text, "jq"), runs_jq, "{text}");
    }
}

#[test]
fn a_line_names_every_command_it_runs() {
    let cases: &[(&str, &[&str])] = &[
        ("X=$(git rev-parse HEAD | head -1)", &["git", "head"]),
        (
            "if [ -f \"$F\" ]; then cat \"$F\"; fi",
            &["if", "then", "cat", "fi"],
        ),
        ("FOO=1 BAR=2 specforge query x", &["specforge"]),
        ("echo ${X} $((Y + 1)) \"$(date -u)\"", &["echo", "date"]),
        ("\"$CMD\" --flag", &[]),
        ("sh -c 'ls | wc -l'", &["sh", "ls", "wc"]),
    ];
    for (line, expected) in cases {
        assert_eq!(commands(line), *expected, "{line}");
    }
}

#[test]
fn a_block_runs_nothing_in_a_string_heredoc_or_arithmetic_across_lines() {
    let block = [
        "X=$(cat <<'EOF'",
        "grep inside a heredoc",
        "EOF",
        ")",
        "node -e \"",
        "const a = 1; grep(a)",
        "\"",
        "for ((i=0; i<3; i++)); do wc -l f; done",
        "git log \\",
        "  --oneline | head -1",
        "case \"$K\" in",
        "  none) sort f ;;",
        "  (a|b) tr a b ;;",
        "esac",
        "f() { cut -d: -f1; }",
    ]
    .join("\n");
    let found: Vec<(usize, String)> = block_commands(&block);
    let expected: Vec<(usize, String)> = [
        (0, "cat"),
        (4, "node"),
        (7, "for"),
        (7, "do"),
        (7, "wc"),
        (7, "done"),
        (8, "git"),
        (9, "head"),
        (10, "case"),
        (11, "sort"),
        (12, "tr"),
        (13, "esac"),
        (14, "cut"),
    ]
    .into_iter()
    .map(|(n, t)| (n, t.to_string()))
    .collect();
    assert_eq!(found, expected);
}
