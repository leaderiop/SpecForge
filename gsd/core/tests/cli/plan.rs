//! The plan kinds, end to end: how `specforge` names, resolves and checks them on the `plan` project.

use crate::harness::{Run, project};

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
fn the_plan_project_checks_clean() {
    assert_eq!(project("plan").codes(), Vec::<String>::new());
}

#[test]
fn a_plan_is_found_as_v1_0_03_03_02() {
    let p = project("plan");
    for name in ["v1.0/01/01-02", "01-02"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert_eq!(node_ids(&query), ["v1.0/01/01-02"], "query {name}");
    }
}

#[test]
fn a_quick_plan_is_in_the_project_scope() {
    let p = project("plan");
    let query = p.run(&["query", "261028-a1b", "--depth", "0", "--format", "brief"]);
    assert_eq!(query.code, 0, "query: {}", query.all());
    assert_eq!(node_ids(&query), ["261028-a1b"]);
}

#[test]
fn two_phases_may_each_have_a_plan_01() {
    let p = project("plan");
    p.write(
        "spec/more.spec",
        "plan 01-01 \"Second\" {\n  phase v1.0/02\n  type execute\n  files_modified [\"src/b.js\"]\n  requirements [STORE-01]\n  objective \"b\"\n}\n",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    for name in ["v1.0/01/01-01", "v1.0/02/01-01"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [name], "query {name}: {}", query.all());
    }
}

fn edges(p: &crate::harness::Project, plan: &str) -> Vec<(String, String, String)> {
    let query = p.run(&["query", plan, "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
    graph["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .filter(|e| e["source"] == plan)
        .map(|e| {
            let text = |key: &str| e[key].as_str().unwrap_or_default().to_string();
            (text("label"), text("source"), text("target"))
        })
        .collect()
}

#[test]
fn a_plan_depends_on_its_sibling_by_short_name() {
    let p = project("plan");
    let found = edges(&p, "v1.0/01/01-02");
    assert!(
        found.contains(&(
            "depends_on".into(),
            "v1.0/01/01-02".into(),
            "v1.0/01/01-01".into()
        )),
        "{found:#?}"
    );
}

#[test]
fn a_coupling_peer_is_an_edge() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "depends_on     [01-01]\n",
        "depends_on     [01-01]\n  coupling_justified [{ peer 01-01 reason \"both read the store\" }]\n",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    let found = edges(&p, "v1.0/01/01-02");
    assert!(
        found.contains(&(
            "coupling_justified.peer".into(),
            "v1.0/01/01-02".into(),
            "v1.0/01/01-01".into()
        )),
        "{found:#?}"
    );
}

#[test]
fn a_file_the_plan_will_create_is_not_e016() {
    let p = project("plan");
    // The plans name `src/store.js` and the others; none exists, and a `files` field would be E016 on each.
    assert!(!p.exists("src/store.js"));
    assert_eq!(p.codes(), Vec::<String>::new());
}

fn fields_of(p: &crate::harness::Project, id: &str) -> serde_json::Value {
    let query = p.run(&["query", id, "--depth", "0", "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
    graph["nodes"][0]["fields"].clone()
}

#[test]
fn a_task_is_named_within_its_plan() {
    let p = project("plan");
    for name in ["v1.0/01/01-01/T1", "v1.0/01/01-02/T1", "v1.0/02/02-01/T1"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [name], "query {name}: {}", query.all());
    }
}

#[test]
fn a_task_with_two_verify_commands_declares_both() {
    // `verify [` is a field (a block list), not a verify statement.
    let p = project("plan");
    let fields = fields_of(&p, "v1.0/01/01-02/T1");
    assert_eq!(
        fields["verify"],
        serde_json::json!([
            { "automated": "node --test test/tasks.test.js", "fails_when": "the test run exits non-zero" },
            { "automated": "node src/complete.js --check", "fails_when": "the script exits non-zero" }
        ])
    );
}

#[test]
fn a_verify_item_without_automated_is_refused() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "{ automated \"node src/complete.js --check\" fails_when \"the script exits non-zero\" }",
        "{ fails_when \"the script exits non-zero\" }",
    );
    assert_eq!(p.codes(), ["E006"]);
    assert!(
        p.run(&["check"]).all().contains(
            "task 'v1.0/01/01-02/T1' is missing required member 'automated' in verify[1]"
        )
    );
}

#[test]
fn a_hand_written_checkpoint_answer_is_e083() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "resume_signal \"Type the option name\"\n",
        "resume_signal \"Type the option name\"\n  answer { option \"flag\" by \"me\" }\n",
    );
    assert_eq!(p.codes(), ["E083"]);
}

#[test]
fn a_settled_by_names_a_decision() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "action \"Write src/tasks.js with add() and list()\"\n",
        "reversibility { rating one_way why \"the file format\" settled_by /D-01 }\n  action \"Write src/tasks.js with add() and list()\"\n",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    let found = edges(&p, "v1.0/01/01-01/T1");
    assert!(
        found.contains(&(
            "reversibility.settled_by".into(),
            "v1.0/01/01-01/T1".into(),
            "D-01".into()
        )),
        "{found:#?}"
    );
}

#[test]
fn a_checkpoint_is_after_a_task_of_its_plan() {
    let p = project("plan");
    let found = edges(&p, "v1.0/01/01-02/C1");
    assert!(
        found.contains(&(
            "after".into(),
            "v1.0/01/01-02/C1".into(),
            "v1.0/01/01-02/T1".into()
        )),
        "{found:#?}"
    );
}
