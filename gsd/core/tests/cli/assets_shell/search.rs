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

/// A search that could not read its file is neither of the two answers its caller reads: a missing page is not an
/// unprotected one, as `$(… -q … && echo yes)` made it.
#[test]
fn a_verdict_keeps_could_not_read_apart_from_no() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = sample();
    std::fs::write(p.path().join("src/guarded.tsx"), "useAuth();\n").expect("a page");
    std::fs::write(p.path().join("src/open.tsx"), "<a/>\n").expect("a page");
    let check = function(&super::asset(CHECKER), "check_auth_protection() {");
    for (page, printed) in [
        ("src/guarded.tsx", "PROTECTED\n"),
        ("src/open.tsx", "UNPROTECTED\n"),
        ("src/nosuch.tsx", "COULD NOT READ: src/nosuch.tsx\n"),
    ] {
        let (out, err, _) = sh.run_in(p.path(), &format!("{check}\ncheck_auth_protection {page}"));
        assert_eq!(out, printed, "{page}\n{err}");
    }
    let wiring = function(&super::asset(PATTERNS), "check_wiring() {");
    for (file, printed) in [
        ("src/a.ts", "WIRED: src/a.ts → API_KEY\n"),
        ("src/open.tsx", "NOT_WIRED: src/open.tsx → API_KEY\n"),
        ("src/nosuch.tsx", "COULD NOT READ: src/nosuch.tsx\n"),
    ] {
        let (out, err, _) = sh.run_in(p.path(), &format!("{wiring}\ncheck_wiring {file} API_KEY"));
        assert_eq!(out, printed, "{file}\n{err}");
    }
    let substantive = function(&super::asset(PATTERNS), "check_substantive() {");
    let (out, err, _) = sh.run_in(
        p.path(),
        &format!("{substantive}\ncheck_substantive src/nosuch.tsx 1 x"),
    );
    assert_eq!(out, "COULD NOT READ: src/nosuch.tsx\n", "{err}");
}

/// A tally that exists and cannot be read stops the step: it is not the same as no failures.
#[test]
fn the_failure_tally_reads_zero_only_when_it_is_absent() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = crate::harness::taskboard();
    let read = handoff_block("gsd-core/workflows/execute-phase.md", "wave-failure-read");
    let (out, err, code) = sh.run_in(p.root(), &read);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("WAVE_FAILURE_COUNT=0"), "{out}");
    let (made, _, _) = sh.run_in(
        p.root(),
        "specforge files temp --dir --name gsd-handoff --pick .path",
    );
    let tally = std::path::Path::new(made.trim()).join("wave-failures");
    std::fs::write(&tally, "1\n").expect("a tally");
    std::fs::hard_link(&tally, p.root().join("second-name")).expect("a second name");
    let (out, _, code) = sh.run_in(p.root(), &read);
    assert_ne!(code, 0, "an unread tally passed: {out}");
}

const WIRING: &str = "gsd-core/references/verifier-wiring-patterns.md";

/// Each proximity line against what `grep -A N X | grep Y` found: Y on X's own line in either order, Y N lines after
/// X, and nothing at N + 1.
#[test]
fn a_proximity_line_finds_y_before_x_and_within_n_lines() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    // (asset, the line's own text, its variables, X, Y, N)
    let table = [
        (
            PATTERNS,
            r"(.*\n){0,20}?.*(Int|",
            "model_name=User schema_path=f.tsx",
            "model User {",
            "  n Int",
            20,
        ),
        (
            PATTERNS,
            r"$component_path",
            "component_path=f.tsx",
            "<form onSubmit={h}>",
            "fetch(u)",
            10,
        ),
        (
            WIRING,
            r"(.*\n){0,5}?.*(fetch|axios|query",
            "STATE_VAR=Data artifact=f.tsx",
            "setData(x)",
            "store.get()",
            5,
        ),
        (
            WIRING,
            r"(.*\n){0,3}?.*=\{",
            "COMPONENT_NAME=Card search_path=.",
            "<Card",
            "a={[]}",
            3,
        ),
        (
            WIRING,
            r"(.*\n){0,5}?.*(await",
            "component=f.tsx",
            "fetch('/x')",
            "await y",
            5,
        ),
        (
            WIRING,
            r#"dispatch)' "$component""#,
            "component=f.tsx",
            "<form onSubmit={h}>",
            "fetch(u)",
            10,
        ),
    ];
    let p = tempfile::tempdir().expect("a project");
    for (asset, needle, vars, x, y, n) in table {
        let search = line_with(
            &super::asset(asset)
                .lines()
                .filter(|line| line.starts_with("specforge files search -U"))
                .collect::<Vec<_>>()
                .join("\n"),
            needle,
        );
        let found = |text: String| {
            std::fs::write(p.path().join("f.tsx"), text).expect("a file");
            let (out, err, _) = sh.run_in(p.path(), &format!("{vars}\n{search}"));
            assert!(err.is_empty(), "{search}\n{err}");
            !out.is_empty()
        };
        let lines = |gap: usize| format!("{x}\n{}{y}\n", "-\n".repeat(gap));
        assert!(found(format!("{y} {x}\n")), "Y before X: {search}");
        assert!(found(format!("{x} {y}\n")), "Y after X: {search}");
        assert!(found(lines(n - 1)), "Y {n} lines after X: {search}");
        assert!(!found(lines(n)), "Y {} lines after X: {search}", n + 1);
    }
    // A line that is not X nor Y's own: the schema's field lines keep their first line after the model's.
    let check = line_with(&super::asset(PATTERNS), r"{1,20}?[ \t]+\w+");
    std::fs::write(p.path().join("f.tsx"), "model User {\n  id Int\n}\n").expect("a schema");
    let (out, err, _) = sh.run_in(
        p.path(),
        &format!("model_name=User schema_path=f.tsx\n{check}"),
    );
    assert!(!out.is_empty(), "{check}\n{err}");
}

