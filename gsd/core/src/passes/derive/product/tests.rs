use serde_json::{Value, json};

use super::*;

/// A pass input of `entities` (JSON), with the edges `edges` as `(source, label, target)`.
fn input(entities: Value, edges: &[(&str, &str, &str)]) -> PassInput {
    let edges: Vec<Value> = edges
        .iter()
        .map(
            |(source, label, target)| json!({ "source": source, "label": label, "target": target }),
        )
        .collect();
    serde_json::from_value(json!({ "entities": entities, "edges": edges })).expect("a pass input")
}

/// The values of `input` with no requirement state or proof.
fn computed(input: &PassInput) -> Vec<PassValue> {
    values(input, &BTreeMap::new(), &BTreeMap::new())
}

/// `field` of `entity` among `values`.
fn of<'v>(values: &'v [PassValue], entity: &str, field: &str) -> Option<&'v Value> {
    values
        .iter()
        .find(|v| v.entity == entity && v.field == field)
        .map(|v| &v.value)
}

fn glossary() -> Value {
    json!([
        { "id": "work_item", "kind": "term",
          "fields": { "name": "task", "definition": "a unit of work" },
          "values": { "aliases": ["to-do"], "avoid": ["ticket"] } },
        { "id": "R1", "kind": "requirement", "fields": { "text": "A user can add a Task" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "Every To-Do is kept" } },
        { "id": "R3", "kind": "requirement", "fields": { "text": "A ticket is closed" } },
        { "id": "D1", "kind": "decision", "fields": { "text": "Store on disk", "rationale": "a task survives" } },
        { "id": "add", "kind": "journey", "fields": { "persona": "owner" },
          "values": { "steps": [ { "action": "opens the board" }, { "action": "adds a task" } ] } },
        { "id": "T1", "kind": "task", "fields": { "action": "write the task form" } },
    ])
}

/// The name and the alias are uses, case-insensitively, in any prose field (a decision's rationale, a journey step's
/// action); an avoided word is none, and a task's action is no prose.
#[test]
fn a_term_is_used_where_its_name_or_an_alias_is_a_whole_word() {
    let values = computed(&input(glossary(), &[]));
    assert_eq!(
        of(&values, "work_item", "used_in"),
        Some(&json!(["D1", "R1", "R2", "add"]))
    );
}

#[test]
fn a_term_inside_a_longer_word_is_unused() {
    let values = computed(&input(
        json!([
            { "id": "work_item", "kind": "term", "fields": { "name": "task", "definition": "a unit" } },
            { "id": "R1", "kind": "requirement", "fields": { "text": "Tasks are saved; multitasking is not" } },
        ]),
        &[],
    ));
    assert_eq!(of(&values, "work_item", "used_in"), Some(&json!([])));
}

#[test]
fn an_unused_term_reads_an_empty_list() {
    let values = computed(&input(
        json!([
            { "id": "board", "kind": "term", "fields": { "name": "board", "definition": "the tasks" } },
            { "id": "R1", "kind": "requirement", "fields": { "text": "A user can add a task" } },
        ]),
        &[],
    ));
    assert_eq!(of(&values, "board", "used_in"), Some(&json!([])));
}

/// Sessions `(id, started)` of phase `v1.0/01`, each with one test that names `journey` when it is not empty, and the
/// edges a host sends for the tests' `session` and `journey`.
fn sessions(of: &[(&str, Option<&str>, &str)]) -> PassInput {
    let mut entities = Vec::new();
    let mut edges = Vec::new();
    for (id, started, journey) in of {
        let mut fields = json!({ "phase": "v1.0/01" });
        if let Some(started) = started {
            fields["started"] = json!(started);
        }
        entities.push(json!({ "id": id, "kind": "uat_session", "fields": fields }));
        let test = format!("{id}/U1");
        entities.push(json!({ "id": test, "kind": "uat_test", "fields": { "session": id, "expected": "it works" } }));
        edges.push((test.clone(), "session".to_string(), id.to_string()));
        if !journey.is_empty() {
            edges.push((test, "journey".to_string(), journey.to_string()));
        }
    }
    entities.push(json!({ "id": "add", "kind": "journey", "fields": { "persona": "owner" } }));
    entities.push(json!({ "id": "share", "kind": "journey", "fields": { "persona": "owner" } }));
    let edges: Vec<(&str, &str, &str)> = edges
        .iter()
        .map(|(s, l, t)| (s.as_str(), l.as_str(), t.as_str()))
        .collect();
    input(Value::Array(entities), &edges)
}

