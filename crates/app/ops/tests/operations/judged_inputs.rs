//! What an agent reads before it judges (ADR 0290): a judged rule's inputs for one entity, the `basis` a `set_verdict`
//! checks, and the judgements every read lists.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::authoring::apply::{
    HostJudgedRules, JudgedRules, Origin, SystemClock, apply_plan_with,
};
use specforge_ops::read::judged::{InputsRequest, judged_inputs};
use specforge_ops::view::ProjectView;
use specforge_project::SharedRuntime;
use specforge_project::testing::{GSD_JUDGED, gsd_judged_project, gsd_judged_runtime};
use specforge_protocol_types::EditPlan;
use specforge_test_macros::test as specforge_test;

const PLAN: &str = "v1.0/03/03-02";
const W311: &str = "gsd/W311";

/// The answer of `specforge query PLAN --inputs W311`, as JSON.
fn inputs_json(
    project: &specforge_project::testing::TestProject,
    entity: &str,
    rule: &str,
) -> Value {
    let compiled = project.compile_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
    let view = ProjectView::of(&compiled);
    let request = InputsRequest {
        entity,
        rule,
        cited: &[],
    };
    let answer = judged_inputs(&view, &request).unwrap_or_else(|r| panic!("{:?}", r.into_error()));
    serde_json::to_value(answer).unwrap()
}

/// Apply a `set_verdict` of W311 on the plan over `basis`: the refusal reason or `applied`.
fn record(project: &specforge_project::testing::TestProject, basis: &Value) -> String {
    let compiled = project.compile_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
    let view = ProjectView::of(&compiled);
    let rules: Arc<dyn JudgedRules> = Arc::new(HostJudgedRules::of(&view));
    let plan: EditPlan = serde_json::from_value(json!({"ops": [
        {"op": "set_verdict", "entity": PLAN, "rule": W311, "outcome": "passed",
         "judge": "gsd-plan-checker", "model": "m", "findings": [], "cited": [], "basis": basis}]}))
    .unwrap();
    let origin = Origin::Command {
        extension: GSD_JUDGED.to_string(),
    };
    match apply_plan_with(
        &view,
        &plan,
        &origin,
        false,
        specforge_ops::testing::caller(),
        &rules,
        &SystemClock,
    ) {
        Ok(_) => "applied".to_string(),
        Err(refusal) => {
            let error = refusal.into_error();
            let data = error.data.map_or(Value::Null, |data| *data);
            data["reason"]
                .as_str()
                .map_or_else(|| error.code.to_string(), str::to_string)
        }
    }
}

/// The judgements of `entity` as `inspect` lists them. K7 T11.
fn judgements_of(project: &specforge_project::testing::TestProject, entity: &str) -> Value {
    let _ = (project, entity);
    panic!("K7 T11: EntityFacts.judgements")
}

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
    assert_eq!(record(&project, &answer["basis"]), "applied");
    // One input differing from what the host reads now is a stale basis.
    let mut edited = answer["basis"].clone();
    edited["plan"] = json!("sha256:00");
    assert_eq!(record(&project, &edited), "stale_revision");
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
