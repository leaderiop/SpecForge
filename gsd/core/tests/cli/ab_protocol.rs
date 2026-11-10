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

/// `prompts/<arm>.md` split at its commands block (`<!-- commands -->` to `<!-- /commands -->`): the text around the
/// block, which every arm shares, and the block.
fn prompt(arm: &str) -> (String, String) {
    let path = ab_dir().join("prompts").join(format!("{arm}.md"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        text.matches("{task.prompt}").count(),
        1,
        "{arm}: the task's prompt goes in once"
    );
    let last = text
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();
    assert!(
        last.starts_with("End your final message with one line: `RESULT: "),
        "{arm}: the prompt ends with the RESULT instruction, not `{last}`"
    );
    let (before, rest) = text
        .split_once("<!-- commands -->\n")
        .unwrap_or_else(|| panic!("{arm}: no commands block"));
    let (commands, after) = rest
        .split_once("<!-- /commands -->\n")
        .unwrap_or_else(|| panic!("{arm}: the commands block is not closed"));
    assert!(
        !commands.trim().is_empty(),
        "{arm}: the commands block names the arm's tools"
    );
    (format!("{before}{after}"), commands.to_string())
}

/// Every arm of `tasks.json` has a prompt; the prompts differ only in their commands block, B0's is B's, and each
/// holds the task's prompt once and ends with the same RESULT instruction.
#[test]
fn the_arms_prompts_differ_only_in_their_commands() {
    let tasks = ab_tasks();
    let arms: Vec<&str> = tasks["arms"]
        .as_array()
        .expect("arms")
        .iter()
        .map(|a| a["id"].as_str().expect("an arm id"))
        .collect();
    assert_eq!(arms, ["A", "B", "B0"]);
    let (shared, a) = prompt("A");
    let (b_shared, b) = prompt("B");
    let (b0_shared, b0) = prompt("B0");
    assert_eq!(
        (&b_shared, &b0_shared),
        (&shared, &shared),
        "every arm's prompt is A's outside its commands"
    );
    assert_ne!(a, b, "A and B name their own tools");
    assert_eq!(
        b0, b,
        "B0 has B's tools: its difference is its configuration"
    );
    let mut files: Vec<String> = std::fs::read_dir(ab_dir().join("prompts"))
        .expect("prompts/")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    files.sort();
    assert_eq!(
        files,
        ["A.md", "B.md", "B0.md"],
        "one prompt per arm, nothing else"
    );
}
