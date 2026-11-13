//! `tests/common/answers.rs` both ways: the harness fails an answer that prints a member its row lacks, and this test
//! fails a row that lists a member no mode of its command prints. Each listed command runs in every mode its row is
//! the union of, and the members printed are exactly the row's.

use std::collections::BTreeSet;

use super::cmd_phase::{backlogged, twelve};
use super::harness::answer_check::answers::ANSWERS;
use super::harness::{Project, project, taskboard};

/// The taskboard, committed: the git-reading commands need a repository.
fn committed() -> Project {
    let p = taskboard();
    p.commit();
    p
}

/// The taskboard with `@gsd/security` beside `@gsd/core`.
fn secured() -> Project {
    taskboard().with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

fn workstreams() -> Project {
    project("workstreams")
}

/// A mode of a listed command: `(command, the project it runs on, its arguments)`.
type Mode = (&'static str, fn() -> Project, &'static [&'static str]);

/// Every mode of every listed command.
const MODES: &[Mode] = &[
    ("branch-check", committed, &[]),
    ("edge-probe", taskboard, &["v1.0/03"]),
    ("gate-command", taskboard, &["build"]),
    ("gate-command", taskboard, &["test"]),
    ("milestone-branches", committed, &["v1.0"]),
    ("phase", twelve, &["--name", "Sharing"]),
    ("phase", twelve, &["--insert", "02", "--name", "Urgent fix"]),
    ("phase", twelve, &["--edit", "03", "--goal", "x"]),
    ("phase", backlogged, &["--promote", "999.1"]),
    ("phase", twelve, &["--remove", "04"]),
    ("progress-report", taskboard, &[]),
    ("quick-init", taskboard, &["--description", "a"]),
    (
        "record-conflict",
        taskboard,
        &[
            "02-01",
            "--dimension",
            "d",
            "--property",
            "p",
            "--conflicts-with",
            "c",
        ],
    ),
    (
        "record-review",
        taskboard,
        &[
            "v1.0/02",
            "--review",
            r#"{"plans": ["v1.0/02/02-01"], "reviewers": [{"name": "codex", "model": "unknown", "source": "unknown"}], "lanes": [{"reviewer": "codex", "summary": "s", "risk": "low"}], "consensus": "c", "concerns": [{"plan": "v1.0/02/02-01", "reviewers": ["codex"], "severity": "low", "text": "t"}]}"#,
        ],
    ),
    ("security-hooks", secured, &["plan:pre"]),
    ("start-plan", taskboard, &["v1.0/02/02-01"]),
    (
        "user-story-validate",
        taskboard,
        &[
            "--story",
            "As a user, I want to add tasks, so that I remember them.",
        ],
    ),
    ("verification-debt", taskboard, &[]),
    ("workstreams", workstreams, &["list"]),
    ("workstreams", workstreams, &["progress"]),
    (
        "workstreams",
        workstreams,
        &["create", "--name", "billing", "--purpose", "invoices"],
    ),
    ("workstreams", workstreams, &["complete", "--name", "auth"]),
    ("workstreams", workstreams, &["switch", "--name", "auth"]),
    ("worktree-contains", committed, &["."]),
];

#[test]
fn every_listed_member_is_one_its_command_prints() {
    let listed: BTreeSet<&str> = ANSWERS.iter().map(|(command, _)| *command).collect();
    let run: BTreeSet<&str> = MODES.iter().map(|(command, _, _)| *command).collect();
    assert_eq!(listed, run, "every listed command runs in its modes here");
    for (command, row) in ANSWERS {
        let mut printed = BTreeSet::new();
        for (_, setup, args) in MODES.iter().filter(|(c, _, _)| c == command) {
            let p = setup();
            let mut all = vec!["gsd", command];
            all.extend_from_slice(args);
            // the answer a pick reads: a writing command's `output`, not the host's envelope
            all.extend(["--pick", "."]);
            let out = p.run(&all);
            let answer: serde_json::Value = serde_json::from_str(&out.stdout)
                .unwrap_or_else(|e| panic!("{command} {args:?} prints JSON ({e}): {}", out.all()));
            let object = answer
                .as_object()
                .unwrap_or_else(|| panic!("{command} {args:?}: not an object: {answer}"));
            printed.extend(object.keys().cloned());
        }
        let row: BTreeSet<String> = row.iter().map(|m| m.to_string()).collect();
        assert_eq!(printed, row, "{command}: the members its modes print");
    }
}
