//! Where a shell command moves, read from its words and never run: the walk `automated_guard` judges and verify-path
//! grounding reads. It reads the shell's words (ADR 0292, [`shell`]), so quoted text is no boundary: a guest links the
//! reader from `specforge-protocol-types`, which also reaches a nested `bash -c '…'` that a host input of
//! one arg would not. The tables below are fixed sets, and what they leave out passes: `worktree-path-safety.md` step 0c
//! states that boundary.

use specforge_extension_sdk::shell::{self, Op};

/// A payload nested deeper than this is not read: the bound keeps a pathological nest finite.
const MAX_DEPTH: usize = 4;

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "do", "while", "until", "!", "{", "coproc",
];
const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash", "ksh"];
pub(crate) const PACKAGE_TOOLS: &[&str] = &["npm", "npx", "pnpm", "yarn", "bun"];
pub(crate) const PACKAGE_DIR_FLAGS: &[&str] = &["--prefix", "--cwd", "--dir", "-C"];
const MAKE_DIR_FLAGS: &[&str] = &["-C", "--directory"];
/// `make`'s other short options that take a word of their own.
const MAKE_OPERANDS: &str = "fIoW";
/// A launcher and its options that only ask about a command: nothing runs.
const PROBES: &[(&str, &[char])] = &[("command", &['v', 'V'])];
/// `env`'s option whose text is a command line of its own.
const SPLIT_STRING: &[&str] = &["-S", "--split-string"];

/// A command that runs the command in its words: its options that take an operand, those that move the command it runs,
/// and the operands before that command.
struct Launcher {
    name: &'static str,
    operands: &'static [&'static str],
    chdir: &'static [&'static str],
    positionals: usize,
}

const fn launcher(
    name: &'static str,
    operands: &'static [&'static str],
    chdir: &'static [&'static str],
    positionals: usize,
) -> Launcher {
    Launcher {
        name,
        operands,
        chdir,
        positionals,
    }
}

const LAUNCHERS: &[Launcher] = &[
    launcher(
        "env",
        &["-u", "--unset", "-S", "--split-string"],
        &["-C", "--chdir"],
        0,
    ),
    launcher("time", &["-o", "--output", "-f", "--format"], &[], 0),
    launcher("timeout", &["-s", "--signal", "-k", "--kill-after"], &[], 1),
    launcher("nohup", &[], &[], 0),
    launcher("setsid", &[], &[], 0),
    launcher(
        "stdbuf",
        &["-i", "-o", "-e", "--input", "--output", "--error"],
        &[],
        0,
    ),
    launcher("nice", &["-n", "--adjustment"], &[], 0),
    launcher("ionice", &["-c", "-n", "--class", "--classdata"], &[], 0),
    launcher(
        "sudo",
        &["-u", "-g", "-h", "-p", "-r", "-t", "-T", "-U", "-C", "-R"],
        &["-D", "--chdir"],
        0,
    ),
    launcher("builtin", &[], &[], 0),
    launcher("exec", &["-a"], &[], 0),
    launcher("command", &[], &[], 0),
];

/// `path`'s parts with `.` and `..` read lexically, and whether a `..` went above where it started. A link is not
/// followed: the one reading of a path for the guard and the verify-path grounding.
pub(crate) fn normal_parts(path: &str) -> (Vec<&str>, bool) {
    let mut parts = Vec::new();
    let mut above = false;
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => above |= parts.pop().is_none(),
            part => parts.push(part),
        }
    }
    (parts, above)
}

/// One thing a command line does to where it runs, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// `cd` or `pushd`: the cwd moves. The target as written.
    Cd(String),
    /// A directory flag: where one tool runs, the cwd stays.
    Flag {
        flag: String,
        target: String,
    },
    /// A word that names an absolute path.
    Absolute(String),
    /// A command's words, without its keywords, assignments and launchers.
    Run(Vec<String>),
    /// A subshell opens: the cwd reached inside does not reach what follows its [`Event::Leave`].
    Enter,
    Leave,
}