#[test]
fn the_last_uat_is_the_latest_session_whose_test_names_the_journey() {
    let values = computed(&sessions(&[
        ("v1.0/01/01-UAT", Some("2026-10-29T10:00:00Z"), "add"),
        ("v1.0/01/02-UAT", Some("2026-10-30T10:00:00Z"), "add"),
        ("v1.0/01/03-UAT", Some("2026-10-31T10:00:00Z"), "share"),
        ("v1.0/01/04-UAT", Some("2026-11-01T10:00:00Z"), ""),
    ]));
    assert_eq!(
        of(&values, "add", "last_uat"),
        Some(&json!("v1.0/01/02-UAT"))
    );
    assert_eq!(
        of(&values, "share", "last_uat"),
        Some(&json!("v1.0/01/03-UAT"))
    );
}

#[test]
fn a_journey_no_test_names_has_no_last_uat() {
    let values = computed(&sessions(&[(
        "v1.0/01/01-UAT",
        Some("2026-10-29T10:00:00Z"),
        "share",
    )]));
    assert_eq!(of(&values, "add", "last_uat"), None);
}

#[test]
fn sessions_started_at_once_are_ordered_by_id() {
    let values = computed(&sessions(&[
        ("v1.0/01/02-UAT", Some("2026-10-29T10:00:00Z"), "add"),
        ("v1.0/01/01-UAT", Some("2026-10-29T10:00:00Z"), "add"),
    ]));
    assert_eq!(
        of(&values, "add", "last_uat"),
        Some(&json!("v1.0/01/02-UAT"))
    );
}

#[test]
fn an_undated_session_is_older_than_a_dated_one() {
    let values = computed(&sessions(&[
        ("v1.0/01/01-UAT", Some("2026-10-29T10:00:00Z"), "add"),
        ("v1.0/01/09-UAT", None, "add"),
    ]));
    assert_eq!(
        of(&values, "add", "last_uat"),
        Some(&json!("v1.0/01/01-UAT"))
    );
}

/// Requirements R1 to R3, a journey `j` whose steps name `steps` (an empty one names none) and whose own
/// `requirements` are `whole`, each written id resolving to itself unless it is `R9`.
fn journey(steps: &[&str], whole: &[&str]) -> PassInput {
    let rows: Vec<Value> = steps
        .iter()
        .map(|r| {
            if r.is_empty() {
                json!({ "action": "looks" })
            } else {
                json!({ "action": "does", "requirement": r })
            }
        })
        .collect();
    let entities = json!([
        { "id": "R1", "kind": "requirement", "fields": { "text": "one" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "two" } },
        { "id": "R3", "kind": "requirement", "fields": { "text": "three" } },
        { "id": "j", "kind": "journey", "fields": { "persona": "owner" },
          "values": { "steps": rows, "requirements": whole } },
    ]);
    let mut edges: Vec<(&str, &str, &str)> = Vec::new();
    for r in steps.iter().filter(|r| !r.is_empty() && **r != "R9") {
        edges.push(("j", "steps.requirement", r));
    }
    for r in whole.iter().filter(|r| **r != "R9") {
        edges.push(("j", "requirements", r));
    }
    input(entities, &edges)
}

