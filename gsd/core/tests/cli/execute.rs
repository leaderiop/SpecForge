//! The execution kinds, end to end (`spec/execute.md`): how `specforge` names, resolves and checks a summary, its
//! deliverables and its deviations on the `execute` project.

use serde_json::json;

use crate::harness::{Run, project};

const SUMMARY: &str = "v1.0/01/01-01/01-01-SUMMARY";
const CHECK: &str = "{ kind unit ref \"test/tasks.test.js#adds a task\" }";
const DEVIATION: &str = "text    \"list() returns the tasks ordered by id\"\n";

fn node_ids(query: &Run) -> Vec<String> {
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
    graph["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect()
}

#[test]
fn a_deliverable_check_status_is_a_computed_member() {
    // The first use of a computed member through the real host: the derive pass claims `checks.status`, and a status
    // written inside `checks` is E082, as a computed top-level field is.
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        CHECK,
        "{ kind unit ref \"test/tasks.test.js#adds a task\" status pass }",
    );
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E082"],
        "{report:?}"
    );
    assert!(
        report[0].1.contains("checks[0].status")
            && report[0]
                .1
                .contains("deliverable 'v1.0/01/01-01/01-01-SUMMARY/D1'"),
        "{report:?}"
    );
}

#[test]
fn the_execute_project_checks_clean() {
    assert_eq!(project("execute").codes(), Vec::<String>::new());
}

