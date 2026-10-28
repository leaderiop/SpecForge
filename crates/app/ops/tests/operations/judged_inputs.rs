//! What an agent reads before it judges (ADR 0290): a judged rule's inputs for one entity, the `basis` a `set_verdict`
//! checks, and the judgements every read lists.

use serde_json::{Value, json};
use specforge_project::testing::{GSD_JUDGED, gsd_judged_project};
use specforge_test_macros::test as specforge_test;

const PLAN: &str = "v1.0/03/03-02";
const W311: &str = "gsd/W311";

/// The answer of `specforge query PLAN --inputs W311`, as JSON. K7 T10 calls `read::judged_inputs`.
fn inputs_json(
    project: &specforge_project::testing::TestProject,
    entity: &str,
    rule: &str,
) -> Value {
    let _ = (project, entity, rule);
    panic!("K7 T10: specforge_ops::read::judged_inputs")
}

/// The judgements of `entity` as `inspect` lists them. K7 T11.
fn judgements_of(project: &specforge_project::testing::TestProject, entity: &str) -> Value {
    let _ = (project, entity);
    panic!("K7 T11: EntityFacts.judgements")
}

#[ignore = "K7 T10"]
#[specforge_test(
    behavior = "answer_judged_inputs",
    verify = "the inputs query answers the basis a set_verdict checks"
)]
fn query_inputs_answers_the_basis_set_verdict_checks() {
    let project = gsd_judged_project("");
    let answer = inputs_json(&project, PLAN, W311);
    assert_eq!(answer["rule"], W311);
    assert_eq!(answer["judge"], "gsd-plan-checker");
    let names: Vec<&str> = answer["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names[0], "plan");
    assert!(answer["basis"]["cited"].is_string(), "{answer}");
    // A verdict recorded over the basis lands; with one input edited in between it is `stale_revision`.
    let _ = (GSD_JUDGED, json!(null));
    panic!("K7 T10: set_verdict over the basis, then over an edited input")
}

#[ignore = "K7 T11"]
#[specforge_test(
    behavior = "answer_judged_inputs",
    verify = "inspect, query, the graph export and a command's graph list each entity's judgements"
)]
fn inspect_export_and_the_command_graph_list_judgements() {
    let project = gsd_judged_project("");
    let listed = judgements_of(&project, PLAN);
    let rules: Vec<&str> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|j| j["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&W311), "{listed}");
    assert_eq!(listed[0]["state"], "missing");
}
