//! The assets' `specforge files` lines: every flag one names is a flag of that verb, and the agents' searches, run
//! as the assets hold them under bash with no grep on the `PATH`, find what they look for.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use crate::specforge_only::{inline_runs, shell_assets, shell_blocks};

/// The flags `specforge files <action> --help` names, long and short.
fn flags_of(action: &str) -> Vec<String> {
    let bin = std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN (see harness.rs)");
    let out = Command::new(bin)
        .args(["files", action, "--help"])
        .stdin(Stdio::null())
        .output()
        .expect("specforge runs");
    assert!(
        out.status.success(),
        "specforge files {action} --help: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let help = String::from_utf8_lossy(&out.stdout);
    let flag = regex::Regex::new(r"(?:^|[\s,\[(])(--?[A-Za-z][A-Za-z-]*)").expect("a regex");
    flag.captures_iter(&help)
        .map(|c| c[1].to_string())
        .collect()
}

/// The words of `text` up to the end of its first command (an unquoted `|`, `;`, `&`, `)`, `>`, `<`, `` ` ``, `#` or
/// newline), each with whether it began outside quotes.
fn command_words(text: &str) -> Vec<(String, bool)> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut bare = true;
    let mut quote: Option<char> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => word.push(c),
            None if c == '\'' || c == '"' => {
                if word.is_empty() {
                    bare = false;
                }
                quote = Some(c);
            }
            None if c == '\\' => {
                // A line continuation joins the next line; any other escape is the character itself.
                match chars.next() {
                    Some('\n') | None => {}
                    Some(next) => word.push(next),
                }
            }
            None if c.is_whitespace() && c != '\n' => {
                if !word.is_empty() || !bare {
                    words.push((std::mem::take(&mut word), bare));
                }
                bare = true;
            }
            None if "|;&)><`\n".contains(c) || (c == '#' && word.is_empty()) => break,
            None => word.push(c),
        }
    }
    if !word.is_empty() || !bare {
        words.push((word, bare));
    }
    words
}

/// Each `specforge files <action>` of `text`, with the flags its command names.
fn files_commands(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("specforge files ") {
        let after = &rest[at + "specforge files ".len()..];
        let words = command_words(after);
        if let Some(((action, true), args)) = words.split_first()
            && action.chars().all(|c| c.is_ascii_lowercase())
        {
            let flags = args
                .iter()
                .filter(|(word, bare)| {
                    *bare && word.starts_with('-') && word != "-" && word != "--"
                })
                .map(|(word, _)| word.split('=').next().unwrap_or(word).to_string())
                .collect();
            out.push((action.clone(), flags));
        }
        rest = after;
    }
    out
}

/// A flag an asset names on `specforge files <action>` that the verb does not have would fail where the asset runs.
#[test]
fn every_files_line_names_a_flag_the_verb_has() {
    let mut known: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut unknown = Vec::new();
    let mut seen = 0;
    for (file, lines) in shell_assets() {
        let fenced = shell_blocks(&lines)
            .into_iter()
            .map(|block| block.join("\n"));
        let inline = inline_runs(&lines)
            .into_iter()
            .map(|(_, run)| run.to_string());
        for text in fenced.chain(inline) {
            for (action, flags) in files_commands(&text) {
                seen += 1;
                let has = known
                    .entry(action.clone())
                    .or_insert_with(|| flags_of(&action));
                unknown.extend(
                    flags
                        .into_iter()
                        .filter(|flag| !has.contains(flag))
                        .map(|flag| format!("{file}: specforge files {action} {flag}")),
                );
            }
        }
    }
    assert!(seen > 20, "only {seen} specforge files lines found");
    assert!(
        unknown.is_empty(),
        "flags the verb does not have:\n{}",
        unknown.join("\n")
    );
}

#[test]
fn a_command_ends_at_its_first_separator_and_quoted_words_are_no_flags() {
    assert_eq!(
        files_commands(
            "specforge files search '-x' f --glob '!a' -q | grep -v y; specforge files list --kind file"
        ),
        [
            (
                "search".to_string(),
                vec!["--glob".to_string(), "-q".to_string()]
            ),
            ("list".to_string(), vec!["--kind".to_string()]),
        ]
    );
    assert_eq!(
        files_commands("specforge files search P \\\n  --and-not=import f # -z"),
        [("search".to_string(), vec!["--and-not".to_string()])]
    );
}
