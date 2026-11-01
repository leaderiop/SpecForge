//! Ported from gsd-core v1.19.0 hooks/gsd-secret-read-guard.js (`findSecretRead` and the scans it calls), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Which operand of which command of a shell line reads a secret file, and the spawn of the debugger as a CLI process.
//! Anything that runs a nested script (`eval`, `sh -c`, `source <(…)`, `xargs`, `git -c alias.x=!…`, a piped `sh`)
//! is scanned again, three levels deep.
//!
//! gsd-core v1.19.0 (#5627): the command word is found past each wrapper's own options (`WRAPPERS`). What stays
//! open, as upstream says: a wrapper, or a wrapper option taking a separate value, the table does not list, and
//! `env -S 'script'` (the split string is skipped as the option's value).

use std::collections::BTreeMap;

use super::git::{name_only, scan_git_config};
use super::names::{last_segment, names_secret};
use super::shell::{Kind, Reading, Token, stopped, tokenize, with_reading};

pub(super) const MAX_NESTING_DEPTH: usize = 3;

/// What `resolve` steps over after a wrapper: `short` letters and `long` options that take a separate value word,
/// leading `VAR=value` words (`assigns`) and positional `operands` (timeout's DURATION). From the man pages; a wrong
/// entry fails open (the option's value becomes the command word).
struct Wrapper {
    short: &'static str,
    long: &'static [&'static str],
    assigns: bool,
    operands: usize,
}

const BARE: Wrapper = Wrapper {
    short: "",
    long: &[],
    assigns: false,
    operands: 0,
};
const WRAPPERS: [(&str, Wrapper); 11] = [
    (
        "sudo",
        Wrapper {
            short: "CDghpRrTtUu",
            long: &[
                "--close-from",
                "--chdir",
                "--group",
                "--host",
                "--prompt",
                "--chroot",
                "--role",
                "--command-timeout",
                "--type",
                "--other-user",
                "--user",
            ],
            ..BARE
        },
    ),
    (
        "doas",
        Wrapper {
            short: "Cu",
            ..BARE
        },
    ),
    (
        "env",
        Wrapper {
            short: "uCSaP",
            long: &["--unset", "--chdir", "--split-string", "--argv0"],
            assigns: true,
            ..BARE
        },
    ),
    (
        "nice",
        Wrapper {
            short: "n",
            long: &["--adjustment"],
            ..BARE
        },
    ),
    ("nohup", BARE),
    ("command", BARE),
    (
        "time",
        Wrapper {
            short: "fo",
            long: &["--format", "--output"],
            ..BARE
        },
    ),
    (
        "timeout",
        Wrapper {
            short: "sk",
            long: &["--signal", "--kill-after"],
            operands: 1,
            ..BARE
        },
    ),
    (
        "stdbuf",
        Wrapper {
            short: "ioe",
            long: &["--input", "--output", "--error"],
            ..BARE
        },
    ),
    ("setsid", BARE),
    ("exec", Wrapper { short: "a", ..BARE }),
];
const NON_READING: [&str; 16] = [
    "test", "[", "[[", "ls", "stat", "touch", "rm", "chmod", "chown", "mkdir", "basename",
    "dirname", "realpath", "file", "echo", "printf",
];
pub(super) const SHELL_INTERPRETERS: [&str; 6] = ["bash", "sh", "zsh", "dash", "ksh", "su"];
const SHELL_VALUE_FLAGS: [&str; 6] = ["-o", "-O", "+o", "+O", "--rcfile", "--init-file"];
const XARGS_VALUE_FLAGS: [&str; 10] = ["-n", "-I", "-i", "-L", "-l", "-P", "-s", "-d", "-E", "-a"];

fn is_assignment(text: &str) -> bool {
    let Some((name, _)) = text.split_once('=') else {
        return false;
    };
    name.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(super) fn is_dash_c(text: &str) -> bool {
    text.len() >= 2
        && text.starts_with('-')
        && text.ends_with('c')
        && text[1..].chars().all(|c| c.is_ascii_alphabetic())
}

/// A command word after its assignments and prefixes: its basename and its operands.
pub(super) struct Command<'a> {
    pub base: String,
    pub operands: Vec<&'a Token>,
}