#[test]
fn a_summary_is_named_03_02_summary_within_its_plan() {
    let p = project("execute");
    for name in [SUMMARY, "01-01-SUMMARY"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert_eq!(node_ids(&query), [SUMMARY], "query {name}");
    }
    for name in ["D1", "DV1"] {
        let full = format!("{SUMMARY}/{name}");
        let query = p.run(&["query", &full, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [full.as_str()], "{}", query.all());
    }
}

#[test]
fn summary_times_are_harness_fields() {
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "status                 complete\n",
        "status                 complete\n  started                \"2026-10-29T09:00:00Z\"\n",
    );
    assert_eq!(p.codes(), ["E083"]);
    p.record_harness(SUMMARY, "started", "\"2026-10-29T09:00:00Z\"");
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_summary_requirement_list_and_a_deliverable_requirement_are_edges() {
    let p = project("execute");
    let query = p.run(&["query", SUMMARY, "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout).expect("JSON");
    let labels: Vec<(String, String)> = graph["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .filter(|e| e["source"] == SUMMARY)
        .map(|e| {
            (
                e["label"].as_str().unwrap_or_default().to_string(),
                e["target"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    assert!(
        labels.contains(&("requirements_completed".into(), "TASK-01".into())),
        "{labels:?}"
    );
}

#[test]
fn a_rewritten_deviation_is_refused() {
    let p = project("execute");
    p.commit();
    p.edit(
        "spec/plans.spec",
        DEVIATION,
        "text    \"list() returns tasks by id\"\n",
    );
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E084"],
        "{report:?}"
    );
    assert!(
        report[0]
            .1
            .contains("deviation 'v1.0/01/01-01/01-01-SUMMARY/DV1'"),
        "{report:?}"
    );
}

#[test]
fn a_removed_deviation_is_e085() {
    let p = project("execute");
    p.commit();
    let spec = p.read("spec/plans.spec");
    let start = spec.find("deviation DV1").expect("the deviation");
    let end = spec.find("plan 01-02").expect("the next plan");
    p.write(
        "spec/plans.spec",
        &format!("{}{}", &spec[..start], &spec[end..]),
    );
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E085"],
        "{report:?}"
    );
    assert!(
        report[0].1.contains("'v1.0/01/01-01/01-01-SUMMARY/DV1'"),
        "{report:?}"
    );
}

#[test]
fn an_appended_deviation_is_clean() {
    let p = project("execute");
    p.commit();
    p.edit(
        "spec/plans.spec",
        "plan 01-02 \"Completion\"",
        "deviation DV2 \"A slow start\" {\n  summary v1.0/01/01-01/01-01-SUMMARY\n  kind    issue\n  text    \"The first run was slow\"\n}\n\nplan 01-02 \"Completion\"",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_deleted_file_in_a_summary_is_not_e016() {
    // A deviation names `src/tasks.js`, which this project does not have: a commit may delete or rename a path, so
    // the paths are text and the host's E016 does not read them.
    let p = project("execute");
    assert!(!p.exists("src/tasks.js"));
    assert_eq!(
        p.fields_of(&format!("{SUMMARY}/DV1"))["files"],
        serde_json::json!(["src/tasks.js"])
    );
    assert_eq!(p.codes(), Vec::<String>::new());
}

const TASK: &str = "v1.0/01/01-01/T1";
const PLAN: &str = "v1.0/01/01-01";

/// Writes the commit `name` into `spec/plans.spec` as the `gsd-git` collector would, and records each of its fields in
/// the ledger as the host does. `name` is the first 12 hex digits of `hash`.
fn record_commit(p: &crate::harness::Project, name: &str, hash: &str) {
    let fields = [
        ("hash", format!("{hash:?}")),
        ("task", format!("{TASK:?}")),
        ("plan", format!("{PLAN:?}")),
        ("type", "\"feat\"".to_string()),
        ("subject", "\"feat(01-01): Store module\"".to_string()),
        ("at", "\"2026-10-29T09:00:00Z\"".to_string()),
        ("files", "[\"src/tasks.js\"]".to_string()),
        ("created", "[\"src/tasks.js\"]".to_string()),
        ("insertions", "10".to_string()),
        ("deletions", "0".to_string()),
    ];
    let spec = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!(
            "{spec}\ncommit {name} \"feat(01-01): Store module\" {{\n  hash {hash:?}\n  task {TASK}\n  plan {PLAN}\n  type feat\n  subject \"feat(01-01): Store module\"\n  at \"2026-10-29T09:00:00Z\"\n  files [\"src/tasks.js\"]\n  created [\"src/tasks.js\"]\n  insertions 10\n  deletions 0\n}}\n"
        ),
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    for (field, value) in fields {
        p.record_harness(name, field, &value);
    }
}

const HASH: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn a_recorded_commit_is_read_back_by_its_task_and_plan() {
    // The first use of an inverse on a harness reference through the real host: a commit written with a ledger line
    // per field checks clean, and its two references are edges the task and the plan read it back by.
    let p = project("execute");
    record_commit(&p, "0123456789ab", HASH);
    assert_eq!(p.codes(), Vec::<String>::new());
    let fields = p.fields_of("0123456789ab");
    assert_eq!(fields["task"], TASK, "{fields}");
    assert_eq!(fields["plan"], PLAN, "{fields}");
    assert_eq!(fields["insertions"], 10, "{fields}");
    for target in [TASK, PLAN] {
        let query = p.run(&["query", target, "--depth", "1", "--format", "json"]);
        let graph: serde_json::Value = serde_json::from_str(&query.stdout).expect("JSON");
        let from_commit: Vec<&str> = graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .filter(|e| e["target"] == target && e["source"] == "0123456789ab")
            .filter_map(|e| e["label"].as_str())
            .collect();
        assert_eq!(from_commit.len(), 1, "{target}: {}", query.stdout);
    }
}

#[test]
fn a_task_cannot_write_its_commits_by_hand() {
    // `commits` is the other side of `commit.task`: the host reads it back and refuses it written (E079).
    let p = project("execute");
    record_commit(&p, "0123456789ab", HASH);
    p.edit(
        "spec/plans.spec",
        "action \"Write src/tasks.js with add() and list()\"\n",
        "commits [0123456789ab]\n  action \"Write src/tasks.js with add() and list()\"\n",
    );
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E079"],
        "{report:?}"
    );
    assert!(
        report[0].1.contains("'commits' on task 'v1.0/01/01-01/T1'"),
        "{report:?}"
    );
}

#[test]
fn a_hand_written_commit_is_e083_at_check() {
    let p = project("execute");
    let spec = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!(
            "{spec}\ncommit abcdefabcdef \"x\" {{\n  hash \"abcdefabcdefabcdefabcdefabcdefabcdefabcd\"\n  plan {PLAN}\n  type fix\n  subject \"x\"\n}}\n"
        ),
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E083"; 4],
        "{report:?}"
    );
    for field in ["hash", "plan", "subject", "type"] {
        assert!(
            report
                .iter()
                .any(|(_, m)| m
                    .contains(&format!("harness field '{field}' of commit 'abcdefabcdef'"))),
            "{field}: {report:?}"
        );
    }
}

#[test]
fn a_hand_written_commit_field_is_refused_by_the_spec_guard() {
    let p = project("execute");
    let file = p.root().join("spec/plans.spec");
    let content = format!(
        "{}\ncommit abcdefabcdef \"x\" {{\n  hash \"abcdefabcdefabcdefabcdefabcdefabcdefabcd\"\n  subject \"x\"\n}}\n",
        p.read("spec/plans.spec")
    );
    let event = serde_json::json!({
        "hook_event_name": "PreToolUse", "tool_name": "Write", "session_id": "s1", "cwd": p.root(),
        "tool_input": { "file_path": file, "content": content }
    });
    let run = p.run_stdin(
        &[
            "hook",
            "core.spec_guard",
            "--host",
            "claude-code",
            "--event",
            "pre_tool",
        ],
        &event.to_string(),
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let reply: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("the hook answers JSON ({e}): {}", run.all()));
    let answer = &reply["hookSpecificOutput"];
    assert_eq!(answer["permissionDecision"], "deny", "{reply}");
    let reason = answer["permissionDecisionReason"]
        .as_str()
        .unwrap_or_default();
    assert!(
        reason.contains(
            "harness field 'hash' of commit 'abcdefabcdef' has no line in specforge-harness"
        ),
        "{reason}"
    );
}

#[test]
fn an_agent_set_of_a_commit_field_is_harness_field() {
    let p = project("execute");
    record_commit(&p, "0123456789ab", HASH);
    let run = p.run(&["set", "0123456789ab", "subject", "x"]);
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(run.all().contains("harness_field"), "{}", run.all());
    assert!(!p.read("spec/plans.spec").contains("subject \"x\""));
}

#[test]
fn a_commit_named_by_twelve_decimal_digits_is_a_name() {
    let p = project("execute");
    record_commit(
        &p,
        "012345678901",
        "012345678901abcdef0123456789abcdef012345",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    let query = p.run(&["query", "012345678901", "--depth", "0", "--format", "brief"]);
    assert_eq!(node_ids(&query), ["012345678901"], "{}", query.all());
}

#[test]
fn a_pause_is_named_within_its_phase() {
    let p = project("execute");
    let query = p.run(&["query", "v1.0/01/PS1", "--depth", "0", "--format", "brief"]);
    assert_eq!(node_ids(&query), ["v1.0/01/PS1"], "{}", query.all());
    let query = p.run(&["query", "v1.0/01/HO1", "--depth", "0", "--format", "brief"]);
    assert_eq!(node_ids(&query), ["v1.0/01/HO1"], "{}", query.all());
}

#[test]
fn a_phaseless_pause_is_in_the_project_scope() {
    let p = project("execute");
    p.edit(
        "spec/ops.spec",
        "  phase   v1.0/01\n  plan    v1.0/01/01-02\n  reason",
        "  plan    v1.0/01/01-02\n  reason",
    );
    let query = p.run(&["query", "PS1", "--depth", "0", "--format", "brief"]);
    assert_eq!(node_ids(&query), ["PS1"], "{}", query.all());
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_hand_written_pause_at_is_e083() {
    let p = project("execute");
    p.edit(
        "spec/ops.spec",
        "  reason  \"How completion is stored is still open\"\n",
        "  reason  \"How completion is stored is still open\"\n  at      \"2026-10-29T10:00:00Z\"\n",
    );
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E083"],
        "{report:?}"
    );
    assert!(
        report[0]
            .1
            .contains("harness field 'at' of pause 'v1.0/01/PS1'"),
        "{report:?}"
    );
    p.record_harness("v1.0/01/PS1", "at", "\"2026-10-29T10:00:00Z\"");
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_resolved_blocker_at_a_local_time_is_e107() {
    // The first black-box E107: `resolved.at` is an authored time member, so a local time is refused.
    let p = project("execute");
    p.edit(
        "spec/ops.spec",
        "at  \"2026-10-29T10:00:00Z\"",
        "at  \"2026-10-29T10:00:00+02:00\"",
    );
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["gsd/E107"],
        "{report:?}"
    );
    assert!(
        report[0].1.contains("blocker 'BL1'") && report[0].1.contains("2026-10-29T10:00:00+02:00"),
        "{report:?}"
    );
}

#[test]
fn a_hand_written_blocker_raised_is_e083() {
    let p = project("execute");
    p.edit(
        "spec/ops.spec",
        "  text  \"How completion is stored is undecided\"\n",
        "  text  \"How completion is stored is undecided\"\n  raised \"2026-10-28T10:00:00Z\"\n",
    );
    let report = p.report();
    assert_eq!(
        report.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E083"],
        "{report:?}"
    );
    assert!(
        report[0]
            .1
            .contains("harness field 'raised' of blocker 'BL1'"),
        "{report:?}"
    );
}

#[test]
fn a_handoff_names_its_blockers_and_decisions() {
    let p = project("execute");
    let query = p.run(&["query", "v1.0/01/HO1", "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout).expect("JSON");
    let mut edges: Vec<(String, String)> = graph["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .filter(|e| e["source"] == "v1.0/01/HO1")
        .map(|e| {
            (
                e["label"].as_str().unwrap_or_default().to_string(),
                e["target"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    edges.sort();
    assert_eq!(
        edges,
        [
            ("blockers".into(), "BL1".into()),
            ("completed_work.task".into(), "v1.0/01/01-01/T1".into()),
            ("decisions_made".into(), "v1.0/01/D-01".into()),
            ("phase".into(), "v1.0/01".into()),
            ("task".into(), "v1.0/01/01-02/T1".into()),
        ]
    );
}

#[test]
fn a_decision_of_a_summary_is_linked_by_its_key_decisions() {
    let p = project("execute");
    p.write(
        "spec/decisions.spec",
        "decision D-77 \"Keep a store\" {\n  phase  v1.0/01\n  text   \"Tasks live in a store\"\n  origin summary\n}\n",
    );
    p.edit(
        "spec/plans.spec",
        "requirements_completed [TASK-01]",
        "requirements_completed [TASK-01]\n  key_decisions          [v1.0/01/D-77]",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    assert_eq!(
        p.fields_of("v1.0/01/01-01/01-01-SUMMARY")["key_decisions"],
        json!(["v1.0/01/D-77"])
    );
}
