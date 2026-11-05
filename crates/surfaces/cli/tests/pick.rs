//! `--pick <path>` on `export`, `query`, `check` and a verb (ADR 0325): the values at a path of the command's
//! JSON answer, one per line or in a shape (D8), so a shell reads them without jq.

use crate::support::{Cli, TestProject};

const SPEC: &str = r#"
behavior alpha "Alpha" {
    contract "The system MUST do alpha" features [nope]
}
feature gamma "Gamma" {
    problem "Alpha is needed"
}
"#;

fn project() -> TestProject {
    TestProject::new()
        .without_config()
        .files(&[("main.spec", SPEC)])
}

#[test]
fn export_picks_the_graph_answer() {
    let dir = project();
    let cli = Cli::on(&dir);
    let ran = cli.run(&["export", "--pick", ".nodes[kind=feature].id"]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (0, "gamma\n"),
        "{}",
        ran.stderr
    );
    for format in ["graph", "json"] {
        let ran = cli.run(&[
            "export",
            "--format",
            format,
            "--pick",
            ".nodes[id=alpha].kind",
        ]);
        assert_eq!(
            (ran.code, ran.stdout.as_str()),
            (0, "behavior\n"),
            "{format}: {}",
            ran.stderr
        );
    }
    let ran = cli.run(&["export", "--pick", ".nodes[id=nope].kind"]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (0, ""),
        "a missing path passes"
    );
}

#[test]
fn export_pick_refuses_a_format_that_is_not_the_graph() {
    let dir = project();
    for format in ["context", "brief", "dot", "markdown"] {
        let ran = Cli::on(&dir).run(&["export", "--format", format, "--pick", ".nodes"]);
        assert_eq!(ran.code, 2, "{format}: {}", ran.stderr);
        assert_eq!(ran.stdout, "", "{format}");
        assert!(ran.stderr.contains("--pick"), "{format}: {}", ran.stderr);
    }
}

#[test]
fn check_picks_its_diagnostics_and_keeps_its_exit_code() {
    let dir = project();
    let cli = Cli::on(&dir);
    let ran = cli.run(&["check", "--pick", ".[code=E003].data.target"]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (1, "nope\n"),
        "{}",
        ran.stderr
    );
    let ran = cli.run(&["check", "--format", "json", "--pick", "[code=E003].code"]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (1, "E003\n"),
        "{}",
        ran.stderr
    );
    let ran = cli.run(&["check", "--pick", ".[code=gsd/W205].code"]);
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
}

#[test]
fn check_pick_refuses_an_explicit_human_format() {
    let dir = project();
    let ran = Cli::on(&dir).run(&["check", "--format", "human", "--pick", ".[0].code"]);
    assert_eq!(ran.code, 2, "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains("--pick"), "{}", ran.stderr);
}

/// The cells are joined by the ASCII unit separator (0x1f), which is not IFS whitespace, so
/// `IFS=$'\x1f' read -r` keeps an empty cell where a tab would collapse it, under macOS's bash 3.2 as under bash 5.
#[test]
fn fields_rows_keep_an_empty_cell_under_a_plain_read() {
    let dir = project();
    let cli = Cli::on(&dir);
    for command in ["export", "query"] {
        let mut args = vec![command];
        if command == "query" {
            args.extend(["gamma", "--depth", "1"]);
        }
        args.extend(["--pick", ".nodes[]", "--fields", "fields.missing,kind,id"]);
        let ran = cli.run(&args);
        assert_eq!(ran.code, 0, "{command}: {}", ran.stderr);
        // one row per node, a cell per key, and an empty cell where the value is missing
        let mut rows: Vec<&str> = ran.stdout.lines().collect();
        rows.sort_unstable();
        let want: &[&str] = if command == "export" {
            &["\u{1f}behavior\u{1f}alpha", "\u{1f}feature\u{1f}gamma"]
        } else {
            &["\u{1f}feature\u{1f}gamma"]
        };
        assert_eq!(rows, want, "{command}");
        for bash in ["/bin/bash", "bash"] {
            let read = format!(
                "while IFS=$'\\x1f' read -r a b c; do printf '[%s][%s][%s]\\n' \"$a\" \"$b\" \"$c\"; done <<'EOF'\n{}EOF",
                ran.stdout
            );
            let Ok(out) = std::process::Command::new(bash)
                .args(["-c", &read])
                .stdin(std::process::Stdio::null())
                .output()
            else {
                continue;
            };
            let mut cells: Vec<String> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::to_string)
                .collect();
            cells.sort_unstable();
            let want: Vec<String> = want
                .iter()
                .map(|row| format!("[{}]", row.replace('\u{1f}', "][")))
                .collect();
            assert_eq!(cells, want, "{command} under {bash}");
        }
    }
    let ran = cli.run(&["export", "--fields", "id"]);
    assert_eq!(ran.code, 2, "--fields needs --pick: {}", ran.stderr);
    let ran = cli.run(&["export", "--pick", ".nodes[]", "--fields", "a,,b"]);
    assert_eq!(ran.code, 2, "an empty key: {}", ran.stderr);
}

