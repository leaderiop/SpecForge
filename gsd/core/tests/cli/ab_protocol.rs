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

/// The scorer's self-test reads its synthetic runs back as files and pairs all of them, refuses an unpaired run, and
/// refuses exactly eight bad run files.
#[test]
fn the_scorer_pairs_twelve_runs_and_refuses_eight_run_files() {
    let run = python("score.py", &["--self-test"]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.lines().any(|l| l == "B - A: 12 pairs over 4 tasks"),
        "{stdout}"
    );
    let count = |prefix: &str| stdout.lines().filter(|l| l.starts_with(prefix)).count();
    assert_eq!(
        (count("refused: "), count("refused run file: ")),
        (1, 8),
        "{stdout}"
    );
}

/// Every acceptance row is a command to run or a rubric to judge by, never both, never a bare string.
#[test]
fn every_acceptance_row_is_run_or_judged() {
    let tasks = ab_tasks();
    for task in tasks["tasks"].as_array().expect("tasks") {
        for row in task["acceptance"].as_array().expect("acceptance") {
            let has = |key: &str| row[key].as_str().is_some_and(|s| !s.is_empty());
            assert!(
                has("text") && (has("run") != has("judge")),
                "{}: {row}",
                task["id"]
            );
        }
    }
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
fn run_files_are_ignored() {
    assert_eq!(run_files_ignored(), 0);
}

/// The corpus is frozen as v1: three ready fixtures whose paths exist, twelve tasks, each listed defect seeded in arm A
/// and arm B, each `b_signals` row about a seeded defect, and no word `pending` left in it.
#[test]
fn the_corpus_is_frozen_as_v1() {
    let tasks = ab_tasks();
    assert_eq!(tasks["version"], "gsd-ab-v1");
    let fixtures = tasks["fixtures"].as_array().expect("fixtures");
    assert_eq!(fixtures.len(), 3);
    for fixture in fixtures {
        assert_eq!(fixture["status"], "ready", "{fixture}");
        let path = fixture["path"].as_str().expect("a path");
        assert!(ab_dir().join("../..").join(path).is_dir(), "{path}");
    }
    let all = tasks["tasks"].as_array().expect("tasks");
    assert_eq!(all.len(), 12);
    for task in all {
        let id = task["id"].as_str().expect("an id");
        let seeded: Vec<&str> = task["seeded"]
            .as_array()
            .expect("seeded")
            .iter()
            .map(|d| d.as_str().expect("a defect"))
            .collect();
        for defect in &seeded {
            for arm in ["A", "B"] {
                let edits = task["seeds"][defect][arm].as_array();
                assert!(
                    edits.is_some_and(|e| !e.is_empty()),
                    "{id} {defect} has no {arm} edits"
                );
            }
        }
        for row in task["b_signals"].as_array().expect("b_signals") {
            let defect = row["defect"].as_str().expect("a defect");
            assert!(
                seeded.contains(&defect),
                "{id}: a signal for {defect}, which it does not seed"
            );
        }
    }
    let text = std::fs::read_to_string(ab_dir().join("tasks.json")).expect("tasks.json");
    assert_eq!(
        text.matches("pending").count(),
        0,
        "a frozen corpus has nothing pending"
    );
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
