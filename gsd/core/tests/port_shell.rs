//! The shell a ported workflow shows. The port rule checks which lines changed, not whether the new
//! line is valid, so a mechanical `.planning/x` -> "the spec" swap left `rm -f the spec` in a bash fence. For each fenced
//! bash block that differs from upstream's: it still parses where upstream's did (in each bash: `shells`), and none of its code lines carries
//! prose standing in for a path.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use regex::Regex;

#[path = "common/files.rs"]
mod files;
#[path = "common/pending.rs"]
mod pending;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The files under `dir` with the extension `ext`.
fn with_extension(dir: &Path, ext: &str) -> Vec<PathBuf> {
    files::under(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect()
}

/// `(label, ours, upstream's)` as body lines of every asset that has an upstream file of the same line count.
fn pairs() -> Vec<(String, Vec<String>, Vec<String>)> {
    let base = root().join("assets");
    let vendored = Path::new(gsd_kit::snapshot!());
    let mut out = Vec::new();
    for p in with_extension(&base, "md") {
        let rel = p
            .strip_prefix(&base)
            .expect("under assets")
            .display()
            .to_string();
        // a file waiting for its re-port is checked once it is re-ported
        if rel.ends_with(".compact.md") || pending::is_pending(&rel) {
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

/// The shells a block must parse in: `bash` on the `PATH`, and `/bin/bash` where it exists (macOS's is bash 3.2, which
/// cannot parse a bare `case` pattern inside `$(…)`; a `(pattern)` form parses in both).
fn shells() -> Vec<&'static str> {
    let mut out = vec!["bash"];
    if Path::new("/bin/bash").exists() {
        out.push("/bin/bash");
    }
    out
}

fn parses(bash: &str, code: &str) -> bool {
    let mut child = Command::new(bash)
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
            for bash in shells() {
                if parses(bash, &theirs[s..e].join("\n")) && !parses(bash, &mine[s..e].join("\n")) {
                    broken.push(format!("{label}:{} ({bash})", s + 2));
                }
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

/// `the spec` joined to the word or path after it (`the specquick/`, `the spec.continue-here.md`,
/// `the `phase` entities03-auth`), in prose and in inline code as well as in a fence. The pattern is not `\bthe spec\b`:
/// the join has no word boundary after `spec`.
#[test]
fn no_text_joins_the_spec_to_a_path() {
    let joined = Regex::new(
        r"\bthe spec(?:[A-Za-z]+/|[A-Za-z-]*\.md|\.[A-Za-z])|\)the spec|\bthe spec\]|entit(?:y|ies)[0-9]",
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
    let mut sources = with_extension(&root().join("src"), "rs");
    sources.extend(with_extension(&root().join("../kit/src"), "rs"));
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

/// Text the model wrote into a quoted heredoc is redirected only into a file a `specforge files temp` answer named in
/// the same block (the file, or a path under its directory): the one write ADR 0343 D2 counts as glue.
#[test]
fn a_heredoc_is_redirected_only_into_a_temp_file() {
    let heredoc = Regex::new(r#"read -rd '' (\w+) <<'"#).expect("a regex");
    let redirect = Regex::new(r#"^\s*printf '%s(?:\\n)?' "\$\{?(\w+)\}?" >>? "\$\{?(\w+)\}?"#)
        .expect("a regex");
    let mut checked = 0;
    let mut wrong = Vec::new();
    for path in with_extension(&root().join("assets"), "md") {
        let text = std::fs::read_to_string(&path).expect("an asset");
        let lines: Vec<String> = text.split('\n').map(String::from).collect();
        for (s, e) in fences(&lines) {
            let block = &lines[s..e];
            let read: Vec<&str> = block
                .iter()
                .filter_map(|l| Some(heredoc.captures(l)?.get(1)?.as_str()))
                .collect();
            for (n, line) in block.iter().enumerate() {
                let Some(c) = redirect.captures(line) else {
                    continue;
                };
                if !read.contains(&&c[1]) {
                    continue;
                }
                checked += 1;
                let temp = format!("{}=$(specforge files temp ", &c[2]);
                if !block.iter().any(|l| l.trim_start().starts_with(&temp)) {
                    wrong.push(format!("{}:{}: {line}", path.display(), s + n + 1));
                }
            }
        }
    }
    assert!(
        checked >= 3,
        "spec-phase, specless-probe-fallback and regression-gate redirect a heredoc"
    );
    assert!(
        wrong.is_empty(),
        "a heredoc redirected outside a temp file: {wrong:#?}"
    );
}

/// One wording per situation: an asset quotes its display text one way (ADR 0343 D7's prose quotation marks), a
/// phase placeholder is `{phase_id}` in the researcher and chunked planning, and the review router's note names the
/// declared command D7 gives as its example.
#[test]
fn a_situation_has_one_wording() {
    let backticked = Regex::new(r"[Dd]isplay:? `[^`]").expect("a regex");
    let quoted = Regex::new(r#"[Dd]isplay:? ""#).expect("a regex");
    let mut mixed = Vec::new();
    for path in with_extension(&root().join("assets"), "md") {
        let text = std::fs::read_to_string(&path).expect("an asset");
        if backticked.is_match(&text) && quoted.is_match(&text) {
            mixed.push(path.display().to_string());
        }
    }
    assert!(mixed.is_empty(), "display text quoted two ways: {mixed:#?}");
    let read = |rel: &str| std::fs::read_to_string(root().join("assets").join(rel)).expect(rel);
    // `${PHASE_ID}` in a block is the shell's variable, not a placeholder
    let upper = Regex::new(r"(^|[^$])\{PHASE_ID\}").expect("a regex");
    for rel in [
        "agents/gsd-phase-researcher.md",
        "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
    ] {
        let text = read(rel);
        assert!(
            text.contains("{phase_id}") && !upper.is_match(&text),
            "{rel}"
        );
    }
    let review = read("commands/gsd/ns-review.md");
    // the route table keeps upstream's skill names, which the router reads verbatim
    assert!(
        review.contains("absorbed by `/gsd-code-review --fix`") && !review.contains("flag of"),
        "{review}"
    );
}
