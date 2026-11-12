//! The taskboard twin's source as item facts: `@javascript/source` (a [`TWIN_BUILTINS`] builtin) has the host's reader
//! parse `src/` and answers an item per definition the twin's modules hold, named after the package `taskboard`, and
//! the package itself; the test file is never read. Nothing cites an item yet, so they add no finding.
//!
//! [`TWIN_BUILTINS`]: super::harness::TWIN_BUILTINS

use serde_json::{Value, json};

use super::harness::{Project, taskboard};

/// `(id, kind, visibility, first line)` of every item, by id.
fn items(p: &Project) -> Vec<(String, Value, String, u64)> {
    let mut found: Vec<(String, Value, String, u64)> = p
        .nodes_of("item")
        .iter()
        .map(|n| {
            (
                n["id"].as_str().expect("an id").to_string(),
                n["fields"]["kind"].clone(),
                n["fields"]["visibility"]
                    .as_str()
                    .expect("a visibility")
                    .to_string(),
                n["located"][0]["start_line"].as_u64().expect("a place"),
            )
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[test]
fn the_twins_sources_collect_as_item_facts() {
    let p = taskboard();
    let run = p.collect(&["--runner", "javascript-source"]);
    assert_eq!(run.code, 0, "collect: {}", run.all());

    let item = |id: &str, kind: &str, visibility: &str, line: u64| {
        (id.to_string(), json!([kind]), visibility.to_string(), line)
    };
    assert_eq!(
        items(&p),
        [
            item("js:taskboard/src/cli.js::board", "const", "private", 6),
            item(
                "js:taskboard/src/tasks.js::addTask",
                "function",
                "public",
                7
            ),
            item(
                "js:taskboard/src/tasks.js::completeTask",
                "function",
                "public",
                14
            ),
            item(
                "js:taskboard/src/tasks.js::createBoard",
                "function",
                "public",
                3
            ),
        ],
        "nothing from test/tasks.test.js"
    );
    let packages: Vec<String> = p
        .nodes_of("package")
        .iter()
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect();
    assert_eq!(packages, ["pkg:npm/taskboard"]);
}

#[test]
fn the_twins_items_add_no_finding() {
    let p = taskboard();
    let golden = p.collect_node_test_golden();
    assert_eq!(golden.code, 0, "collect: {}", golden.all());
    let run = p.collect(&["--runner", "javascript-source"]);
    assert_eq!(run.code, 0, "collect: {}", run.all());
    p.check_exact(&[]);
}
