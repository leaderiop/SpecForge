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
    assert_eq!(named, citing_tasks_js());
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

/// Truth TR1 of plan 01-01, whose text the spec edit changes.
const TR1: &str = "v1.0/01/01-01/TR1";

/// The truths that cite `src/tasks.js`.
fn citing_tasks_js() -> BTreeSet<String> {
    BTreeSet::from(
        [
            "v1.0/01/01-01/TR1",
            "v1.0/01/01-02/TR1",
            "v1.0/01/01-02/TR2",
        ]
        .map(String::from),
    )
}

/// The message of an advisory.
fn message(advisory: &Value) -> &str {
    advisory["message"].as_str().expect("a message")
}

#[test]
fn a_truth_text_edit_is_named_as_the_spec() {
    let p = taskboard();
    p.commit();
    p.edit(
        "spec/plans.spec",
        "text \"A task added to a board has the next integer id and keeps its title\"",
        "text \"A task added to a board has the next integer id and keeps its title exactly\"",
    );
    let advisories = p.advisories(I527, "HEAD");
    let named: BTreeSet<String> = advisories.keys().cloned().collect();
    assert_eq!(named, p.stale_truths("v1.0/01"));
    let on_tr1 = message(
        advisories
            .get(TR1)
            .unwrap_or_else(|| panic!("{I527} on {TR1}: {advisories:#?}")),
    );
    assert!(
        on_tr1.contains(&format!("the spec it reads changed: truth '{TR1}' (text)")),
        "{on_tr1}"
    );
    assert!(on_tr1.contains("no file it cites changed"), "{on_tr1}");
}

#[test]
fn a_verdict_recorded_again_before_an_edit_is_not_the_spec() {
    let p = taskboard();
    p.commit();
    let citing = p.citing(TR1);
    let extra: Vec<&str> = citing.iter().map(String::as_str).collect();
    let run = p.record(TR1, "E501", "verified_coincidental", &extra);
    assert_eq!(run.code, 0, "{}", run.all());
    touch(&p, "src/tasks.js");
    let nodes = delta_nodes(&p)["nodes"].clone();
    let node = nodes
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["id"] == TR1)
        .unwrap_or_else(|| panic!("{TR1} is in the delta: {nodes:#}"));
    assert_eq!(node["change"], "modified", "{node}");
    assert!(
        node["aspects"]
            .as_array()
            .is_some_and(|a| a.iter().any(|x| x == "verdicts")),
        "{node}"
    );
    assert_eq!(node["before"], node["after"], "{node}");
    let advisories = p.advisories(I527, "HEAD");
    let on_tr1 = message(
        advisories
            .get(TR1)
            .unwrap_or_else(|| panic!("{I527} on {TR1}: {advisories:#?}")),
    );
    assert!(
        on_tr1.contains("only the files it cites changed: 'src/tasks.js'"),
        "{on_tr1}"
    );
    assert!(!on_tr1.contains("the spec it reads changed"), "{on_tr1}");
}

#[test]
fn the_baseline_is_read_at_the_revision_since_names() {
    let p = taskboard();
    p.commit();
    let judged = p.head();
    touch(&p, "src/tasks.js");
    p.commit();
    let edited = p.head();
    let cases = [
        (
            judged.as_str(),
            &judged,
            "only the files it cites changed: 'src/tasks.js'",
        ),
        (
            "HEAD",
            &edited,
            "neither the spec it reads nor a file it cites changed",
        ),
    ];
    for (revision, commit, cause) in cases {
        let advisories = p.advisories(I527, revision);
        let named: BTreeSet<String> = advisories.keys().cloned().collect();
        assert_eq!(named, citing_tasks_js(), "since {revision}");
        let since = format!("since {}", &commit[..12]);
        for (truth, advisory) in &advisories {
            let text = message(advisory);
            assert!(text.contains(&since), "{truth} since {revision}: {text}");
            assert!(text.contains(cause), "{truth} since {revision}: {text}");
        }
    }
}