/// How many words the wrapper option `word` occupies: 2 when it takes a separate value word, else 1.
fn option_width(word: &str, w: &Wrapper) -> usize {
    if word.starts_with("--") {
        // getopt_long accepts any unambiguous prefix (`--adj 5`).
        let valued =
            !word.contains('=') && word.len() > 2 && w.long.iter().any(|l| l.starts_with(word));
        return if valued { 2 } else { 1 };
    }
    let valued = word
        .char_indices()
        .skip(1)
        .find(|&(_, c)| w.short.contains(c));
    // `-n5` carries its value; a valued letter last takes the next word.
    1 + usize::from(valued.is_some_and(|(i, _)| i + 1 == word.len()))
}

/// The index of the first word after a wrapper's own options, assignments and operands (`-n 5`, `-i A=1`, `--`).
fn skip_wrapper_args(words: &[&Token], start: usize, w: &Wrapper) -> usize {
    let (mut idx, mut options_ended) = (start, false);
    while let Some(word) = words.get(idx) {
        let text = word.text.as_str();
        if w.assigns && is_assignment(text) {
            idx += 1;
        } else if !options_ended && text == "--" {
            options_ended = true;
            idx += 1;
        } else if !options_ended && text.len() > 1 && text.starts_with('-') {
            idx += option_width(text, w);
        } else {
            break;
        }
    }
    if w.operands == 0 {
        return idx;
    }
    idx = (idx + w.operands).min(words.len());
    // `timeout 5 -- cmd`: some getopt builds still accept `--` after DURATION.
    idx + usize::from(words.get(idx).is_some_and(|t| t.text == "--"))
}

#[cfg(test)]
thread_local! {
    /// The words handed to `resolve` on this thread: a test bounds the wrapper walk's work by it (#5628).
    pub(super) static RESOLVED_WORDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) fn resolve<'a>(words: &[&'a Token]) -> Option<Command<'a>> {
    #[cfg(test)]
    RESOLVED_WORDS.with(|n| n.set(n.get() + words.len()));
    let mut idx = 0;
    while idx < words.len() && is_assignment(&words[idx].text) {
        idx += 1;
    }
    while let Some(word) = words.get(idx) {
        let base = last_segment(&word.text).to_lowercase();
        let Some((_, w)) = WRAPPERS.iter().find(|(name, _)| *name == base) else {
            break;
        };
        idx = skip_wrapper_args(words, idx + 1, w);
    }
    let first = words.get(idx)?;
    Some(Command {
        base: last_segment(&first.text).to_lowercase(),
        operands: words[idx + 1..].to_vec(),
    })
}

/// Whether an operand names a secret file: `@file`, `--opt=file`, and in a single-dash word (`-2if.env`) both the
/// two-character strip and the value after the run of option characters (`-if.env`, `-nf=.env`; #5046). At most
/// three name checks, whatever the word's length. The run also swallows a value's leading letters and digits, so
/// `-fapp.env` is denied although `cat app.env` is not: upstream's fail-safe false positive, pinned in the tests.
pub(super) fn operand_names_secret(text: &str) -> bool {
    let v = text.strip_prefix('@').unwrap_or(text);
    if v.starts_with("--") {
        return names_secret(v.split_once('=').map_or(v, |(_, value)| value));
    }
    let mut cs = v.chars();
    let (Some('-'), Some(second), Some(_)) = (cs.next(), cs.next(), cs.next()) else {
        return names_secret(v);
    };
    if second == '-' {
        return names_secret(v);
    }
    if names_secret(&v[1 + second.len_utf8()..]) {
        return true;
    }
    // An ASCII run, so `end` is a char boundary.
    let end = 1 + v[1..].bytes().take_while(u8::is_ascii_alphanumeric).count();
    if end > 2 && names_secret(&v[end..]) {
        return true;
    }
    v.as_bytes().get(end) == Some(&b'=') && names_secret(&v[end + 1..])
}

/// What `echo` or `printf` would print: a script some shell may read from a pipe or a process substitution.
fn reconstructed(words: &[&Token]) -> Option<String> {
    let cmd = resolve(words)?;
    let join = |ops: &[&Token]| {
        ops.iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };
    match cmd.base.as_str() {
        "echo" => {
            let skip = cmd
                .operands
                .iter()
                .take_while(|w| {
                    w.text.len() > 1
                        && w.text.starts_with('-')
                        && w.text[1..].chars().all(|c| "neE".contains(c))
                })
                .count();
            Some(join(&cmd.operands[skip..]))
        }
        "printf" => Some(join(&cmd.operands)),
        _ => None,
    }
}

