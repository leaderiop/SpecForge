use serde_json::{Value, json};

use super::*;

const PHASE: &str = "v1.0/01";
const PLAN: &str = "v1.0/01/01-01";
const TR1: &str = "v1.0/01/01-01/TR1";
/// The commit the base names.
const R: &str = "0123456789abcdef0123456789abcdef01234567";

/// A `gsd/E501` judgement in `state` with the inputs `changed` and the `(path, digest)` files it read.
fn verdict(state: &str, changed: &[&str], files: &[(&str, &str)]) -> Value {
    verdict_of("gsd/E501", state, changed, files)
}

fn verdict_of(rule: &str, state: &str, changed: &[&str], files: &[(&str, &str)]) -> Value {
    let files: Vec<Value> = files
        .iter()
        .map(|(path, digest)| json!({ "path": path, "digest": digest }))
        .collect();
    json!({
        "rule": rule, "state": state, "passed": state == "fresh", "outcome": "verified",
        "at": "2026-11-20T10:00:00Z", "changed": changed, "files": files,
    })
}

/// The stale verdict on TR1 after `src/tasks.js` moved from `A` to `C`; `test/tasks.test.js` is still `B`.
fn stale_after_an_edit() -> Value {
    verdict(
        "stale",
        &["cited"],
        &[("src/tasks.js", "C"), ("test/tasks.test.js", "B")],
    )
}

/// TR1's verdict at the base: fresh over `src/tasks.js` at `A` and `test/tasks.test.js` at `B`.
fn fresh_at_the_base() -> Value {
    verdict(
        "fresh",
        &[],
        &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
    )
}

/// A phase, its plan and the plan's truth TR1 holding `judgements`, with `extra` keys (`delta`, `previous`) on the
/// input.
fn input(judgements: Vec<Value>, extra: Value) -> PassInput {
    let mut input = json!({
        "entities": [
            { "id": PHASE, "kind": "phase" },
            { "id": PLAN, "kind": "plan" },
            { "id": TR1, "kind": "truth", "judgements": judgements },
        ],
        "edges": [
            { "source": PLAN, "target": PHASE, "label": "phase" },
            { "source": TR1, "target": PLAN, "label": "plan" },
        ],
    });
    for (key, value) in extra.as_object().expect("an object") {
        input[key] = value.clone();
    }
    serde_json::from_value(input).expect("a pass input")
}

fn revision() -> Value {
    json!({ "kind": "revision", "id": R })
}

/// A delta from `base` with `nodes`.
fn delta(base: Value, nodes: Vec<Value>) -> Value {
    json!({ "base": base, "nodes": nodes })
}

/// The baseline read at `revision`, TR1 holding `judgement` there.
fn baseline(revision: &str, judgement: Value) -> Value {
    json!({
        "revision": revision,
        "entities": { TR1: { "kind": "truth", "judgements": [judgement] } },
    })
}

/// A node `modified` with these fields and aspects and two different content digests.
fn modified(kind: &str, id: &str, fields: &[&str], aspects: &[&str]) -> Value {
    json!({
        "kind": kind, "id": id, "change": "modified", "fields": fields, "aspects": aspects,
        "before": "d0", "after": "d1",
    })
}

/// A check since `R` with these delta nodes, TR1 stale after the edit and fresh at the base.
fn since_r(nodes: Vec<Value>, now: Value) -> PassInput {
    input(
        vec![now],
        json!({ "delta": delta(revision(), nodes), "previous": baseline(R, fresh_at_the_base()) }),
    )
}

/// The I527 on TR1, its code and severity checked.
fn on_tr1(input: &PassInput) -> PassDiagnostic {
    let found = check(input);
    let advisory = found
        .iter()
        .find(|d| d.entity.as_deref() == Some(TR1))
        .unwrap_or_else(|| panic!("an advisory on {TR1}: {found:?}"))
        .clone();
    assert_eq!(advisory.code, "I527");
    assert_eq!(advisory.severity, PassSeverity::Info);
    assert_eq!(found.len(), 1, "{found:?}");
    advisory
}

#[test]
fn no_stale_truth_says_nothing() {
    let input = input(
        vec![fresh_at_the_base()],
        json!({ "delta": delta(revision(), vec![]) }),
    );
    assert!(check(&input).is_empty());
}