/// The requirement states and proofs derive holds, built directly: each of `proven` complete and proven, every other
/// requirement complete with no test.
fn maps(
    proven: &[&'static str],
) -> (
    BTreeMap<&'static str, &'static str>,
    BTreeMap<&'static str, Proof>,
) {
    let all = ["R1", "R2", "R3"];
    let states = all.iter().map(|r| (*r, "complete")).collect();
    let proofs = all
        .iter()
        .map(|r| {
            (
                *r,
                if proven.contains(r) {
                    Proof::Proven
                } else {
                    Proof::NoTest
                },
            )
        })
        .collect();
    (states, proofs)
}

fn covered_of(input: &PassInput, proven: &[&'static str]) -> Option<Value> {
    let (states, proofs) = maps(proven);
    of(&values(input, &states, &proofs), "j", "covered").cloned()
}

#[test]
fn a_journey_whose_steps_are_all_proven_is_covered() {
    assert_eq!(
        covered_of(&journey(&["R1", "R2", "R1"], &[]), &["R1", "R2"]),
        Some(json!(true))
    );
}

#[test]
fn a_step_without_a_requirement_leaves_the_journey_uncovered() {
    assert_eq!(
        covered_of(&journey(&["R1", ""], &[]), &["R1"]),
        Some(json!(false))
    );
}

#[test]
fn an_unproven_step_leaves_the_journey_uncovered() {
    assert_eq!(
        covered_of(&journey(&["R1", "R2"], &[]), &["R1"]),
        Some(json!(false))
    );
}

#[test]
fn a_journey_naming_no_requirement_is_not_covered() {
    assert_eq!(
        covered_of(&journey(&["", ""], &[]), &["R1"]),
        Some(json!(false))
    );
    assert_eq!(covered_of(&journey(&[], &[]), &["R1"]), Some(json!(false)));
}

#[test]
fn a_journey_level_requirement_carries_a_bare_step() {
    assert_eq!(
        covered_of(&journey(&["R1", ""], &["R2"]), &["R1", "R2"]),
        Some(json!(true))
    );
}

#[test]
fn an_unproven_journey_level_requirement_leaves_the_journey_uncovered() {
    assert_eq!(
        covered_of(&journey(&["R1"], &["R2"]), &["R1"]),
        Some(json!(false))
    );
}

/// A reference that does not resolve is unproven, whatever the others say.
#[test]
fn an_unresolved_reference_leaves_the_journey_uncovered() {
    assert_eq!(
        covered_of(&journey(&["R1", "R9"], &[]), &["R1"]),
        Some(json!(false))
    );
    assert_eq!(
        covered_of(&journey(&["R1"], &["R9"]), &["R1"]),
        Some(json!(false))
    );
}

/// A `Proven` thread is not enough: the requirement is complete too, as gsd/E515 reads it.
#[test]
fn an_incomplete_requirement_is_not_proven() {
    let input = journey(&["R1"], &[]);
    let states = BTreeMap::from([("R1", "in_progress")]);
    let proofs = BTreeMap::from([("R1", Proof::Proven)]);
    assert_eq!(
        of(&values(&input, &states, &proofs), "j", "covered"),
        Some(&json!(false))
    );
}

/// Milestones v1.0, v2.0 and v3.0, R1 and R2, and `journeys` `(id, step requirement, milestones)`, each step's
/// requirement resolving to itself.
fn shipped(journeys: &[(&str, &str, &[&str])]) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.0", "kind": "milestone", "fields": { "name": "1.0" } }),
        json!({ "id": "v2.0", "kind": "milestone", "fields": { "name": "2.0" } }),
        json!({ "id": "v3.0", "kind": "milestone", "fields": { "name": "3.0" } }),
        json!({ "id": "R1", "kind": "requirement", "fields": { "text": "one" } }),
        json!({ "id": "R2", "kind": "requirement", "fields": { "text": "two" } }),
    ];
    let mut edges = Vec::new();
    for (id, requirement, milestones) in journeys {
        entities.push(json!({ "id": id, "kind": "journey", "fields": { "persona": "owner" },
            "values": { "steps": [ { "action": "does", "requirement": requirement } ], "milestones": milestones } }));
        edges.push((*id, "steps.requirement", *requirement));
        for milestone in *milestones {
            edges.push((*id, "milestones", *milestone));
        }
    }
    input(Value::Array(entities), &edges)
}

/// The values of `input` where R1 is complete and proven, and R2 complete with no test.
fn with_r1_proven(input: &PassInput) -> Vec<PassValue> {
    let states = BTreeMap::from([("R1", "complete"), ("R2", "complete")]);
    let proofs = BTreeMap::from([("R1", Proof::Proven), ("R2", Proof::NoTest)]);
    values(input, &states, &proofs)
}

