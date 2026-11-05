//! The assets' `specforge files` lines: every flag one names is a flag of that verb, and the agents' searches, run
//! as the assets hold them under bash with no grep on the `PATH`, find what they look for.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use super::Shell;
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

/// The catalogs' sample project: a protected page, a use of `process.env.API_KEY`, a 12-line file that exports
/// nothing, and a hard-coded count.
fn sample() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a project dir");
    let src = dir.path().join("src");
    std::fs::create_dir(&src).expect("src");
    std::fs::write(
        src.join("dashboard.tsx"),
        "<a href=\"/dashboard\">dashboard</a>\n",
    )
    .expect("a page");
    std::fs::write(src.join("a.ts"), "const key = process.env.API_KEY;\n").expect("a use");
    let thin: String = (1..=12).map(|n| format!("const a{n} = {n};\n")).collect();
    std::fs::write(dir.path().join("thin.ts"), thin).expect("a thin file");
    std::fs::write(dir.path().join("c.ts"), "let count = 5;\n").expect("a count");
    dir
}

/// The first line of `text` that contains `needle`.
fn line_with(text: &str, needle: &str) -> String {
    text.lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line contains {needle}"))
        .to_string()
}

/// The lines of `text` from the first that starts (after its indent) with `start` through the next line that holds
/// only `}`.
fn function(text: &str, start: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let a = lines
        .iter()
        .position(|l| l.trim_start().starts_with(start))
        .unwrap_or_else(|| panic!("no line starts with {start}"));
    let b = a + lines[a..]
        .iter()
        .position(|l| l.trim() == "}")
        .unwrap_or_else(|| panic!("no closing brace after {start}"));
    lines[a..=b].join("\n")
}

const CHECKER: &str = "agents/gsd-integration-checker.md";
const PATTERNS: &str = "gsd-core/references/verification-patterns.md";

/// Step 4 of the integration checker lists the pages whose name says they need a login.
#[test]
fn the_protected_pages_are_found() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = sample();
    let step = super::block(
        &super::asset(CHECKER),
        "protected_patterns=",
        "\"$protected_patterns\"",
    );
    let (out, err, _) = sh.run_in(p.path(), &step);
    assert_eq!(out, "src/dashboard.tsx\n", "{step}\n{err}");
}

/// The environment check finds a file that reads the variable through `process.env`.
#[test]
fn the_env_variable_use_is_found() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = sample();
    let search = line_with(&super::asset(PATTERNS), r"process\.env\.$VAR_NAME");
    let (out, err, _) = sh.run_in(p.path(), &format!("VAR_NAME=API_KEY\n{search}"));
    assert_eq!(
        out, "src/a.ts:1:13: const key = process.env.API_KEY;\n",
        "{search}\n{err}"
    );
}

/// `check_substantive` on a file long enough but without the pattern reports one count of lines and one of matches.
#[test]
fn a_thin_file_names_its_count_once() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = sample();
    let check = function(&super::asset(PATTERNS), "check_substantive() {");
    let (out, err, _) = sh.run_in(
        p.path(),
        &format!("{check}\ncheck_substantive thin.ts 10 export"),
    );
    assert_eq!(out, "THIN: thin.ts (12 lines, 0 matches)\n", "{err}");
}

/// The stub catalog's hard-coded count check finds `count = 5`.
#[test]
fn a_hardcoded_count_is_found() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = sample();
    let check = line_with(&super::asset(PATTERNS), r"count.*=.*\d+");
    let (out, err, _) = sh.run_in(p.path(), &format!("file=c.ts\n{check}"));
    assert_eq!(out, "c.ts:1:5: let count = 5;\n", "{check}\n{err}");
}
