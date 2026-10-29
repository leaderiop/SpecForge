//! Black-box tests: the `specforge` binary on small projects that load the built `@gsd/core` (ADR gsd/0001 D5, D7).

mod agents;
mod cmd_execute;
mod cmd_refactor;
mod cmd_state;
mod cmd_verify;
mod cmd_windows;
mod collect;
mod commands_support;
mod discuss;
mod e502_probe;
mod execute;
mod execute_rules;
mod execute_status;
mod gates;
mod harness;
mod install_agents;
mod judged;
mod judged_flow;
mod judged_refusals;
mod ladder;
mod parity_discuss;
mod parity_plans;
mod parity_roadmap;
mod parity_state;
mod parity_verify;
mod plan;
mod plan_rules;
mod progress;
mod roadmap;
mod scan;
mod settings;
mod verify_paths;
mod verify_rules;
mod verify_status;
mod verify_verdicts;

use harness::project;

#[test]
fn an_empty_project_loads_gsd_core_and_checks_clean() {
    let p = project("empty");
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    let listed = p.run(&["extensions"]);
    assert!(
        listed.all().contains("@gsd/core") && listed.all().contains("0.1.0"),
        "extensions: {}",
        listed.all()
    );
}

#[test]
fn explain_names_a_gsd_code() {
    let p = project("empty");
    let explained = p.run(&["explain", "gsd/E101"]);
    assert_eq!(explained.code, 0, "explain: {}", explained.all());
    assert!(
        explained
            .all()
            .contains("Requirement not mapped to a phase"),
        "explain: {}",
        explained.all()
    );
}

#[test]
fn a_project_with_requirements_checks_clean() {
    let p = project("roadmap");
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    assert!(
        check.all().contains("0 errors, 0 warnings"),
        "check: {}",
        check.all()
    );
}

#[test]
fn a_requirement_with_an_unknown_tier_is_refused() {
    let p = project("roadmap");
    p.write(
        "spec/bad.spec",
        "requirement TASK-09 \"Bad\" {\n  text \"x\"\n  category \"Tasks\"\n  tier v3\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains("E061") && check.all().contains("v1, v2, out_of_scope"),
        "check: {}",
        check.all()
    );
}

fn node_ids(query: &harness::Run) -> Vec<String> {
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
fn a_phase_is_found_by_its_full_path_and_by_a_unique_short_form() {
    let p = project("roadmap");
    for name in ["v1.0/01.1", "01.1"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert_eq!(node_ids(&query), ["v1.0/01.1"], "query {name}");
    }
}

#[test]
fn two_milestones_may_each_have_a_phase_01() {
    let p = project("roadmap");
    p.write(
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\nphase 01 \"Sharing\" {\n  milestone v2.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"Boards are shared\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    for name in ["v1.0/01", "v2.0/01"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [name], "query {name}: {}", query.all());
    }
}

#[test]
fn an_ambiguous_short_form_names_each_candidate() {
    let p = project("roadmap");
    p.write(
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\nphase 01 \"Sharing\" {\n  milestone v2.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"Boards are shared\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let query = p.run(&["query", "01", "--depth", "0", "--format", "brief"]);
    assert_ne!(
        query.code,
        0,
        "an ambiguous short form is refused: {}",
        query.all()
    );
    assert!(
        query.all().contains("v1.0/01") && query.all().contains("v2.0/01"),
        "candidates: {}",
        query.all()
    );
}

#[test]
fn a_milestone_in_a_workstream_is_named_by_its_workstream() {
    let p = project("roadmap");
    // `v1.0` of the workstream is a second milestone beside the project's own `v1.0`.
    p.write(
        "spec/learning.spec",
        "workstream auth-rework \"Auth rework\" {\n  purpose \"Replace the session store\"\n}\n\nmilestone v1.0 \"Auth rework 1.0\" {\n  name \"Auth rework 1.0\"\n  workstream auth-rework\n}\n\nphase 01 \"Store\" {\n  milestone auth-rework/v1.0\n  name \"Store\"\n  slug \"store\"\n  goal \"The store is replaced\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    for name in ["v1.0", "auth-rework/v1.0", "auth-rework/v1.0/01", "v1.0/01"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [name], "query {name}: {}", query.all());
    }
}

#[test]
fn a_lesson_is_named_within_its_retrospective() {
    let p = project("roadmap");
    p.write(
        "spec/learning.spec",
        "retrospective v1.0-RETROSPECTIVE \"Taskboard 1.0 retrospective\" {\n  milestone v1.0\n  built [\"Add and complete tasks\"]\n}\n\nlesson L1 \"Plan the reload first\" {\n  retrospective v1.0-RETROSPECTIVE\n  text \"Persistence decides the data model\"\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    let query = p.run(&[
        "query",
        "v1.0/v1.0-RETROSPECTIVE/L1",
        "--depth",
        "0",
        "--format",
        "brief",
    ]);
    assert_eq!(
        node_ids(&query),
        ["v1.0/v1.0-RETROSPECTIVE/L1"],
        "{}",
        query.all()
    );
}