/// `^MISSING(\s|$)`: the Wave-0 sentinel is not a command. Only a line that is nothing else is exempt, whatever its prose
/// says (`MISSING - can't yet`): anything after an operator is a command, and the walk reads it.
pub fn missing_sentinel(text: &str) -> bool {
    text.strip_prefix("MISSING").is_some_and(|after| {
        (after.is_empty() || after.starts_with(char::is_whitespace))
            && !after.contains([';', '&', '|', '(', ')', '`', '\n'])
    })
}

/// Whether the shell builds `target` at run time: what it names cannot be read here.
pub fn dynamic(target: &str) -> bool {
    target.contains(EXPANDS) || target.starts_with('~')
}

const EXPANDS: [char; 4] = ['$', '`', '*', '?'];

/// The directory `target` names before the first thing the shell expands: `/main/` of `/main/$X` and of `/main/a*`,
/// empty of `a*`. All of a literal target.
pub fn literal_prefix(target: &str) -> &str {
    match target.find(EXPANDS) {
        Some(at) => target[..at]
            .rfind('/')
            .map_or("", |slash| &target[..=slash]),
        None => target,
    }
}

/// The events of `line`, with `&amp;` read as `&` (a plan's text is XML). An unclosed quote is an `Err`.
pub fn events(line: &str) -> Result<Vec<Event>, String> {
    let mut out = Vec::new();
    read(&line.replace("&amp;", "&"), 0, &mut out)?;
    Ok(out)
}

fn read(line: &str, depth: usize, out: &mut Vec<Event>) -> Result<(), String> {
    let segments = shell::segments(line)?;
    let mut open = 0usize;
    for (at, segment) in segments.iter().enumerate() {
        match segment.after {
            Some(Op::Open) => {
                out.push(Event::Enter);
                open += 1;
            }
            // A `case` arm closes what nothing opened.
            Some(Op::Close) if open > 0 => {
                out.push(Event::Leave);
                open -= 1;
            }
            _ => {}
        }
        for text in segment.substitutions.iter().filter(|_| depth < MAX_DEPTH) {
            out.push(Event::Enter);
            read(text, depth + 1, out)?;
            out.push(Event::Leave);
        }
        segment
            .redirects
            .iter()
            .for_each(|word| absolute(word, out));
        if segment.words.is_empty() {
            continue;
        }
        let next = segments.get(at + 1).and_then(|next| next.after);
        let apart =
            segment.after == Some(Op::Pipe) || matches!(next, Some(Op::Pipe | Op::Background));
        if apart {
            out.push(Event::Enter);
        }
        command(&segment.words, depth, out)?;
        if apart {
            out.push(Event::Leave);
        }
    }
    out.extend(std::iter::repeat_n(Event::Leave, open));
    Ok(())
}

