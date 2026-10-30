//! The shell a ported workflow shows (REVIEW-r42 M4). The port rule checks which lines changed, not whether the new
//! line is valid, so a mechanical `.planning/x` -> "the spec" swap left `rm -f the spec` in a bash fence. For each fenced
//! bash block that differs from upstream's: it still parses where upstream's did, and none of its code lines carries
//! prose standing in for a path.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use regex::Regex;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

/// `(label, ours, upstream's)` as body lines of every asset that has an upstream file of the same line count.
fn pairs() -> Vec<(String, Vec<String>, Vec<String>)> {
    let base = root().join("assets");
    let vendored = root().join("../parity/upstream/v1.18.0");
    let mut paths = Vec::new();
    walk(&base, &mut paths);
    paths.sort();
    let mut out = Vec::new();
    for p in paths {
        let rel = p
            .strip_prefix(&base)
            .expect("under assets")
            .display()
            .to_string();
        if rel.ends_with(".compact.md") {
            continue;
        }
        let plain = vendored.join(&rel);
        let up = if plain.is_file() {
            plain
        } else {
            vendored.join(format!("{rel}.txt"))
        };
        let split = |text: &str| -> Vec<String> { text.split('\n').map(String::from).collect() };
        let mine = split(&std::fs::read_to_string(&p).expect("an asset"))[1..].to_vec();
        let mut theirs = split(
            &std::fs::read_to_string(&up).unwrap_or_else(|e| panic!("{}: {e}", up.display())),
        );
        if rel.starts_with("commands/") || rel.starts_with("agents/") {
            let end = theirs[1..]
                .iter()
                .position(|l| l == "---")
                .expect("frontmatter")
                + 2;
            theirs.drain(..end);
        }
        assert_eq!(mine.len(), theirs.len(), "{rel}: line count");
        out.push((rel, mine, theirs));
    }
    out
}

/// The bash and sh fences of `lines` as `(first code line, end)` (0-based, end exclusive).
fn fences(lines: &[String]) -> Vec<(usize, usize)> {
    let open = Regex::new(r"^\s*```(bash|sh)\s*$").expect("a regex");
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        if let Some(s) = start {
            if line.trim_start().starts_with("```") {
                out.push((s, i));
                start = None;
            }
        } else if open.is_match(line) {
            start = Some(i + 1);
        } else if line.trim_start().starts_with("```") {
            // another fence: skip to its end
            start = None;
        }
    }
    out
}

fn parses(code: &str) -> bool {
    let mut child = Command::new("bash")
        .arg("-n")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("bash");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(code.as_bytes())
        .expect("write");
    child.wait().expect("bash -n").success()
}

#[test]
fn a_changed_bash_block_still_parses() {
    let mut checked = 0;
    let mut broken = Vec::new();
    for (label, mine, theirs) in pairs() {
        for (s, e) in fences(&mine) {
            if mine[s..e] == theirs[s..e] {
                continue;
            }
            checked += 1;
            if parses(&theirs[s..e].join("\n")) && !parses(&mine[s..e].join("\n")) {
                broken.push(format!("{label}:{}", s + 2));
            }
        }
    }
    assert!(checked > 100, "{checked} changed blocks");
    assert!(
        broken.is_empty(),
        "blocks that no longer parse:\n{}",
        broken.join("\n")
    );
}

#[test]
fn no_bash_code_line_carries_prose_for_a_path() {
    let prose = Regex::new(
        r"\bthe spec\b|\bthe pending todos\b|\bthe computed\b|\bthe `\w+` (and `\w+` )?entit(y|ies)\b",
    )
    .expect("a regex");
    let mut found = Vec::new();
    for (label, mine, theirs) in pairs() {
        for (s, e) in fences(&mine) {
            for n in s..e {
                let code = mine[n].trim_start();
                // a `#` or `: #` line is a comment: it may say why there is nothing to run
                if mine[n] != theirs[n]
                    && !code.starts_with('#')
                    && !code.starts_with(": #")
                    && prose.is_match(code)
                {
                    found.push(format!("{label}:{}: {code}", n + 2));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "prose where a path or command belongs:\n{}",
        found.join("\n")
    );
}

/// REVIEW-r43 Minor 1: `the spec` joined to the word or path after it (`the specquick/`, `the spec.continue-here.md`,
/// `the `phase` entities03-auth`), in prose and in inline code as well as in a fence. The pattern is not `\bthe spec\b`:
/// the join has no word boundary after `spec`.
#[test]
fn no_text_joins_the_spec_to_a_path() {
    let joined = Regex::new(
        r"\bthe spec(?:[A-Za-z]+/|[A-Za-z-]*\.md|\.[A-Za-z])|\)the spec|entit(?:y|ies)[0-9]",
    )
    .expect("a regex");
    let mut found = Vec::new();
    for (label, mine, _) in pairs() {
        for (n, line) in mine.iter().enumerate() {
            if joined.is_match(line) {
                found.push(format!("{label}:{}: {line}", n + 2));
            }
        }
    }
    // the Rust side writes text too: its string literals
    let mut sources = Vec::new();
    walk_rs(&root().join("src"), &mut sources);
    for path in sources {
        let text = std::fs::read_to_string(&path).expect("a source");
        for (n, line) in text.lines().enumerate() {
            if joined.is_match(line) {
                found.push(format!("{}:{}: {line}", path.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "joined words:\n{}", found.join("\n"));
}

fn walk_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
