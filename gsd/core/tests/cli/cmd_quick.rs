//! `specforge gsd quick-init`: the host's nonce reaches the guest through the real binary.

use std::collections::BTreeSet;

use super::commands_support::{gsd, json, rows};
use crate::harness::project;

#[test]
fn quick_init_mints_a_fresh_free_id_per_call() {
    let p = project("execute");
    let taken: BTreeSet<String> = p
        .run(&["export", "--format", "graph", "--pick", ".nodes[].id"])
        .stdout
        .lines()
        .map(str::to_string)
        .collect();
    let id = || {
        let run = gsd(
            &p,
            "quick-init",
            &["--description=a task", "--pick", ".items[0].id"],
        );
        assert_eq!(run.code, 0, "{}", run.all());
        run.stdout.trim_end().to_string()
    };
    let (first, second) = (id(), id());
    assert_ne!(first, second);
    for id in [&first, &second] {
        assert!(!taken.contains(id), "{id} is a node of the graph");
        assert_eq!(id.len(), 10, "{id}");
    }
    // One call's values are read in one line, and its slug and branch agree with its id.
    let run = gsd(
        &p,
        "quick-init",
        &[
            "--description",
            "Fix login",
            "--template",
            "quick/{num}-{slug}",
            "--pick",
            ".items[0]",
            "--fields",
            "id,slug,branch_name",
        ],
    );
    let cells = &rows(&run.stdout)[0];
    assert_eq!(
        cells[1..],
        ["fix-login", &format!("quick/{}-fix-login", cells[0])],
        "{}",
        run.all()
    );
    // A human reads one line per item. A list starts with a hyphen, so its value is attached with `=`.
    let run = gsd(&p, "quick-init", &["--batch", "--tasks=- a\n- b"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        run.stdout.lines().all(|line| line.contains('\t')),
        "{}",
        run.stdout
    );
    assert_eq!(run.stdout.lines().count(), 2, "{}", run.stdout);
    let refused = gsd(&p, "quick-init", &["--batch", "--tasks=- a"]);
    assert_eq!(refused.code, 1, "{}", refused.all());
    let answer = json(&gsd(
        &p,
        "quick-init",
        &["--batch", "--tasks=- a\n- b", "--format", "json"],
    ));
    assert_eq!(answer["items"][1]["count"], 2);
    // The command declares no `--count` of its own, so the host's shape counts what `--pick` reaches.
    let counted = gsd(
        &p,
        "quick-init",
        &[
            "--batch",
            "--tasks=- a\n- b",
            "--pick",
            ".items[]",
            "--count",
        ],
    );
    assert_eq!(
        (counted.code, counted.stdout.trim()),
        (0, "2"),
        "{}",
        counted.all()
    );
}

/// Two quick plans of the twin, as `/gsd-quick-batch` leaves them before it records the batch.
fn quick_plans(p: &crate::harness::Project) {
    super::commands_support::plant(
        p,
        "spec/quick.spec",
        "plan 261112-abc \"Trim titles\" {\n  type           execute\n  files_modified []\n  objective      \"Trim titles\"\n}\n\nplan 261112-def \"Sort tasks\" {\n  type           execute\n  files_modified []\n  objective      \"Sort tasks\"\n}\n",
    );
}

const BASE: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn a_batch_is_recorded_with_its_items() {
    let p = super::commands_support::twin();
    quick_plans(&p);
    let batch_id = || {
        let run = gsd(
            &p,
            "quick-init",
            &[
                "--batch",
                "--tasks=- a
- b",
                "--pick",
                ".items[0].batch",
            ],
        );
        assert_eq!(run.code, 0, "{}", run.all());
        run.stdout.trim_end().to_string()
    };
    let id = batch_id();
    assert!(id.ends_with("-batch"), "{id}");
    let run = gsd(
        &p,
        "start-batch",
        &[
            &id,
            "--items",
            r#"["261112-abc", "261112-def"]"#,
            "--base",
            BASE,
            "--format",
            "json",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(json(&run)["output"]["batch"], id.as_str());
    let batch = p.fields_of(&id);
    assert_eq!(
        batch["items"],
        serde_json::json!(["261112-abc", "261112-def"])
    );
    assert_eq!(batch["base"], BASE);
    assert_eq!(batch["status"], "open");
    assert_eq!(
        batch["eligible"],
        serde_json::json!(["261112-abc", "261112-def"])
    );
    assert!(
        batch["created"].as_str().is_some_and(|t| t.ends_with('Z')),
        "{batch}"
    );
    // the day's id is taken now: quick-init answers the next one
    assert_eq!(batch_id(), format!("{id}-2"));
}

#[test]
fn a_taken_batch_id_or_a_phase_plan_is_refused() {
    let p = super::commands_support::twin();
    quick_plans(&p);
    let items = r#"["261112-abc", "261112-def"]"#;
    assert_eq!(
        gsd(
            &p,
            "start-batch",
            &["261112-batch", "--items", items, "--base", BASE]
        )
        .code,
        0
    );
    let before = super::commands_support::bytes(&p);
    let refused = super::commands_support::refused;
    refused(
        &gsd(
            &p,
            "start-batch",
            &["261112-batch", "--items", items, "--base", BASE],
        ),
        "invalid_input",
        &["261112-batch", "taken"],
    );
    refused(
        &gsd(
            &p,
            "start-batch",
            &[
                "261112-batch-2",
                "--items",
                r#"["261112-abc", "01-01"]"#,
                "--base",
                BASE,
            ],
        ),
        "invalid_input",
        &["01-01", "phase"],
    );
    refused(
        &gsd(
            &p,
            "start-batch",
            &[
                "261112-batch-2",
                "--items",
                r#"["261112-abc", "261112-abc"]"#,
                "--base",
                BASE,
            ],
        ),
        "invalid_input",
        &["twice"],
    );
    refused(
        &gsd(
            &p,
            "start-batch",
            &["261112-batch-2", "--items", items, "--base", "HEAD"],
        ),
        "invalid_input",
        &["HEAD", "40"],
    );
    refused(
        &gsd(
            &p,
            "start-batch",
            &[
                "261112-batch-2",
                "--items",
                r#"["261112-abc"]"#,
                "--base",
                BASE,
            ],
        ),
        "invalid_input",
        &["at least 2"],
    );
    super::commands_support::unchanged(&p, &before);
}
