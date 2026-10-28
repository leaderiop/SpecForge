//! A judged rule whose judge is a prompt of its extension (ADR 0295): its verdict is recorded and goes stale as an
//! agent's does, only the extension's own command may record it, and the rendered prompt lists the rules it judges.
//! The fixture is `@gsd/core` of the judged tests, whose plan-check rules (`W311`...) are judged by the prompt
//! `plan_check` instead of the agent `gsd-plan-checker`.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::authoring::apply::{
    HostJudgedRules, JudgedRules, Origin, SystemClock, apply_plan_with,
};
use specforge_ops::extension::prompt::{ExtensionPrompts, PromptArgs, render};
use specforge_ops::read::judged::{InputsRequest, judged_inputs};
use specforge_ops::view::ProjectView;
use specforge_project::SharedRuntime;
use specforge_project::testing::{
    GSD_JUDGED, TestProject, declare_gsd_judged, gsd_judged_project, gsd_judged_rules,
};
use specforge_protocol_types::EditPlan;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

const PLAN: &str = "v1.0/03/03-02";
const W311: &str = "gsd/W311";

/// `@gsd/core`, its plan-check rules judged by the prompt `plan_check`, which another prompt `plain` does not
/// judge; `prompts` answers both.
fn runtime() -> Arc<InProcessRuntime> {
    let mut rules = gsd_judged_rules();
    for rule in rules.as_array_mut().unwrap() {
        if rule["judged"]["judge"] == "gsd-plan-checker" {
            rule["judged"]["judge"] = json!("prompt:plan_check");
        }
    }
    let said = |text: &str| {
        WasmCallResult::Ok(
            json!({"messages": [{"role": "user", "text": text}]})
                .to_string()
                .into_bytes(),
        )
    };
    Arc::new(
        InProcessRuntime::new()
            .with(move || {
                let mut c = specforge_extension_sdk::prelude::ContributionsBuilder::new(
                    specforge_extension_sdk::prelude::ExtensionMeta::new(GSD_JUDGED, "0.1.0"),
                );
                declare_gsd_judged(&mut c);
                specforge_project::testing::declare_rules(&mut c, rules.clone());
                c.raw_category(
                    specforge_extension_sdk::DeclaredCategory::Surfaces,
                    json!([{"prompts": [
                        {"id": "plan_check", "title": "Check", "description": "Check a plan", "export": "prompt__plan_check"},
                        {"id": "plain", "title": "Plain", "description": "Plain", "export": "prompt__plain"},
                    ]}]),
                );
                c
            })
            .answer_raw(GSD_JUDGED, "prompt__plan_check", said("check"))
            .answer_raw(GSD_JUDGED, "prompt__plain", said("plain")),
    )
}

fn compile(project: &TestProject) -> specforge_project::CompiledProject {
    project.compile_in(runtime() as SharedRuntime)
}

/// Apply a `set_verdict` of W311 on the plan, as a command of `extension`: the refusal reason or `applied`.
fn record(project: &TestProject, extension: &str, judge: &str) -> String {
    let compiled = compile(project);
    let view = ProjectView::of(&compiled);
    let basis = judged_inputs(
        &view,
        &InputsRequest {
            entity: PLAN,
            rule: W311,
            cited: &[],
        },
    )
    .unwrap_or_else(|r| panic!("{:?}", r.into_error()))
    .basis;
    let rules: Arc<dyn JudgedRules> = Arc::new(HostJudgedRules::of(&view));
    let plan: EditPlan = serde_json::from_value(json!({"ops": [
        {"op": "set_verdict", "entity": PLAN, "rule": W311, "outcome": "passed",
         "judge": judge, "model": "m", "findings": [], "cited": [], "basis": basis}]}))
    .unwrap();
    let origin = Origin::Command {
        extension: extension.to_string(),
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

fn state_of(project: &TestProject) -> String {
    let compiled = compile(project);
    let facts = specforge_ops::read::inspect::inspect(&ProjectView::of(&compiled), PLAN)
        .unwrap_or_else(|r| panic!("{:?}", r.into_error()));
    let judgements = serde_json::to_value(&facts.judgements).unwrap();
    judgements
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["rule"] == W311)
        .map(|j| j["state"].as_str().unwrap().to_string())
        .expect("a judgement of W311")
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a prompt-judged verdict is recorded and goes stale like an agent's, and its rendered prompt lists the rules it judges"
)]
fn a_prompt_judged_verdict_is_recorded() {
    let project = gsd_judged_project("");
    assert_eq!(state_of(&project), "missing");
    assert_eq!(record(&project, GSD_JUDGED, "prompt:plan_check"), "applied");
    let source = project.read("a.spec");
    assert!(
        source.contains("judge \"prompt:plan_check@0.1.0\""),
        "{source}"
    );
    assert_eq!(state_of(&project), "fresh");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a prompt-judged verdict is recorded and goes stale like an agent's, and its rendered prompt lists the rules it judges"
)]
fn a_verdict_from_another_extensions_command_is_refused() {
    let project = gsd_judged_project("");
    assert_ne!(
        record(&project, "@other/ext", "prompt:plan_check"),
        "applied"
    );
    // And the judge is the rule's, as an agent's is.
    assert_eq!(record(&project, GSD_JUDGED, "gsd-plan-checker"), "judge");
    assert_eq!(state_of(&project), "missing");
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a prompt-judged verdict is recorded and goes stale like an agent's, and its rendered prompt lists the rules it judges"
)]
fn a_prompt_judged_verdict_goes_stale_like_an_agents() {
    let project = gsd_judged_project("");
    assert_eq!(record(&project, GSD_JUDGED, "prompt:plan_check"), "applied");
    assert_eq!(state_of(&project), "fresh");
    // An input of the plan changes (the instructions file the rule reads).
    project.write("CLAUDE.md", "# Instructions, changed\n");
    assert_eq!(state_of(&project), "stale");
}

fn rendered(project: &TestProject, id: &str) -> Value {
    let compiled = compile(project);
    let view = ProjectView::of(&compiled);
    let prompts = ExtensionPrompts::build(&view);
    let prompt = prompts
        .all()
        .iter()
        .find(|p| p.mcp_name().ends_with(&format!(".{id}")))
        .unwrap_or_else(|| panic!("no prompt {id}"));
    let rendered = render(
        &view,
        prompt,
        PromptArgs::Named(&serde_json::Map::new()),
        None,
    )
    .unwrap_or_else(|_| panic!("{id} did not render"));
    serde_json::to_value(rendered.payload).unwrap()
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a prompt-judged verdict is recorded and goes stale like an agent's, and its rendered prompt lists the rules it judges"
)]
fn a_rendered_prompt_lists_the_rules_it_judges() {
    let payload = rendered(&gsd_judged_project(""), "plan_check");
    let judged = payload["judged"].as_array().expect("a judged list");
    let codes: Vec<&str> = judged.iter().map(|r| r["rule"].as_str().unwrap()).collect();
    assert!(codes.contains(&W311), "{payload}");
    let w311 = judged.iter().find(|r| r["rule"] == W311).unwrap();
    assert_eq!(w311["outcomes"], json!(["passed", "issues"]));
    assert_eq!(w311["passing"], json!(["passed"]));
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a prompt-judged verdict is recorded and goes stale like an agent's, and its rendered prompt lists the rules it judges"
)]
fn a_prompt_with_no_judged_rule_has_no_judged_key() {
    let payload = rendered(&gsd_judged_project(""), "plain");
    assert!(payload.get("judged").is_none(), "{payload}");
}
