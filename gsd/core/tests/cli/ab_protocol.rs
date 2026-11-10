//! The GSD A/B protocol (`evaluation/gsd-ab/`) as files: its scripts' self-tests pass, and what it says about
//! itself holds.

use std::process::{Command, Stdio};

use super::harness::ab::{ab_dir, ab_tasks, python};

fn passes(script: &str) {
    let run = python(script, &["--self-test"]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success() && stdout.lines().any(|l| l == "self-test: ok"),
        "{script} --self-test: {stdout}{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn the_scorer_passes_its_self_test() {
    passes("score.py");
}

#[test]
fn the_seeding_script_passes_its_self_test() {
    passes("seed.py");
}

/// `git check-ignore`'s exit code for a run file: 0 when it is ignored.
fn run_files_ignored() -> i32 {
    Command::new("git")
        .args(["check-ignore", "-q", "evaluation/gsd-ab/runs/A/t/1.json"])
        .current_dir(ab_dir().join("../.."))
        .stdin(Stdio::null())
        .status()
        .expect("run git")
        .code()
        .unwrap_or(-1)
}

#[test]
fn run_files_are_not_ignored_yet() {
    assert_eq!(run_files_ignored(), 1);
}

#[test]
fn no_task_is_a_placeholder() {
    let tasks = ab_tasks();
    let placeholders = tasks["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .filter(|t| t.get("status").is_some())
        .count();
    assert_eq!(placeholders, 0);
}