/// The hook wiring check names the hook's file by its path below `src`: its own file is left out, a barrel of the same
/// name is not, and with no path the check stops rather than exclude everything.
#[test]
fn the_hook_check_leaves_out_only_the_hooks_own_file() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = tempfile::tempdir().expect("a project");
    let write = |path: &str, text: &str| {
        let at = p.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    };
    write("src/hooks/index.ts", "import { useX } from './useX';\n");
    write(
        "src/hooks/useX.ts",
        "import { useX } from 'self';\nuseX();\n",
    );
    write("src/index.ts", "import { useX } from './hooks';\nuseX();\n");
    let text = super::asset(PATTERNS);
    let imported = line_with(&text, "\"import.*$hook_name\"");
    let called = line_with(&text, "\"$hook_name\\(\\)\"");
    let (out, err, _) = sh.run_in(
        p.path(),
        &format!("hook_name=useX hook_file=hooks/useX.ts\n{imported}"),
    );
    assert!(
        out.contains("src/hooks/index.ts:1:"),
        "{imported}\n{out}\n{err}"
    );
    assert!(out.contains("src/index.ts:1:"), "{imported}\n{out}\n{err}");
    assert!(!out.contains("useX.ts"), "{imported}\n{out}");
    let (out, err, _) = sh.run_in(
        p.path(),
        &format!("hook_name=useX hook_file=hooks/useX.ts\n{called}"),
    );
    assert!(out.contains("src/index.ts:2:"), "{called}\n{out}\n{err}");
    assert!(!out.contains("useX.ts"), "{called}\n{out}");
    for line in [&imported, &called] {
        let (out, _, code) = sh.run_in(p.path(), &format!("hook_name=useX\n{line}"));
        assert_ne!(code, 0, "no path, no check: {line}\n{out}");
        assert_eq!(out, "", "no path, no check: {line}");
    }
}

/// The lines of the `# gsd-handoff:begin <name>` block of `path`.
fn handoff_block(path: &str, name: &str) -> String {
    let text = super::asset(path);
    super::block(
        &text,
        &format!("# gsd-handoff:begin {name}"),
        &format!("# gsd-handoff:end {name}"),
    )
}

/// Fenced blocks are separate shells: each opens the same hand-off directory, so a count recorded in one and a file
/// list written in one reach the block that reads them, and a reset clears what an earlier run left.
#[test]
fn the_handoff_blocks_share_one_directory() {
    let Some(sh) = Shell::only(super::PICK_TOOLS) else {
        return;
    };
    let p = crate::harness::taskboard();
    let run = |script: String| {
        let (out, err, code) = sh.run_in(p.root(), &script);
        assert_eq!(code, 0, "{script}\n{err}");
        out
    };
    const PHASE: &str = "gsd-core/workflows/execute-phase.md";
    const GATE: &str = "gsd-core/workflows/execute-phase/steps/post-merge-gate.md";
    let read = || run(handoff_block(PHASE, "wave-failure-read"));
    assert!(read().contains("WAVE_FAILURE_COUNT=0"), "no tally yet");
    run(handoff_block(PHASE, "wave-failure-reset"));
    assert!(read().contains("WAVE_FAILURE_COUNT=0"), "an empty tally");
    run(handoff_block(GATE, "wave-failure-record"));
    run(handoff_block(GATE, "wave-failure-record"));
    assert!(read().contains("WAVE_FAILURE_COUNT=2"), "{}", read());
    run(handoff_block(PHASE, "wave-failure-reset"));
    assert!(read().contains("WAVE_FAILURE_COUNT=0"), "reset");

    const LIST: &str = "gsd-core/workflows/execute-phase/steps/regression-gate.md";
    const GATE_RUN: &str = "gsd-core/workflows/execute-phase/steps/regression-gate-run.md";
    let listed = || {
        run(format!(
            "{}\nprintf '%s' \"$REGRESSION_FILES\"",
            handoff_block(GATE_RUN, "regression-files-read")
        ))
    };
    run(handoff_block(LIST, "regression-files-write"));
    assert_eq!(
        listed(),
        "tests/test_example_one.py tests/test_example_two.py "
    );
    run(handoff_block(LIST, "regression-files-reset"));
    assert_eq!(listed(), "", "a reset list is gone");
}
