use std::collections::BTreeMap;

use serde_json::json;

use specforge_extension_sdk::EntityTests;

use super::*;

const PHASE: &str = "v1.0/01";

fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn edge(source: &str, label: &str, target: &str) -> PassEdge {
    PassEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

fn plan(name: &str) -> String {
    format!("{PHASE}/{name}")
}

fn commit(plan_name: &str, task: Option<&str>, files: &[&str], created: &[&str]) -> Commit {
    Commit {
        plan: Some(plan(plan_name)),
        task: task.map(|t| format!("{}/{t}", plan(plan_name))),
        files: files.iter().map(|f| f.to_string()).collect(),
        created: created.iter().map(|f| f.to_string()).collect(),
        ..Default::default()
    }
}

fn summary(plan_name: &str, status: &str) -> PassEntity {
    entity(
        &format!("{}/S", plan(plan_name)),
        "summary",
        &[("status", status)],
    )
}

fn input(plans: &[&str], depends: &[(&str, &str)], summaries: Vec<PassEntity>) -> PassInput {
    let mut entities: Vec<PassEntity> = plans
        .iter()
        .map(|p| entity(&plan(p), "plan", &[]))
        .collect();
    entities.extend(summaries);
    PassInput {
        entities,
        edges: depends
            .iter()
            .map(|(a, b)| edge(&plan(a), "depends_on", &plan(b)))
            .collect(),
        ..Default::default()
    }
}

fn status(input: &PassInput, commits: &[Commit], name: &str) -> &'static str {
    plan_statuses(input, commits)[plan(name).as_str()]
}

#[test]
fn plan_status_follows_summary_halt_and_commits() {
    let base = input(
        &["a", "b", "c"],
        &[("b", "a")],
        vec![summary("a", "Halted")],
    );
    assert_eq!(status(&base, &[], "a"), "halted");
    assert_eq!(status(&base, &[], "b"), "blocked");
    assert_eq!(status(&base, &[], "c"), "ready");
    // A commit moves a ready plan to executing, but not a blocked one.
    let commits = [commit("b", None, &[], &[]), commit("c", None, &[], &[])];
    assert_eq!(status(&base, &commits, "b"), "blocked");
    assert_eq!(status(&base, &commits, "c"), "executing");
    // A complete summary wins over everything, and the status is case-folded.
    let done = input(&["a"], &[], vec![summary("a", "COMPLETE")]);
    assert_eq!(
        status(&done, &[commit("a", None, &[], &[])], "a"),
        "complete"
    );
}

#[test]
fn a_task_named_by_a_commit_is_done() {
    let mut input = input(&["a"], &[], vec![]);
    input
        .entities
        .push(entity(&format!("{}/t1", plan("a")), "task", &[]));
    input
        .entities
        .push(entity(&format!("{}/t2", plan("a")), "task", &[]));
    let task_status = |input: &PassInput, commits: &[Commit]| -> Vec<(String, Value)> {
        values(input, commits, &plan_statuses(input, commits))
            .into_iter()
            .filter(|v| v.field == "status" && v.entity.ends_with(['1', '2']))
            .map(|v| (v.entity, v.value))
            .collect()
    };
    assert_eq!(
        task_status(&input, &[commit("a", Some("t1"), &[], &[])]),
        [
            (format!("{}/t1", plan("a")), json!("done")),
            (format!("{}/t2", plan("a")), json!("pending")),
        ]
    );
    // A complete plan finishes every task.
    input.entities.push(summary("a", "complete"));
    assert!(task_status(&input, &[]).iter().all(|(_, v)| v == "done"));
}

fn with_times(started: Option<&str>, completed: Option<&str>) -> PassEntity {
    let mut fields = vec![];
    if let Some(s) = started {
        fields.push(("started", s));
    }
    if let Some(c) = completed {
        fields.push(("completed", c));
    }
    entity("v1.0/01/a/S", "summary", &fields)
}

#[test]
fn duration_is_gsds() {
    let d = |s, c| duration(&with_times(s, c));
    assert_eq!(
        d(Some("2026-10-28T08:00:00Z"), Some("2026-10-28T08:45:00Z")).as_deref(),
        Some("45 min")
    );
    assert_eq!(
        d(Some("2026-10-28T08:00:00Z"), Some("2026-10-28T09:05:00Z")).as_deref(),
        Some("1h 5m")
    );
    assert_eq!(d(Some("2026-10-28T08:00:00Z"), None), None);
    // A completion before the start has no duration.
    assert_eq!(
        d(Some("2026-10-28T09:00:00Z"), Some("2026-10-28T08:00:00Z")),
        None
    );
}

