//! A plan's paths, end to end: what the plan will create is checked once the plan is complete, what it deletes
//! while it is pending (ADR 0350).

use crate::harness::{Project, project};

/// `(field, value, status)` of each checked value of entity `id`, as `export --with-checks` lists them.
fn checks_of(p: &Project, id: &str) -> Vec<(String, String, String)> {
    let run = p.run(&["export", "--format", "graph", "--with-checks"]);
    let graph: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("export is JSON ({e}): {}", run.all()));
    let node = graph["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["id"] == id))
        .unwrap_or_else(|| panic!("'{id}' is exported"));
    let text =
        |row: &serde_json::Value, key: &str| row[key].as_str().unwrap_or_default().to_string();
    node["checks"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| (text(row, "field"), text(row, "value"), text(row, "status")))
                .collect()
        })
        .unwrap_or_default()
}

/// The statuses of `field`'s checked values on entity `id`.
fn statuses(p: &Project, id: &str, field: &str) -> Vec<String> {
    checks_of(p, id)
        .into_iter()
        .filter(|(f, _, _)| f == field)
        .map(|(_, _, status)| status)
        .collect()
}

/// The `E016` messages of a project's check.
fn missing(p: &Project) -> Vec<String> {
    p.checked()
        .into_iter()
        .filter(|(code, _)| code == "E016")
        .map(|(_, message)| message)
        .collect()
}

#[test]
fn a_complete_plans_missing_planned_file_is_e016() {
    let p = project("execute");
    assert_eq!(p.fields_of("v1.0/01/01-01")["status"], "complete");
    assert_eq!(p.codes(), Vec::<String>::new());
    std::fs::remove_file(p.root().join("src/tasks.js")).expect("the file");
    let found = missing(&p);
    // The plan's `files_modified`, its task's `files`, its artifact's `path` and its key link's `to`.
    assert_eq!(found.len(), 4, "{found:#?}");
    assert!(
        found.iter().all(|m| m.contains("src/tasks.js")),
        "{found:#?}"
    );
    assert_eq!(
        statuses(&p, "v1.0/01/01-01", "files_modified"),
        ["failed", "passed"]
    );
}

#[test]
fn a_pending_plans_planned_file_is_planned() {
    let p = project("plan");
    assert!(!p.exists("src/complete.js"));
    assert_ne!(p.fields_of("v1.0/01/01-02")["status"], "complete");
    assert_eq!(p.codes(), Vec::<String>::new());
    assert_eq!(statuses(&p, "v1.0/01/01-02", "files_modified"), ["planned"]);
    // The waiting values are counted once, in the note the harness leaves out of `codes()`.
    let notes: Vec<String> = p
        .checked()
        .into_iter()
        .filter(|(c, _)| c == "I213")
        .map(|(_, m)| m)
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("plan.files_modified"), "{notes:?}");
}

#[test]
fn a_pending_plans_deleted_file_must_exist() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "files_modified [\"src/complete.js\"]",
        "files_modified [\"src/complete.js\"]\n  files_deleted  [\"src/old.js\"]",
    );
    assert_eq!(p.codes(), ["E016"]);
    assert!(missing(&p)[0].contains("src/old.js"), "{:?}", missing(&p));
    p.write("src/old.js", "// to delete\n");
    assert_eq!(p.codes(), Vec::<String>::new());
    assert_eq!(statuses(&p, "v1.0/01/01-02", "files_deleted"), ["passed"]);
}

#[test]
fn a_complete_plans_deleted_file_is_closed() {
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]",
        "files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]\n  files_deleted  [\"src/old.js\"]",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    assert_eq!(statuses(&p, "v1.0/01/01-01", "files_deleted"), ["closed"]);
}