fn reconstructed_procsub(inner: &str) -> Option<String> {
    let (tokens, _) = tokenize(inner);
    let words: Vec<&Token> = tokens
        .iter()
        .take_while(|t| t.kind != Kind::Sep)
        .filter(|t| t.kind == Kind::Word)
        .collect();
    reconstructed(&words)
}

/// The segments of a line, for the scans that look at the command before a pipe.
struct Segments<'a> {
    by_seg: BTreeMap<usize, Vec<&'a Token>>,
    sep_after: BTreeMap<usize, &'a str>,
}

impl Segments<'_> {
    /// `(operator, previous segment)` before segment `s`: the last non-grouping separator, and the nearest earlier
    /// segment that holds a word.
    fn preceding(&self, s: usize) -> (Option<&str>, Option<usize>) {
        let Some(prev) = (0..s).rev().find(|p| {
            self.by_seg
                .get(p)
                .is_some_and(|ts| ts.iter().any(|t| t.kind == Kind::Word))
        }) else {
            return (None, None);
        };
        let op = (prev..s)
            .filter_map(|q| self.sep_after.get(&q).copied())
            .rfind(|t| !["(", ")", "{", "}"].contains(t));
        (op, Some(prev))
    }

    fn words_of(&self, seg: usize) -> Vec<&Token> {
        self.by_seg
            .get(&seg)
            .map(|ts| {
                ts.iter()
                    .copied()
                    .filter(|t| t.kind == Kind::Word)
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn scan_shell_interpreter(
    operands: &[&Token],
    heredocs: &[&Token],
    here_strings: &[String],
    s: usize,
    segs: &Segments,
    depth: usize,
) -> Option<String> {
    if let Some(c) = operands.iter().position(|w| is_dash_c(&w.text)) {
        return operands
            .get(c + 1)
            .and_then(|script| find_secret_read(&script.text, depth + 1));
    }
    let mut m = 0;
    let mut file = None;
    while m < operands.len() {
        if SHELL_VALUE_FLAGS.contains(&operands[m].text.as_str()) {
            m += 2;
            continue;
        }
        if !operands[m].text.starts_with('-') {
            file = Some(operands[m]);
            break;
        }
        m += 1;
    }
    if let Some(f) = file {
        return f
            .procsub
            .as_deref()
            .and_then(reconstructed_procsub)
            .and_then(|src| find_secret_read(&src, depth + 1));
    }
    for h in heredocs {
        if let Some(hit) = find_secret_read(&h.text, depth + 1) {
            return Some(hit);
        }
    }
    for hs in here_strings {
        if let Some(hit) = find_secret_read(hs, depth + 1) {
            return Some(hit);
        }
    }
    let (op, prev) = segs.preceding(s);
    if matches!(op, Some("|" | "|&"))
        && let Some(prev) = prev
    {
        return reconstructed(&segs.words_of(prev))
            .and_then(|src| find_secret_read(&src, depth + 1));
    }
    None
}

fn scan_xargs(operands: &[&Token], s: usize, segs: &Segments, depth: usize) -> Option<String> {
    let (mut arg_file, mut sub) = (false, None);
    let mut m = 0;
    while m < operands.len() {
        let t = operands[m].text.as_str();
        if t == "-a" || t == "--arg-file" {
            arg_file = true;
            m += 2;
        } else if t.starts_with("--arg-file=") {
            arg_file = true;
            m += 1;
        } else if XARGS_VALUE_FLAGS.contains(&t) {
            m += 2;
        } else if t.starts_with('-') {
            m += 1;
        } else {
            sub = Some(m);
            break;
        }
    }
    let sub = sub?;
    let sub_base = last_segment(&operands[sub].text).to_lowercase();
    if SHELL_INTERPRETERS.contains(&sub_base.as_str())
        && let Some(hit) = scan_shell_interpreter(&operands[sub + 1..], &[], &[], s, segs, depth)
    {
        return Some(hit);
    }
    if arg_file || NON_READING.contains(&sub_base.as_str()) {
        return None;
    }
    let (op, prev) = segs.preceding(s);
    if !matches!(op, Some("|" | "|&")) {
        return None;
    }
    let prev_cmd = resolve(&segs.words_of(prev?))?;
    prev_cmd
        .operands
        .iter()
        .find(|w| operand_names_secret(&w.text))
        .map(|w| w.text.clone())
}

/// The first secret file `command` reads, as it is written.
pub(super) fn find_secret_read(command: &str, depth: usize) -> Option<String> {
    if stopped() {
        return None;
    }
    let (tokens, nested) = tokenize(command);
    if stopped() {
        return None;
    }
    if depth < MAX_NESTING_DEPTH {
        for sub in &nested {
            if let Some(hit) = find_secret_read(sub, depth + 1) {
                return Some(hit);
            }
        }
    }
    let mut segs = Segments {
        by_seg: BTreeMap::new(),
        sep_after: BTreeMap::new(),
    };
    let mut heredocs: BTreeMap<usize, Vec<&Token>> = BTreeMap::new();
    let mut max_seg = 0;
    for t in &tokens {
        max_seg = max_seg.max(t.seg);
        match t.kind {
            Kind::Sep => {
                segs.sep_after.insert(t.seg, t.text.as_str());
            }
            Kind::Heredoc => heredocs.entry(t.seg).or_default().push(t),
            _ => segs.by_seg.entry(t.seg).or_default().push(t),
        }
    }
    for s in 0..=max_seg {
        if stopped() {
            return None;
        }
        let Some(seg_tokens) = segs.by_seg.get(&s) else {
            continue;
        };
        let (mut words, mut here_strings) = (Vec::new(), Vec::new());
        let mut k = 0;
        while k < seg_tokens.len() {
            let t = seg_tokens[k];
            k += 1;
            if t.kind != Kind::Op {
                words.push(t);
            } else if !t.dup
                && let Some(target) = seg_tokens.get(k).filter(|n| n.kind == Kind::Word)
            {
                k += 1;
                if t.text.ends_with("<<<") {
                    here_strings.push(target.text.clone());
                } else if t.read && names_secret(&target.text) {
                    return Some(target.text.clone());
                }
            }
        }
        let Some(cmd) = resolve(&words) else {
            continue;
        };
        let operands = &cmd.operands;
        let base = cmd.base.as_str();
        if base == "eval" {
            if depth < MAX_NESTING_DEPTH {
                let script = operands
                    .iter()
                    .map(|w| w.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                if let Some(hit) = find_secret_read(&script, depth + 1) {
                    return Some(hit);
                }
            }
            continue;
        }
        if base == "source" || base == "." {
            for w in operands {
                if let Some(inner) = &w.procsub {
                    if depth < MAX_NESTING_DEPTH
                        && let Some(src) = reconstructed_procsub(inner)
                        && let Some(hit) = find_secret_read(&src, depth + 1)
                    {
                        return Some(hit);
                    }
                } else if operand_names_secret(&w.text) {
                    return Some(w.text.clone());
                }
            }
            continue;
        }
        if depth < MAX_NESTING_DEPTH {
            let hit = if base == "xargs" {
                scan_xargs(operands, s, &segs, depth)
            } else {
                None
            };
            let hit = hit.or_else(|| {
                if SHELL_INTERPRETERS.contains(&base) {
                    let here = heredocs.get(&s).map(Vec::as_slice).unwrap_or_default();
                    scan_shell_interpreter(operands, here, &here_strings, s, &segs, depth)
                } else {
                    None
                }
            });
            let hit = hit.or_else(|| {
                if base == "git" || base == "git.exe" {
                    scan_git_config(operands, depth)
                } else {
                    None
                }
            });
            if hit.is_some() {
                return hit;
            }
        }
        if NON_READING.contains(&base) {
            continue;
        }
        let exempt = name_only(base, operands);
        for (k, o) in operands.iter().enumerate() {
            if !exempt.contains(&k) && operand_names_secret(&o.text) {
                return Some(o.text.clone());
            }
        }
    }
    None
}

/// The same scan under the other readings of a heredoc (#5513): bash 3.2's `$( )` span, then the earlier tag
/// reading. Only a command that opens a heredoc can read differently, so any other is not scanned again.
pub(super) fn find_secret_read_under_other_readings(command: &str) -> Option<String> {
    if !command.contains("<<") {
        return None;
    }
    [(false, true), (true, false)].into_iter().find_map(
        |(bodies_skipped_in_spans, tags_as_words)| {
            let r = Reading {
                bodies_skipped_in_spans,
                tags_as_words,
            };
            with_reading(r, || find_secret_read(command, 0))
        },
    )
}
