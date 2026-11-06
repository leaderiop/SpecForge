//! `specforge gsd quick-init`: the host's nonce reaches the guest through the real binary.

use std::collections::BTreeSet;

use super::commands_support::{gsd, json};
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
        let run = gsd(&p, "quick-init", &["--pick", ".items[0].id"]);
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
    let row = run.stdout.trim_end().to_string();
    let cells: Vec<&str> = row.split('\u{1f}').collect();
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
    assert_eq!(answer["count"], 2);
}
