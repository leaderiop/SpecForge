use serde_json::json;

use super::*;

const V: &str = "v1.0/01/01-VERIFICATION";

fn judgement(outcome: &str, state: &str, passed: bool) -> serde_json::Value {
    json!({ "rule": "gsd/E501", "state": state, "passed": passed, "outcome": outcome, "at": "2026-10-29T10:00:00Z",
            "files": [{ "path": "src/a.js", "digest": "sha256:x" }] })
}

fn truth(n: u8, judgements: Vec<serde_json::Value>) -> serde_json::Value {
    json!({ "id": format!("v1.0/01/01-01/TR{n}"), "kind": "truth", "judgements": judgements })
}

/// A phase with one plan, these items and a verification; `extra` are more entities.
fn input(items: Vec<serde_json::Value>, extra: Vec<serde_json::Value>) -> PassInput {
    let mut entities = vec![
        json!({ "id": V, "kind": "verification", "fields": { "phase": "v1.0/01" } }),
        json!({ "id": "v1.0/01/01-01", "kind": "plan", "fields": { "phase": "v1.0/01" } }),
    ];
    entities.extend(items);
    entities.extend(extra);
    serde_json::from_value(crate::passes::plans::with_field_edges(json!({
        "entities": entities,
        "edges": [{ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" }],
    })))
    .expect("a pass input")
}

fn of(output: &[PassValue], field: &str) -> Option<serde_json::Value> {
    output
        .iter()
        .find(|v| v.entity == V && v.field == field)
        .map(|v| v.value.clone())
}

fn run(input: &PassInput) -> Vec<PassValue> {
    values(input, &BTreeMap::new())
}

#[test]
fn all_truths_verified_is_passed() {
    let fresh = judgement("verified", "fresh", true);
    let out = run(&input(
        vec![truth(1, vec![fresh.clone()]), truth(2, vec![fresh])],
        vec![],
    ));
    assert_eq!(of(&out, "status"), Some(json!("passed")));
    assert_eq!(of(&out, "score"), Some(json!("2/2")));
    assert_eq!(of(&out, "verified"), Some(json!("2026-10-29T10:00:00Z")));
    assert_eq!(of(&out, "covered_files"), Some(json!(["src/a.js"])));
}

#[test]
fn a_failed_truth_is_gaps_found() {
    let out = run(&input(
        vec![
            truth(1, vec![judgement("failed", "fresh", false)]),
            truth(2, vec![judgement("verified", "fresh", true)]),
        ],
        vec![],
    ));
    assert_eq!(of(&out, "status"), Some(json!("gaps_found")));
    assert_eq!(of(&out, "score"), Some(json!("1/2")));
}

#[test]
fn an_override_counts_as_verified() {
    let o = json!({ "id": format!("{V}/OV1"), "kind": "override", "fields": { "verification": V, "truth": "v1.0/01/01-01/TR1" } });
    let out = run(&input(vec![truth(1, vec![])], vec![o]));
    assert_eq!(of(&out, "status"), Some(json!("passed")));
    assert_eq!(of(&out, "score"), Some(json!("1/1")));
}

#[test]
fn behavior_unverified_is_human_needed_and_not_counted_as_verified() {
    let out = run(&input(
        vec![
            truth(
                1,
                vec![judgement("present_behavior_unverified", "fresh", false)],
            ),
            truth(2, vec![judgement("verified", "fresh", true)]),
        ],
        vec![],
    ));
    assert_eq!(of(&out, "status"), Some(json!("human_needed")));
    assert_eq!(of(&out, "score"), Some(json!("1/2")));
}

#[test]
fn a_deferred_truth_does_not_count() {
    let mut input = input(
        vec![
            truth(1, vec![judgement("verified", "fresh", true)]),
            truth(2, vec![judgement("failed", "fresh", false)]),
        ],
        vec![],
    );
    input.entities[0].values.insert(
        "deferred".into(),
        json!([{ "truth": "v1.0/01/01-01/TR2", "addressed_in": "v1.0/02", "evidence": "phase 2" }]),
    );
    let out = run(&input);
    assert_eq!(of(&out, "status"), Some(json!("passed")));
    assert_eq!(of(&out, "score"), Some(json!("1/1")));
}

#[test]
fn a_stale_judgement_is_not_a_pass() {
    let out = run(&input(
        vec![truth(1, vec![judgement("verified", "stale", true)])],
        vec![],
    ));
    assert_eq!(of(&out, "status"), Some(json!("pending")));
    assert_eq!(of(&out, "score"), Some(json!("0/1")));
    assert_eq!(of(&out, "verified"), Some(json!("2026-10-29T10:00:00Z")));
}

#[test]
fn a_blocker_anti_pattern_is_gaps_found() {
    let mut input = input(
        vec![truth(1, vec![judgement("verified", "fresh", true)])],
        vec![],
    );
    input.entities[0].values.insert(
        "anti_patterns".into(),
        json!([{ "file": "src/a.js", "line": 3, "text": "// FIXME", "severity": "blocker" }]),
    );
    assert_eq!(of(&run(&input), "status"), Some(json!("gaps_found")));
}

#[test]
fn a_gap_is_closed_by_a_complete_gap_closure_plan() {
    let gap = json!({ "id": format!("{V}/VG1"), "kind": "verify_gap" });
    let closer = json!({ "id": "v1.0/01/01-03", "kind": "plan", "values": { "gap_closure": [format!("{V}/VG1")] } });
    let input = input(vec![], vec![gap, closer]);
    let statuses: Statuses = [("v1.0/01/01-03", "complete")].into();
    let out = values(&input, &statuses);
    let field = |f: &str| {
        out.iter()
            .find(|v| v.entity == format!("{V}/VG1") && v.field == f)
            .map(|v| v.value.clone())
    };
    assert_eq!(field("status"), Some(json!("closed")));
    assert_eq!(field("closed_by"), Some(json!("v1.0/01/01-03")));
    let open = values(&input, &BTreeMap::new());
    assert!(
        open.iter()
            .any(|v| v.field == "status" && v.value == json!("open"))
    );
}

#[test]
fn a_partial_scan_is_not_passed() {
    let mut input = input(
        vec![truth(1, vec![judgement("verified", "fresh", true)])],
        vec![],
    );
    input.entities[0]
        .values
        .insert("scan_partial".into(), json!(true));
    assert_eq!(of(&run(&input), "status"), Some(json!("pending")));
}

fn baseline(entities: serde_json::Value) -> Option<PassBaseline> {
    Some(
        serde_json::from_value(json!({ "revision": "abc123", "entities": entities }))
            .expect("a baseline"),
    )
}

fn then(
    id: &str,
    kind: &str,
    fields: serde_json::Value,
    judgements: Vec<serde_json::Value>,
) -> serde_json::Value {
    json!({ id: { "kind": kind, "fields": fields, "judgements": judgements } })
}

fn merged(parts: Vec<serde_json::Value>) -> serde_json::Value {
    let mut all = serde_json::Map::new();
    for part in parts {
        all.extend(part.as_object().cloned().unwrap_or_default());
    }
    all.into()
}

#[test]
fn re_verification_closes_keeps_and_regresses_truths_by_verdict() {
    let tr = |n: u8| format!("v1.0/01/01-01/TR{n}");
    let mut found = input(
        vec![
            truth(1, vec![judgement("verified", "fresh", true)]),
            truth(2, vec![judgement("failed", "fresh", false)]),
            truth(3, vec![judgement("uncertain", "fresh", false)]),
        ],
        vec![],
    );
    found.previous = baseline(merged(vec![
        then(V, "verification", json!({}), vec![]),
        then(
            &tr(1),
            "truth",
            json!({}),
            vec![judgement("failed", "fresh", false)],
        ),
        then(
            &tr(2),
            "truth",
            json!({}),
            vec![judgement("verified", "fresh", true)],
        ),
        then(
            &tr(3),
            "truth",
            json!({}),
            vec![judgement("failed", "fresh", false)],
        ),
    ]));
    let block = of(&run(&found), "re_verification").expect("a re_verification block");
    assert_eq!(
        block,
        json!({ "gaps_closed": [tr(1)], "gaps_remaining": [tr(3)], "regressions": [tr(2)] })
    );
}

#[test]
fn re_verification_closes_and_reopens_verify_gaps() {
    let gap = |n: u8| format!("{V}/VG{n}");
    let entity =
        |n: u8| json!({ "id": gap(n), "kind": "verify_gap", "fields": { "verification": V } });
    let closer =
        json!({ "id": "v1.0/01/01-03", "kind": "plan", "values": { "gap_closure": [gap(1)] } });
    let mut found = input(vec![], vec![entity(1), entity(2), closer]);
    found.previous = baseline(merged(vec![
        then(V, "verification", json!({}), vec![]),
        then(&gap(1), "verify_gap", json!({ "status": "open" }), vec![]),
        then(&gap(2), "verify_gap", json!({ "status": "closed" }), vec![]),
    ]));
    let statuses: Statuses = [("v1.0/01/01-03", "complete")].into();
    let out = values(&found, &statuses);
    let block = of(&out, "re_verification").expect("a re_verification block");
    assert_eq!(
        block,
        json!({ "gaps_closed": [gap(1)], "regressions": [gap(2)] })
    );
}

#[test]
fn e501_outcomes_are_the_derivations() {
    let status = |outcome: &str, passed: bool| {
        let out = run(&input(
            vec![truth(1, vec![judgement(outcome, "fresh", passed)])],
            vec![],
        ));
        of(&out, "status")
    };
    assert_eq!(status("verified", true), Some(json!("passed")));
    assert_eq!(status("verified_coincidental", true), Some(json!("passed")));
    assert_eq!(status("failed", false), Some(json!("gaps_found")));
    assert_eq!(status("uncertain", false), Some(json!("pending")));
    assert_eq!(
        status("present_behavior_unverified", false),
        Some(json!("human_needed"))
    );
}

#[test]
fn a_truth_with_no_verdict_is_never_passed() {
    let out = run(&input(vec![truth(1, vec![])], vec![]));
    assert_ne!(of(&out, "status"), Some(json!("passed")));
}

#[test]
fn the_scan_verifies_only_a_wired_key_link() {
    let state = |kind, scan| item_state(kind, scan, Item::Unknown, false);
    assert_eq!(state("key_link", Some("wired")), Item::Verified);
    assert_eq!(state("key_link", Some("unchecked")), Item::Unknown);
    assert_eq!(state("key_link", Some("not_wired")), Item::Failed);
    assert_eq!(state("artifact", Some("present")), Item::Unknown);
}
