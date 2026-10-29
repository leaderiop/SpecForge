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
    project("plan").check_exact(&[]);
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
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\nrequirement SHARE-02 \"Share a board\" {\n  text \"A board is shared by link\"\n  category \"Sharing\"\n  tier v2\n}\n\nphase 01 \"Sharing\" {\n  milestone v2.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"Boards are shared\"\n  requirements [SHARE-02]\n  success_criteria [\"a\", \"b\"]\n}\n\nplan 01-01 \"Share link\" {\n  phase v2.0/01\n  type execute\n  files_modified [\"src/share.js\"]\n  requirements [SHARE-02]\n  objective \"A board has a share link\"\n}\n\ntruth TR1 \"A link\" {\n  plan v2.0/01/01-01\n  text \"A board has a link\"\n}\n\nartifact AR1 \"Link\" {\n  plan v2.0/01/01-01\n  path \"src/share.js\"\n  provides \"share()\"\n}\n",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    for name in ["v1.0/01/01-01", "v2.0/01/01-01"] {
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
    // 01-02 follows 01-01, so the exemption covers nothing (gsd/I321, an advisory).
    assert_eq!(p.codes(), ["gsd/I321"]);
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

#[test]
fn must_haves_are_named_within_their_plan() {
    let p = project("plan");
    for name in [
        "v1.0/01/01-01/TR1",
        "v1.0/02/02-01/TR1",
        "v1.0/01/01-01/AR1",
        "v1.0/01/01-02/KL1",
        "v1.0/02/02-01/PR1",
        "261028-a1b/TR1",
    ] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [name], "query {name}: {}", query.all());
    }
}

#[test]
fn an_artifact_the_plan_creates_is_not_e016() {
    let p = project("plan");
    assert!(
        p.read("spec/plans.spec")
            .contains("path     \"src/tasks.js\"")
    );
    assert!(!p.exists("src/tasks.js"));
    assert_eq!(p.codes(), Vec::<String>::new());
}

/// The first `spec` block of "What it looks like", with the phase, requirements and sibling plan it names.
#[test]
fn the_readme_example_checks_clean() {
    let readme = include_str!("../../spec/README.md");
    let block = readme
        .split("## What it looks like")
        .nth(1)
        .and_then(|rest| rest.split("```spec\n").nth(1))
        .and_then(|rest| rest.split("\n```").next())
        .expect("the example block");
    let p = project("plan");
    p.write(
        "spec/auth.spec",
        "requirement AUTH-02 \"Sessions\" {\n  text \"A user has a session\"\n  category \"Auth\"\n  tier v1\n}\n\nrequirement AUTH-03 \"Rotation\" {\n  text \"A session token rotates\"\n  category \"Auth\"\n  tier v1\n}\n\nphase 03 \"Auth\" {\n  milestone v1.0\n  name \"Auth\"\n  slug \"auth\"\n  goal \"Users sign in\"\n  depends_on [02]\n  requirements [AUTH-02, AUTH-03]\n  success_criteria [\"a\", \"b\"]\n}\n\nplan 03-01 \"Sign-in\" {\n  phase 03\n  type execute\n  files_modified [\"src/auth/signin.ts\"]\n  requirements [AUTH-02]\n  objective \"Users sign in\"\n}\n\ntruth TR1 \"A user signs in\" {\n  plan 03-01\n  text \"A user signs in\"\n}\n\nartifact AR1 \"Sign-in\" {\n  plan 03-01\n  path \"src/auth/signin.ts\"\n  provides \"sign-in\"\n}\n",
    );
    p.write("spec/readme.spec", &format!("{block}\n"));
    p.check_exact(&[]);
}

const DISPOSITION: &str = "disposition DSP1 \"Name the file format\" {\n  plan     v1.0/02/02-01\n  round    1\n  reviewer \"codex\"\n  feedback \"The plan does not say which file format the store writes\"\n  outcome  addressed\n}\n";

#[test]
fn a_rewritten_disposition_is_refused() {
    let p = project("plan");
    p.commit();
    p.edit("spec/plans.spec", "outcome  addressed", "outcome  deferred");
    assert_eq!(p.codes(), ["E084"]);
}

#[test]
fn a_removed_disposition_is_e085() {
    let p = project("plan");
    p.commit();
    p.edit("spec/plans.spec", DISPOSITION, "");
    assert_eq!(p.codes(), ["E085"]);
}

#[test]
fn an_appended_disposition_is_clean() {
    let p = project("plan");
    p.commit();
    p.edit(
        "spec/plans.spec",
        DISPOSITION,
        &format!(
            "{DISPOSITION}\ndisposition DSP2 \"Pin the path\" {{\n  plan     v1.0/02/02-01\n  round    2\n  reviewer \"codex\"\n  feedback \"The data directory is not pinned\"\n  outcome  deferred\n  reason   \"Decided in phase 3\"\n}}\n"
        ),
    );
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_hand_written_setup_done_is_e083() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "local_dev [\"mkdir -p data\"]\n",
        "local_dev [\"mkdir -p data\"]\n  done      { at \"2026-10-28T09:30:00Z\" by \"me\" }\n",
    );
    assert_eq!(p.codes(), ["E083"]);
}

#[test]
fn waves_follow_phase_plan_index() {
    let p = project("plan");
    let second = fields_of(&p, "v1.0/01/01-02");
    assert_eq!(
        (&second["wave"], &second["autonomous"]),
        (&2.into(), &false.into()),
        "{second}"
    );
    let first = fields_of(&p, "v1.0/01/01-01");
    assert_eq!(
        (&first["wave"], &first["autonomous"]),
        (&1.into(), &true.into()),
        "{first}"
    );
    // Phase 02's plan depends on nothing and is a root of its own phase.
    assert_eq!(fields_of(&p, "v1.0/02/02-01")["wave"], 1);
}

#[test]
fn a_quick_plan_is_autonomous_and_has_no_wave() {
    let p = project("plan");
    let quick = fields_of(&p, "261028-a1b");
    assert_eq!(quick["autonomous"], true, "{quick}");
    assert!(quick.get("wave").is_none(), "{quick}");
}

#[test]
fn a_dependency_on_another_phase_leaves_the_wave_alone() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  files_modified [\"src/store.js\"]\n",
        "  depends_on     [v1.0/01/01-02]\n  files_modified [\"src/store.js\"]\n",
    );
    assert_eq!(fields_of(&p, "v1.0/02/02-01")["wave"], 1);
}