#[test]
fn a_milestone_counts_its_journeys_and_the_covered_ones() {
    let values = with_r1_proven(&shipped(&[
        ("add", "R1", &["v1.0"]),
        ("share", "R2", &["v1.0"]),
        ("find", "R2", &["v2.0"]),
    ]));
    assert_eq!(
        of(&values, "v1.0", "journeys_covered"),
        Some(&json!({ "journeys": 2, "covered": 1 }))
    );
    assert_eq!(
        of(&values, "v2.0", "journeys_covered"),
        Some(&json!({ "journeys": 1, "covered": 0 }))
    );
}

#[test]
fn a_journey_listing_two_milestones_counts_in_both() {
    let values = with_r1_proven(&shipped(&[("add", "R1", &["v1.0", "v2.0"])]));
    for milestone in ["v1.0", "v2.0"] {
        assert_eq!(
            of(&values, milestone, "journeys_covered"),
            Some(&json!({ "journeys": 1, "covered": 1 })),
            "{milestone}"
        );
    }
}

#[test]
fn a_milestone_with_no_journey_has_no_count() {
    let values = with_r1_proven(&shipped(&[("add", "R1", &["v1.0"]), ("share", "R2", &[])]));
    assert_eq!(of(&values, "v3.0", "journeys_covered"), None);
    assert_eq!(of(&values, "v2.0", "journeys_covered"), None);
}

/// The journey `j` of [`journey`] with the test facts `facts` `(id, status, the entities its name tags)`.
fn tagged(steps: &[&str], facts: &[(&str, TestStatus, &[&str])]) -> PassInput {
    let mut input = journey(steps, &[]);
    for (id, status, names) in facts {
        crate::passes::thread::fixture::add_test(
            &mut input,
            id,
            id,
            "test/j.test.js",
            *status,
            names,
        );
    }
    input
}

#[test]
fn a_journey_lists_the_tests_tagged_with_it() {
    let input = tagged(
        &["R1"],
        &[
            ("js:app/test/j.test.js#walks", TestStatus::Passed, &["j"]),
            (
                "js:app/test/j.test.js#adds",
                TestStatus::Failed,
                &["j", "R1"],
            ),
            ("js:app/test/j.test.js#other", TestStatus::Passed, &["R1"]),
        ],
    );
    assert_eq!(
        of(&computed(&input), "j", "tests"),
        Some(&json!([
            "js:app/test/j.test.js#adds",
            "js:app/test/j.test.js#walks"
        ]))
    );
}

/// A test fact records no status before it runs: one that tags the journey is listed all the same.
#[test]
fn a_journey_lists_a_tagged_test_that_has_no_status() {
    let mut input = tagged(
        &["R1"],
        &[
            ("js:app/test/j.test.js#pays", TestStatus::Passed, &["j"]),
            ("js:app/test/j.test.js#walks", TestStatus::Passed, &["j"]),
        ],
    );
    for fact in input
        .entities
        .iter_mut()
        .filter(|e| e.id == "js:app/test/j.test.js#pays")
    {
        fact.fields.remove("status");
    }
    assert_eq!(
        of(&computed(&input), "j", "tests"),
        Some(&json!([
            "js:app/test/j.test.js#pays",
            "js:app/test/j.test.js#walks"
        ]))
    );
}

#[test]
fn a_journey_no_test_names_lists_none() {
    assert_eq!(
        of(&computed(&tagged(&["R1"], &[])), "j", "tests"),
        Some(&json!([]))
    );
    let other = tagged(
        &["R1"],
        &[("js:app/test/j.test.js#other", TestStatus::Passed, &["R1"])],
    );
    assert_eq!(of(&computed(&other), "j", "tests"), Some(&json!([])));
}

/// A passing test tagged with the journey is listed and leaves `covered` alone: R1 is unproven, so the journey is not
/// covered.
#[test]
fn a_tagged_test_does_not_cover_a_journey() {
    let input = tagged(
        &["R1"],
        &[("js:app/test/j.test.js#walks", TestStatus::Passed, &["j"])],
    );
    let (states, proofs) = maps(&[]);
    let values = values(&input, &states, &proofs);
    assert_eq!(
        of(&values, "j", "tests"),
        Some(&json!(["js:app/test/j.test.js#walks"]))
    );
    assert_eq!(of(&values, "j", "covered"), Some(&json!(false)));
}
