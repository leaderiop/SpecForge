//! How a core command ends (plan 02, ADR 0029): what it prints when its
//! operation refuses, and the exit code of its verdict.
//!
//! `refusals_after` is the table of every refusal as the binary writes it
//! (it was taken before the renderer as `refusals_today`, bugs included);
//! the ticket that changes a row re-blesses the snapshot, so its diff shows
//! each user-visible change. `verdicts` pins the exit codes of the
//! commands that judge.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

use crate::coverage_corpus::copy_tree;

/// A scratch copy of `fixtures/read_views/rv1`, optionally with a report.
fn rv1(report: Option<&str>) -> TempDir {
    let tmp = TempDir::new().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_views/rv1"),
        tmp.path(),
    );
    if let Some(report) = report {
        std::fs::write(tmp.path().join("specforge-report.json"), report).unwrap();
    }
    tmp
}

/// What one command printed.
struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_in(dir: &Path, args: &[&str], stdin: Option<&str>) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_specforge"))
        .args(args)
        .current_dir(dir)
        .env("HOME", std::env::temp_dir().join("specforge-outcome-home"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(text) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let canonical = dir.canonicalize().unwrap();
    let clean = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .replace(canonical.to_str().unwrap(), "[ROOT]")
            .replace(dir.to_str().unwrap(), "[ROOT]")
    };
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: clean(&out.stdout),
        stderr: clean(&out.stderr),
    }
}

/// One row of the table: `label | command`, then exit, stdout, stderr.
fn row(label: &str, args: &[&str], run: &Run) -> String {
    format!(
        "{label} | specforge {}\nexit {}\n-- stdout:\n{}-- stderr:\n{}\n",
        args.join(" "),
        run.code,
        run.stdout,
        run.stderr
    )
}

#[specforge_test_macros::test(
    behavior = "report_command_outcome",
    verify = "Report a Command's Outcome: command outcome holds — operation_ran, one_refusal_shape, one_exit_table, surfaces_agree"
)]
fn refusals_after() {
    let mut table = String::new();
    let mut add = |label: &str, dir: &Path, args: &[&str]| {
        let run = run_in(dir, args, None);
        table.push_str(&row(label, args, &run));
    };

    let broken = rv1(Some("{"));
    add("C1", broken.path(), &["stats"]);
    add("C2", broken.path(), &["stats", "--format", "json"]);
    add("C3", broken.path(), &["analyze", "--json"]);
    let project = rv1(None);
    add("C4", project.path(), &["analyze", "nope"]);
    add("C5", project.path(), &["trace", "nope", "--format", "json"]);
    add(
        "C6",
        project.path(),
        &["migrate", "--target-version", "9.9"],
    );
    add(
        "C7",
        project.path(),
        &["migrate", "--target-version", "9.9", "--format", "json"],
    );
    add("C8", project.path(), &["schema", "--kind", "behavor"]);
    add("C9", project.path(), &["export", "--scope", "nope"]);
    let empty = TempDir::new().unwrap();
    add(
        "C10",
        empty.path(),
        &["init", "--name", "a", "--format", "json"],
    );
    let loose = TempDir::new().unwrap();
    add(
        "C11",
        loose.path(),
        &["collect", "--path", ".", "--format", "json"],
    );
    add(
        "C12",
        project.path(),
        &["add", "@specforge/nope", "--format", "json"],
    );

    insta::assert_snapshot!("refusals_after", table);
}

const MESSY: &str =
    "behavior messy \"Messy\" {\ncategory \"core\"\ncontract \"The system MUST work\"\n}\n";
/// A region the formatter cannot parse and keeps as written (W142).
const REGION: &str = "behavior broken \"Broken\" {\n  @@@ ]]\n}\n";