/// `--count`, `--first`, `--last` and `--join` shape what `--pick` prints (ADR 0325 D8).
#[test]
fn export_and_query_shape_what_they_pick() {
    let dir = project();
    let cli = Cli::on(&dir);
    for (shape, want) in [
        (&["--count"][..], "2\n"),
        (&["--first", "1"], "alpha\n"),
        (&["--last", "1"], "gamma\n"),
        (&["--join", ","], "alpha,gamma\n"),
        (&["--last", "5", "--join", ", "], "alpha, gamma\n"),
    ] {
        let mut args = vec!["export", "--pick", ".nodes[].id"];
        args.extend(shape);
        let ran = cli.run(&args);
        assert_eq!(
            (ran.code, ran.stdout.as_str()),
            (0, want),
            "{shape:?}: {}",
            ran.stderr
        );
    }
    let ran = cli.run(&["export", "--pick", ".nodes[id=nope]", "--count"]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (0, "0\n"),
        "{}",
        ran.stderr
    );
    let ran = cli.run(&[
        "query",
        "alpha",
        "--depth",
        "0",
        "--pick",
        ".nodes[].id",
        "--count",
    ]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (0, "1\n"),
        "{}",
        ran.stderr
    );
    let ran = cli.run(&[
        "export", "--pick", ".nodes[]", "--fields", "kind,id", "--first", "1",
    ]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (0, "behavior\u{1f}alpha\n"),
        "{}",
        ran.stderr
    );
}

/// A refused read answers nothing, so `$(… --count) || n='?'` tells it from 0.
#[test]
fn a_refused_read_prints_no_count() {
    let dir = project();
    let ran = Cli::on(&dir).run(&["query", "nope", "--pick", ".nodes", "--count"]);
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
}

#[test]
fn check_counts_its_diagnostics_and_keeps_its_exit_code() {
    let dir = project();
    let ran = Cli::on(&dir).run(&["check", "--pick", ".[code=E003]", "--count"]);
    assert_eq!(
        (ran.code, ran.stdout.as_str()),
        (1, "1\n"),
        "{}",
        ran.stderr
    );
}

#[test]
fn a_shape_is_a_usage_error_without_pick_or_beside_its_conflict() {
    let dir = project();
    let cli = Cli::on(&dir);
    for args in [
        &["export", "--count"][..],
        &["export", "--pick", ".nodes[].id", "--count", "--first", "1"],
        &["export", "--pick", ".nodes[].id", "--count", "--join", ","],
        &[
            "export",
            "--pick",
            ".nodes[].id",
            "--first",
            "1",
            "--last",
            "1",
        ],
        &["export", "--pick", ".nodes[].id", "--first", "-1"],
        &["export", "--pick", ".nodes[]", "--fields", "id", "--count"],
        &[
            "export", "--pick", ".nodes[]", "--fields", "id", "--join", ",",
        ],
        &["query", "alpha", "--join", ","],
        &["check", "--count"],
        &["settings", "--first", "1"],
        &["files", "list", "--last", "1"],
    ] {
        let ran = cli.run(args);
        assert_eq!(
            (ran.code, ran.stdout.as_str()),
            (2, ""),
            "{args:?}: {}",
            ran.stderr
        );
    }
}

#[test]
fn a_verb_shapes_its_answer() {
    let dir = project();
    let cli = Cli::on(&dir);
    let paths = cli.run(&["files", "list", "--pick", ".entries[].path"]);
    assert_eq!(paths.code, 0, "{}", paths.stderr);
    let count = cli.run(&["files", "list", "--pick", ".entries[].path", "--count"]);
    assert_eq!(
        (count.code, count.stdout.as_str()),
        (0, format!("{}\n", paths.stdout.lines().count()).as_str()),
        "{}",
        count.stderr
    );
    assert!(
        paths.stdout.lines().any(|path| path == "main.spec"),
        "{}",
        paths.stdout
    );
    let joined = cli.run(&["files", "list", "--pick", ".entries[].path", "--join", " "]);
    assert_eq!(
        joined.stdout,
        paths.stdout.lines().collect::<Vec<_>>().join(" ") + "\n"
    );
}