/// `NAME=value`.
fn assignment(word: &str) -> Option<&str> {
    let (name, value) = word.split_once('=')?;
    let mut chars = name.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    (starts && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')).then_some(value)
}

/// The absolute path a word names: bare, or after `NAME=` or `--flag=`.
fn absolute(word: &str, out: &mut Vec<Event>) {
    // An assignment's value is a list as often as a path: `PATH=/a/bin:$PATH`.
    if let Some(value) = assignment(word) {
        for value in value.split(':').filter(|value| value.starts_with('/')) {
            out.push(Event::Absolute(value.to_string()));
        }
        return;
    }
    let value = word
        .strip_prefix("--")
        .and_then(|flag| flag.split_once('='))
        .map_or(word, |(_, value)| value);
    if value.starts_with('/') {
        out.push(Event::Absolute(value.to_string()));
    }
}

fn command(words: &[String], depth: usize, out: &mut Vec<Event>) -> Result<(), String> {
    let mut rest = words;
    while let Some((first, tail)) = rest.split_first() {
        // `function f { …`: the body is the commands that follow, read where they stand.
        if first == "function" {
            rest = tail.get(1..).unwrap_or_default();
            continue;
        }
        if !(KEYWORDS.contains(&first.as_str()) || assignment(first).is_some()) {
            break;
        }
        absolute(first, out);
        rest = tail;
    }
    let Some((head, tail)) = rest.split_first() else {
        return Ok(());
    };
    // The sentinel's own command; what follows an operator is a command again.
    if head == "MISSING" {
        return Ok(());
    }
    absolute(head, out);
    let name = head.rsplit('/').next().unwrap_or(head);
    if let Some(launcher) = LAUNCHERS.iter().find(|l| l.name == name) {
        return launched(launcher, tail, depth, out);
    }
    match name {
        "cd" | "pushd" => out.push(Event::Cd(cd_target(tail))),
        // Where `popd` lands is a stack this walk does not keep.
        "popd" => out.push(Event::Cd("$OLDPWD".to_string())),
        "eval" => {
            if depth < MAX_DEPTH {
                read(&tail.join(" "), depth + 1, out)?;
            }
        }
        _ if SHELLS.contains(&name) => {
            let (script, others) = shell_script(tail);
            if let Some(script) = script.filter(|_| depth < MAX_DEPTH) {
                out.push(Event::Enter);
                read(script, depth + 1, out)?;
                out.push(Event::Leave);
            }
            others.iter().for_each(|word| absolute(word, out));
        }
        _ => match program_text(name, tail) {
            Some(at) => {
                literal_paths(&tail[at], out);
                let others = tail[..at].iter().chain(&tail[at + 1..]);
                others.for_each(|word| absolute(word, out));
                out.push(Event::Run(rest.to_vec()));
            }
            None => {
                directory_flags(name, tail, out);
                tail.iter().for_each(|word| absolute(word, out));
                out.push(Event::Run(rest.to_vec()));
            }
        },
    }
    Ok(())
}

fn cd_target(args: &[String]) -> String {
    let mut args = args.iter().map(String::as_str).peekable();
    let option = |arg: &&str| {
        arg.strip_prefix('-')
            .is_some_and(|l| !l.is_empty() && l.chars().all(|c| "LPe@nqs".contains(c)))
    };
    while args.next_if(option).is_some() {}
    args.next_if(|a| *a == "--");
    match args.next() {
        None => "~".to_string(),
        Some("-") => "$OLDPWD".to_string(),
        // `pushd +1` turns the stack.
        Some(target) if target.starts_with('+') => "$OLDPWD".to_string(),
        Some(target) => target.to_string(),
    }
}

/// The command a launcher runs, read in place.
fn launched(
    launcher: &Launcher,
    tail: &[String],
    depth: usize,
    out: &mut Vec<Event>,
) -> Result<(), String> {
    let mut at = 0;
    let mut chdir = None;
    let mut split: Vec<String> = Vec::new();
    while let Some(word) = tail.get(at) {
        if word == "--" {
            at += 1;
            break;
        }
        if !word.starts_with('-') {
            break;
        }
        at += 1;
        if word == "-" {
            continue;
        }
        // `command -v bash` asks whether it exists: nothing runs.
        let probes = PROBES.iter().find(|(name, _)| *name == launcher.name);
        if probes.is_some_and(|(_, letters)| word.contains(*letters) && !word.starts_with("--")) {
            return Ok(());
        }
        let Some((flag, glued)) = launcher_option(launcher, word) else {
            continue;
        };
        let value = glued.map(str::to_string).or_else(|| {
            at += 1;
            tail.get(at - 1).cloned()
        });
        if SPLIT_STRING.contains(&flag.as_str()) {
            if let Some(text) = value {
                split = shell::words(&text)?;
            }
            break;
        }
        if launcher.chdir.contains(&flag.as_str()) {
            chdir = value;
        }
    }
    let payload: Vec<String> = split
        .into_iter()
        .chain(
            tail.get(at + launcher.positionals..)
                .unwrap_or_default()
                .iter()
                .cloned(),
        )
        .collect();
    match chdir {
        Some(target) => {
            out.push(Event::Enter);
            out.push(Event::Cd(target));
            command(&payload, depth, out)?;
            out.push(Event::Leave);
            Ok(())
        }
        None => command(&payload, depth, out),
    }
}

/// In a cluster of short options (`-iC`, `-wCd`), the first letter that takes an operand, and what is glued after it.
fn cluster(word: &str, takes: impl Fn(char) -> bool) -> Option<(char, Option<&str>)> {
    let letters = word
        .strip_prefix('-')
        .filter(|letters| !letters.is_empty() && !letters.starts_with('-'))?;
    let (at, letter) = letters.char_indices().find(|(_, c)| takes(*c))?;
    let rest = &letters[at + letter.len_utf8()..];
    Some((letter, (!rest.is_empty()).then_some(rest)))
}

/// The option of `launcher` that `word` is and takes an operand: its name, and the operand when `word` carries it.
fn launcher_option<'a>(launcher: &Launcher, word: &'a str) -> Option<(String, Option<&'a str>)> {
    let known = |flag: &str| launcher.operands.contains(&flag) || launcher.chdir.contains(&flag);
    if word.starts_with("--") {
        let (name, value) = word
            .split_once('=')
            .map_or((word, None), |(name, value)| (name, Some(value)));
        return known(name).then(|| (name.to_string(), value));
    }
    let (letter, glued) = cluster(word, |c| known(&format!("-{c}")))?;
    Some((format!("-{letter}"), glued))
}