#[test]
fn verdicts() {
    let exit = |dir: &Path, args: &[&str]| run_in(dir, args, None).code;

    let messy = rv1(None);
    std::fs::write(messy.path().join("spec/messy.spec"), MESSY).unwrap();
    let check = exit(messy.path(), &["format", "--check"]);
    let diff = exit(messy.path(), &["format", "--diff"]);
    let write = exit(messy.path(), &["format"]);

    let region = rv1(None);
    std::fs::write(region.path().join("spec/region.spec"), REGION).unwrap();
    let region_write = exit(region.path(), &["format"]);
    let region_stdin = run_in(region.path(), &["format", "--stdin"], Some(REGION)).code;

    let bad_header = rv1(None);
    std::fs::write(
        bad_header.path().join("spec/bad.spec"),
        "// specforge-format: 99.0\nbehavior bad \"Bad\" {\n}\n",
    )
    .unwrap();
    let migrate_fails = exit(bad_header.path(), &["migrate"]);

    let old = rv1(None);
    std::fs::write(
        old.path().join("spec/old.spec"),
        "// specforge-format: 0.1\nbehavior old_one \"Old\" {\n  category \"core\"\n  contract \"The system MUST work\"\n}\n",
    )
    .unwrap();
    let migrate_dry_run = exit(old.path(), &["migrate", "--dry-run"]);

    let dangling = rv1(None);
    std::fs::write(
        dangling.path().join("spec/dangling.spec"),
        "behavior dangling \"D\" {\n  category \"core\"\n  contract \"The system MUST work\"\n  invariants [missing]\n}\n",
    )
    .unwrap();
    let check_fails = exit(dangling.path(), &["check"]);

    for (what, got, expected) in [
        ("format --check, a change", check, 1),
        ("format --diff, a change", diff, 0),
        ("format, a change", write, 0),
        ("format, a W142 region", region_write, 1),
        ("format --stdin, a W142 region", region_stdin, 1),
        ("migrate, a 99.0 header", migrate_fails, 1),
        ("migrate --dry-run, a 0.1 header", migrate_dry_run, 0),
        ("check, an E003", check_fails, 1),
    ] {
        assert_eq!(got, expected, "{what}");
    }
}

/// The refusals that print the error document under JSON output, with the
/// code each carries: stdout holds one document, stderr nothing.
#[specforge_test_macros::test(
    behavior = "report_command_outcome",
    verify = "stats, trace, analyze, migrate and init refuse with the error document under --format json"
)]
fn json_refusals_are_the_error_document() {
    let broken = rv1(Some("{"));
    let project = rv1(None);
    let empty = TempDir::new().unwrap();
    let rows: [(&Path, &[&str], &str); 5] = [
        (broken.path(), &["stats", "--format", "json"], "E045"),
        (
            project.path(),
            &["trace", "nope", "--format", "json"],
            "E003",
        ),
        (broken.path(), &["analyze", "--json"], "E045"),
        (
            project.path(),
            &["migrate", "--target-version", "9.9", "--format", "json"],
            "E019",
        ),
        (
            empty.path(),
            &["init", "--name", "a", "--format", "json"],
            "invalid_name",
        ),
    ];
    for (dir, args, code) in rows {
        let run = run_in(dir, args, None);
        let document: serde_json::Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("{args:?}: stdout is no document ({e}): {}", run.stdout));
        assert_eq!(document["code"], code, "{args:?}: {document}");
        assert!(document["error"].is_string(), "{args:?}: {document}");
        assert_eq!(run.stderr, "", "{args:?}: nothing on stderr");
    }
}

/// `stats` and `analyze` measure: a refusal means they could not judge the
/// project, exit 2; every other command's refusal is exit 1.
#[specforge_test_macros::test(
    behavior = "report_command_outcome",
    verify = "a passed run exits 0, a failed verdict or a refusal 1, a refusal of a measuring command 2"
)]
fn measuring_commands_exit_two_on_refusal() {
    let broken = rv1(Some("{"));
    let project = rv1(None);

    for args in [&["stats"][..], &["analyze"], &["analyze", "nope"]] {
        let dir = if args == ["analyze", "nope"] {
            &project
        } else {
            &broken
        };
        assert_eq!(run_in(dir.path(), args, None).code, 2, "{args:?}");
    }
    for args in [&["trace", "nope"][..], &["schema", "--kind", "behavor"]] {
        assert_eq!(run_in(project.path(), args, None).code, 1, "{args:?}");
    }
    assert_eq!(run_in(project.path(), &["stats"], None).code, 0);
}

#[specforge_test_macros::test(
    behavior = "report_command_outcome",
    verify = "a command run outside any project refuses with no_project"
)]
fn collect_outside_a_project_is_no_project() {
    let loose = TempDir::new().unwrap();

    let run = run_in(
        loose.path(),
        &["collect", "--path", ".", "--format", "json"],
        None,
    );

    assert_eq!(run.code, 1);
    let document: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(document["code"], "no_project", "{document}");
    let human = run_in(loose.path(), &["collect", "--path", "."], None);
    assert!(
        human.stderr.starts_with("error[no_project]:"),
        "{}",
        human.stderr
    );
}