#[test]
fn actuals_and_key_files_come_from_commits() {
    let mut input = input(
        &["a", "b"],
        &[],
        vec![summary("a", "complete"), summary("b", "complete")],
    );
    input
        .entities
        .push(entity(&format!("{}/t1", plan("a")), "task", &[]));
    let commits = [
        commit("a", Some("t1"), &["src/x.js", "src/y.js"], &["src/y.js"]),
        commit("a", None, &["src/x.js", "src/z.js"], &["src/z.js"]),
    ];
    let given: BTreeMap<(String, String), Value> =
        values(&input, &commits, &plan_statuses(&input, &commits))
            .into_iter()
            .map(|v| ((v.entity, v.field), v.value))
            .collect();
    let at = |plan_name: &str, field: &str| {
        given.get(&(format!("{}/S", plan(plan_name)), field.to_string()))
    };
    assert_eq!(at("a", "actuals"), Some(&json!({"tasks": 1, "commits": 2})));
    assert_eq!(
        at("a", "files_created"),
        Some(&json!(["src/y.js", "src/z.js"]))
    );
    assert_eq!(at("a", "files_modified"), Some(&json!(["src/x.js"])));
    // With no commit, the count is zero and the lists are absent.
    assert_eq!(at("b", "actuals"), Some(&json!({"tasks": 0, "commits": 0})));
    assert_eq!(at("b", "files_created"), None);
    assert_eq!(at("b", "files_modified"), None);
}

fn deliverable(refs: &[&str]) -> PassEntity {
    let mut d = entity("v1.0/01/a/S/D1", "deliverable", &[]);
    d.values.insert(
        "checks".into(),
        Value::Array(refs.iter().map(|r| json!({ "ref": r })).collect()),
    );
    d
}

fn report(tests: &[(&str, TestStatus)]) -> TestResults {
    let mut r = TestResults::default();
    r.results.insert(
        "v1.0/01/a/S/D1".into(),
        EntityTests {
            tests: tests
                .iter()
                .map(|(name, status)| TestResult {
                    name: name.to_string(),
                    status: *status,
                    verify: None,
                    duration_ms: None,
                })
                .collect(),
        },
    );
    r
}

#[test]
fn a_check_status_is_pass_fail_or_unknown_from_the_report() {
    let d = deliverable(&["t#a", "t#b", "t#c"]);
    let mut input = PassInput::default();
    // No report, no status.
    assert!(check_statuses(&input, &d).is_empty());
    input.test_results = Some(report(&[
        ("t#a", TestStatus::Passed),
        ("t#b", TestStatus::Passed),
        ("t#b", TestStatus::Failed),
        ("other", TestStatus::Failed),
    ]));
    let given: Vec<(String, Value)> = check_statuses(&input, &d)
        .into_iter()
        .map(|v| (v.field, v.value))
        .collect();
    assert_eq!(
        given,
        [
            ("checks[0].status".to_string(), json!("pass")),
            ("checks[1].status".to_string(), json!("fail")),
            ("checks[2].status".to_string(), json!("unknown")),
        ]
    );
}

fn judgement(rule: &str, state: &str, passed: bool) -> Value {
    json!({ "rule": format!("gsd/{rule}"), "state": state, "passed": passed })
}

/// A plan with these plan-check judgements, over a project of one plan.
fn judged_plan(judgements: Vec<Value>) -> PassInput {
    let mut input = input(&["a"], &[], vec![]);
    input.entities[0] = serde_json::from_value(
        json!({ "id": plan("a"), "kind": "plan", "judgements": judgements }),
    )
    .expect("a plan entity");
    input
}

fn checks(states: [(&str, bool); 4]) -> Vec<Value> {
    ["W311", "W313", "W315", "W316"]
        .into_iter()
        .zip(states)
        .map(|(rule, (state, passed))| judgement(rule, state, passed))
        .collect()
}

#[test]
fn a_plan_is_checked_only_with_four_fresh_passing_verdicts() {
    let fresh = ("fresh", true);
    // All four fresh and passing: checked.
    assert_eq!(
        status(&judged_plan(checks([fresh; 4])), &[], "a"),
        "checked"
    );
    // Any one missing, stale, unrecorded or failing: draft.
    for odd in [
        ("missing", false),
        ("stale", false),
        ("unrecorded", false),
        ("fresh", false),
    ] {
        for at in 0..4 {
            let mut states = [fresh; 4];
            states[at] = odd;
            assert_eq!(
                status(&judged_plan(checks(states)), &[], "a"),
                "draft",
                "{odd:?} at {at}"
            );
        }
    }
    // Fewer than four judgements is no check either.
    let mut three = checks([fresh; 4]);
    three.pop();
    assert_eq!(status(&judged_plan(three), &[], "a"), "draft");
    // Another rule's judgement is not a plan check.
    let other = vec![judgement("E501", "fresh", true)];
    assert_eq!(status(&judged_plan(other), &[], "a"), "ready");
    // No judgement at all (the rules are gated off): GSD's no-check path.
    assert_eq!(status(&judged_plan(vec![]), &[], "a"), "ready");
}

#[test]
fn execution_beats_the_check() {
    let fresh = ("fresh", true);
    let mut input = judged_plan(checks([fresh; 4]));
    assert_eq!(status(&input, &[], "a"), "checked");
    assert_eq!(
        status(&input, &[commit("a", None, &[], &[])], "a"),
        "executing"
    );
    input.entities.push(summary("a", "complete"));
    assert_eq!(status(&input, &[], "a"), "complete");
    // A failing check does not hide a halt either.
    let mut halted = judged_plan(checks([("missing", false); 4]));
    halted.entities.push(summary("a", "halted"));
    assert_eq!(status(&halted, &[], "a"), "halted");
}
