//! The execution kinds, end to end (`spec/execute.md`): how `specforge` names, resolves and checks a summary, its
//! deliverables and its deviations on the `execute` project.

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