#[test]
fn a_cold_check_says_nothing_even_over_a_stale_truth() {
    let added = json!({ "kind": "truth", "id": TR1, "change": "added", "after": "d1" });
    let input = input(
        vec![stale_after_an_edit()],
        json!({ "delta": delta(json!({ "kind": "none" }), vec![added]) }),
    );
    assert!(check(&input).is_empty());
}

#[test]
fn a_pass_input_without_a_delta_says_nothing() {
    let input = input(vec![stale_after_an_edit()], json!({}));
    assert!(check(&input).is_empty());
}

#[test]
fn only_a_stale_verifier_verdict_on_a_truth_is_named() {
    let mut input = input(
        vec![verdict("fresh", &[], &[("src/tasks.js", "C")])],
        json!({ "delta": delta(revision(), vec![]) }),
    );
    let stale_plan_check = serde_json::from_value(verdict_of("gsd/W311", "stale", &["self"], &[]))
        .expect("a judgement");
    let stale_artifact =
        serde_json::from_value(verdict("stale", &["file"], &[("src/tasks.js", "C")]))
            .expect("a judgement");
    input
        .entities
        .iter_mut()
        .find(|e| e.id == PLAN)
        .expect("the plan")
        .judgements
        .push(stale_plan_check);
    input.entities.push(PassEntity {
        id: "v1.0/01/01-01/AR1".into(),
        kind: "artifact".into(),
        judgements: vec![stale_artifact],
        ..Default::default()
    });
    assert!(check(&input).is_empty());
}

#[test]
fn a_cited_file_changed_since_the_revision_is_named_alone() {
    let advisory = on_tr1(&since_r(vec![], stale_after_an_edit()));
    let message = &advisory.message;
    assert!(
        message.contains(&format!("since {}", &R[..12])),
        "{message}"
    );
    assert!(
        message.contains("only the files it cites changed: 'src/tasks.js'"),
        "{message}"
    );
    assert!(!message.contains("test/tasks.test.js"), "{message}");
    assert!(
        message.contains("(cited changed since it was recorded)"),
        "{message}"
    );
}

#[test]
fn nothing_changed_since_the_revision_is_said() {
    let now = verdict(
        "stale",
        &["tests"],
        &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
    );
    let advisory = on_tr1(&since_r(vec![], now));
    assert!(
        advisory
            .message
            .contains("neither the spec it reads nor a file it cites changed"),
        "{}",
        advisory.message
    );
}

#[test]
fn a_revision_the_baseline_was_not_read_at_claims_no_file() {
    let other = "fedcba9876543210fedcba9876543210fedcba98";
    let input = input(
        vec![stale_after_an_edit()],
        json!({ "delta": delta(revision(), vec![]), "previous": baseline(other, fresh_at_the_base()) }),
    );
    let message = on_tr1(&input).message;
    assert!(
        message.contains(
            "the spec it reads is unchanged; it cites 'src/tasks.js', 'test/tasks.test.js'"
        ),
        "{message}"
    );
    assert!(!message.contains("only the files"), "{message}");
}

#[test]
fn the_last_check_names_the_cited_files_without_claiming_them() {
    let input = input(
        vec![stale_after_an_edit()],
        json!({
            "delta": delta(json!({ "kind": "previous" }), vec![]),
            "previous": baseline(R, fresh_at_the_base()),
        }),
    );
    let message = on_tr1(&input).message;
    assert!(message.contains("since the last check"), "{message}");
    assert!(message.contains("it cites"), "{message}");
    assert!(!message.contains("only the files"), "{message}");
}

#[test]
fn a_truth_changed_since_the_base_is_the_spec() {
    let now = verdict(
        "stale",
        &["self"],
        &[("src/tasks.js", "A"), ("test/tasks.test.js", "B")],
    );
    let input = since_r(vec![modified("truth", TR1, &["text"], &[])], now);
    let message = on_tr1(&input).message;
    assert!(
        message.contains(&format!(
            "the spec it reads changed: truth '{TR1}' (text); no file it cites changed"
        )),
        "{message}"
    );
}

#[test]
fn the_suggestion_names_re_verify_on_the_truths_plan() {
    let advisory = on_tr1(&since_r(vec![], stale_after_an_edit()));
    assert_eq!(
        advisory.suggestion.as_deref(),
        Some(
            "`specforge gsd re-verify v1.0/01/01-01 --stale-only` lists the verdicts for the verifier to record \
             again"
        )
    );
}