/// The value `word` carries itself (`--flag=x`), when it is one of the long `flags`, and whether it is one.
pub(crate) fn long_value<'a, 'b>(
    word: &'a str,
    flags: impl Iterator<Item = &'b &'b str>,
) -> (Option<&'a str>, bool) {
    for flag in flags.filter(|flag| flag.starts_with("--")) {
        if word == *flag {
            return (None, true);
        }
        if let Some(value) = word.strip_prefix(*flag).and_then(|r| r.strip_prefix('=')) {
            return (Some(value), true);
        }
    }
    (None, false)
}

/// A shell's `-c` script, and the words that are not its options or script.
fn shell_script(tail: &[String]) -> (Option<&str>, &[String]) {
    let mut at = 0;
    let mut has_c = false;
    while let Some(word) = tail.get(at) {
        if word == "--" {
            at += 1;
            break;
        }
        if !word.starts_with(['-', '+']) {
            break;
        }
        let long = word.starts_with("--");
        has_c |= !long && word.starts_with('-') && word.contains('c');
        // `-eo pipefail`, `--rcfile x`: its operand is no script.
        let operand = if long {
            matches!(word.as_str(), "--rcfile" | "--init-file")
        } else {
            word.ends_with(['o', 'O'])
        };
        at += if operand { 2 } else { 1 };
    }
    match tail.get(at).filter(|_| has_c) {
        Some(script) => (Some(script), &tail[at + 1..]),
        None => (None, tail.get(at..).unwrap_or_default()),
    }
}

/// The index of an interpreter's program text: the word after `python -c`, `node -e`, `perl -e`, `ruby -e`.
fn program_text(name: &str, tail: &[String]) -> Option<usize> {
    let takes = |word: &str| match name {
        _ if name.starts_with("python") => short_cluster_ends(word, &['c']),
        "node" => matches!(word, "--eval" | "--print") || short_cluster_ends(word, &['e', 'p']),
        "perl" => short_cluster_ends(word, &['e', 'E']),
        "ruby" => short_cluster_ends(word, &['e']),
        _ => false,
    };
    let at = tail.iter().position(|word| takes(word))?;
    (at + 1 < tail.len()).then_some(at + 1)
}

fn short_cluster_ends(word: &str, letters: &[char]) -> bool {
    !word.starts_with("--") && word.starts_with('-') && word.ends_with(letters)
}

