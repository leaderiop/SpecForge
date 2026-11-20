//! A check since a commit says what changed under each stale truth (`gsd/I527`), on the taskboard twin, and names
//! exactly the truths `re-verify --stale-only` lists: the stale set is the verifier's judgements, and the graph delta
//! and the judgement at the base say why.

use std::collections::BTreeSet;

use serde_json::Value;

use super::harness::{Project, taskboard};

const I527: &str = "gsd/I527";

/// Appends one comment line to a file of the project.
fn touch(p: &Project, file: &str) {
    let text = p.read(file);
    assert!(text.ends_with('\n'), "{file} ends with a line break");
    p.write(file, &format!("{text}// edited\n"));
}

/// `{nodes, edges}` of `specforge diff --kind truth --kind plan --kind phase`: what the delta of the pass holds since
/// `HEAD`.
fn delta_nodes(p: &Project) -> Value {
    let run = p.run(&[
        "diff", "--kind", "truth", "--kind", "plan", "--kind", "phase", "--format", "json",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let diff: Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("diff prints JSON ({e}): {}", run.all()));
    serde_json::json!({ "nodes": diff["nodes"], "edges": diff["edges"] })
}

/// The codes of a check since `revision`.
fn codes_since(p: &Project, revision: &str) -> Vec<String> {
    p.checked_since(revision)
        .iter()
        .filter_map(|d| d["code"].as_str().map(String::from))
        .collect()
}

#[test]
fn an_edit_to_a_cited_file_names_the_truths_re_verify_lists() {
    let p = taskboard();
    p.commit();
    let base: String = p.head().chars().take(12).collect();
    touch(&p, "src/tasks.js");
    // A one-file edit is not in the delta: files are not graph nodes.
    assert_eq!(
        delta_nodes(&p),
        serde_json::json!({ "nodes": [], "edges": [] })
    );
    let codes = codes_since(&p, "HEAD");
    assert!(!codes.iter().any(|c| c == "W021"), "{codes:?}");
    let advisories = p.advisories(I527, "HEAD");
    let named: BTreeSet<String> = advisories.keys().cloned().collect();
    assert_eq!(
        named,
        BTreeSet::from(
            [
                "v1.0/01/01-01/TR1",
                "v1.0/01/01-02/TR1",
                "v1.0/01/01-02/TR2"
            ]
            .map(String::from)
        )
    );
    assert_eq!(named, p.stale_truths("v1.0/01"));
    for (truth, advisory) in &advisories {
        assert_eq!(advisory["severity"], "Info", "{truth}: {advisory}");
        let message = advisory["message"].as_str().expect("a message");
        assert!(message.contains(&format!("since {base}")), "{message}");
        assert!(
            message.contains("only the files it cites changed: 'src/tasks.js'"),
            "{message}"
        );
        assert!(!message.contains("test/tasks.test.js"), "{message}");
    }
}

#[test]
fn the_unedited_twin_names_nothing_since_its_commit() {
    let p = taskboard();
    p.commit();
    assert_eq!(
        delta_nodes(&p),
        serde_json::json!({ "nodes": [], "edges": [] })
    );
    let codes = codes_since(&p, "HEAD");
    assert!(!codes.iter().any(|c| c == I527 || c == "W021"), "{codes:?}");
}