/// Runs of absolute-path characters in program text: only a literal path can be read from a program.
fn literal_paths(text: &str, out: &mut Vec<Event>) {
    let path_char = |c: char| {
        !c.is_whitespace()
            && !matches!(
                c,
                '"' | '\'' | '`' | '(' | ')' | ',' | ';' | '[' | ']' | '{' | '}' | '='
            )
    };
    let mut rest = text;
    while let Some(at) = rest.find('/') {
        let boundary = rest[..at].chars().next_back().is_none_or(|c| !path_char(c));
        let run_len = rest[at..]
            .find(|c| !path_char(c))
            .unwrap_or(rest.len() - at);
        if boundary {
            out.push(Event::Absolute(rest[at..at + run_len].to_string()));
        }
        rest = &rest[at + run_len.max(1)..];
    }
}

/// A tool whose flags name the directory it works in.
struct DirectoryTool {
    flags: &'static [&'static str],
    /// The short options that take an operand and name no directory.
    operands: &'static str,
    /// Its first plain word ends its own options: git's subcommand. A package tool's and `make`'s may follow it.
    subcommand_ends: bool,
    /// Long and short options whose operand is no subcommand (git's).
    word_operands: &'static [&'static str],
    /// Short options cluster (`make -wC d`). git's `-C` stands alone.
    clusters: bool,
}

fn directory_tool(name: &str) -> Option<DirectoryTool> {
    let tool = |flags, operands, clusters| DirectoryTool {
        flags,
        operands,
        subcommand_ends: false,
        word_operands: &[],
        clusters,
    };
    match name {
        _ if PACKAGE_TOOLS.contains(&name) => Some(tool(PACKAGE_DIR_FLAGS, "", true)),
        "make" => Some(tool(MAKE_DIR_FLAGS, MAKE_OPERANDS, true)),
        "git" => Some(DirectoryTool {
            subcommand_ends: true,
            word_operands: &["-c", "--git-dir", "--work-tree", "--namespace"],
            ..tool(&["-C", "--directory"], "", false)
        }),
        _ => None,
    }
}

/// The directory flags of a package tool, `make` and `git`: `--flag D`, `--flag=D` or `-CD`, before a `--` (past it the
/// words are the script's), and for git before its subcommand.
fn directory_flags(name: &str, tail: &[String], out: &mut Vec<Event>) {
    let Some(tool) = directory_tool(name) else {
        return;
    };
    let flags = tool.flags;
    let mut at = 0;
    while let Some(word) = tail.get(at) {
        at += 1;
        if word == "--" {
            return;
        }
        if tool.subcommand_ends && !word.starts_with('-') {
            return;
        }
        if tool.word_operands.contains(&word.as_str()) {
            at += 1;
            continue;
        }
        // A cluster's first operand-taking letter: `make -wC d`, `make -f x`.
        let short = if tool.clusters {
            cluster(word, |c| c == 'C' || tool.operands.contains(c))
        } else {
            word.strip_prefix("-C")
                .filter(|_| !word.starts_with("--"))
                .map(|rest| ('C', (!rest.is_empty()).then_some(rest)))
        };
        if let Some((letter, glued)) = short {
            let target = glued.map(str::to_string).or_else(|| {
                at += 1;
                tail.get(at - 1).cloned()
            });
            if let Some(target) = target.filter(|_| letter == 'C') {
                out.push(Event::Flag {
                    flag: "-C".to_string(),
                    target,
                });
            }
            continue;
        }
        let (value, is_flag) = long_value(word, flags.iter());
        if !is_flag {
            continue;
        }
        let flag = flags
            .iter()
            .find(|flag| word.starts_with(**flag))
            .map_or(String::new(), ToString::to_string);
        let target = match value {
            Some(value) => Some(value.to_string()),
            None => {
                at += 1;
                tail.get(at - 1).cloned()
            }
        };
        if let Some(target) = target {
            out.push(Event::Flag { flag, target });
        }
    }
}

#[cfg(test)]
mod tests;
