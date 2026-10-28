//! `set_verdict`, an edit plan op checked through the judged-rules port (ADR 0289, D6). The host has no judged
//! rule until K7: every `set_verdict` is `not_judged`.

use std::sync::Arc;

use serde_json::{Map, Value, json};
use specforge_ops::authoring::apply::testing::FakeJudgedRules;
use specforge_ops::authoring::apply::{JudgedRules, PlanOutcome, SystemClock, execute_with};
use specforge_ops::extension::command::{CommandFormat, ExtensionCommands};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, TestProject};
use specforge_test_macros::test as specforge_test;

use crate::plans::{compiled, project, refused, serving};

fn verdict_of(
    entity: &str,
    rule: &str,
    outcome: &str,
    judge: &str,
    cited: &[&str],
    basis: Value,
) -> Value {
    let cited: Vec<Value> = cited.iter().map(|c| json!({"entity": c})).collect();
    json!([{"op": "set_verdict", "entity": entity, "rule": rule, "outcome": outcome,
        "judge": judge, "model": "m", "findings": [], "cited": cited, "basis": basis}])
}

/// A verdict on `V`, the verification the fake rule judges.
fn verdict(rule: &str, outcome: &str, judge: &str, cited: &[&str], basis: Value) -> Value {
    verdict_of("v1.0/03/V", rule, outcome, judge, cited, basis)
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict is refused without a judged rule"
)]
fn set_verdict_is_refused_without_a_judged_rule() {
    let p = project();
    let ops = verdict("quality", "pass", "judge", &[], json!({}));
    assert_eq!(
        refused(&p, CORE, "core", "record", ops).as_deref(),
        Some("not_judged")
    );
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict checks the rule and its basis"
)]
fn set_verdict_checks_the_rule_and_its_basis() {
    // The fake rule `quality` targets `verification`, declares outcomes `pass` and `fail`, and belongs to
    // `@t/core`'s judge `judge`; `with_rules` gives the host this port.
    let p = project();
    let reasons = |ops: Value| with_rules(&p, "record", ops);
    assert_eq!(
        reasons(verdict_of(
            "v1.0/03/U1",
            "quality",
            "pass",
            "judge",
            &[],
            json!({})
        )),
        "kind"
    );
    assert_eq!(
        reasons(verdict("quality", "meh", "judge", &[], json!({}))),
        "outcome"
    );
    assert_eq!(
        reasons(verdict("quality", "pass", "other", &[], json!({}))),
        "judge"
    );
    assert_eq!(
        reasons(verdict("quality", "pass", "judge", &["nowhere"], json!({}))),
        "cite"
    );
    assert_eq!(
        reasons(verdict(
            "quality",
            "pass",
            "judge",
            &[],
            json!({"cited": "sha256:0"})
        )),
        "stale_revision"
    );
    // Judged over the digests the rule computes now, it is planned.
    assert!(
        reasons(verdict(
            "quality",
            "pass",
            "judge",
            &[],
            json!({"cited": "sha256:1"})
        ))
        .starts_with("not refused"),
    );
}

// The fake host writes no verdict member, so the pin flips with the host's own port (K7 T9); the ledger's half, that a
// verdict is a recorded value, is pinned in `harness::rewrite` (K7 T5).
#[ignore = "K7 T9"]
#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict writes its verdict and its ledger line"
)]
fn set_verdict_writes_its_verdict_and_its_ledger_line() {
    let p = project();
    let ops = verdict(
        "quality",
        "pass",
        "judge",
        &[],
        json!({"cited": "sha256:1"}),
    );
    let outcome = run_with_rules(&p, "record", ops, false, || {});
    assert!(outcome.starts_with("not refused"), "{outcome}");
    let spec = std::fs::read_to_string(p.root().join("03-02.spec")).unwrap();
    assert!(spec.contains("verdict quality pass {"), "{spec}");
    let ledger = std::fs::read_to_string(p.root().join("specforge-harness")).unwrap_or_default();
    assert!(ledger.contains(" verdict:quality "), "{ledger}");
}

/// The host with the fake judged rule: the refusal reason of `ops` (the code when it has none) answered by command
/// `command` of `@t/core`.
fn with_rules(project: &TestProject, command: &str, ops: Value) -> String {
    run_with_rules(project, command, ops, true, || {})
}

/// [`with_rules`], applied unless `dry_run`; `landed` runs after the compile, before the plan is applied.
fn run_with_rules(
    project: &TestProject,
    command: &str,
    ops: Value,
    dry_run: bool,
    landed: impl FnOnce(),
) -> String {
    let runtime = serving(&[(CORE, &format!("cmd__{command}"), 0, ops)]);
    let compiled = compiled(project, &runtime);
    landed();
    let view = ProjectView::of(&compiled);
    let commands = ExtensionCommands::build(view.registries());
    let command = commands
        .of("core")
        .find(|c| c.id() == command)
        .expect("the command");
    let rules: Arc<dyn JudgedRules> = Arc::new(FakeJudgedRules);
    let run = execute_with(
        &view,
        command,
        &Map::new(),
        CommandFormat::Json,
        dry_run,
        specforge_ops::testing::caller(),
        &rules,
        &SystemClock,
    )
    .unwrap();
    match run.plan {
        Some(PlanOutcome::Refused(error)) => {
            let data = error.data.map_or(Value::Null, |data| *data);
            data["reason"]
                .as_str()
                .map_or_else(|| error.code.to_string(), str::to_string)
        }
        other => format!("not refused: {other:?}"),
    }
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict is refused on a human-only kind and for a computed value"
)]
fn set_verdict_is_refused_on_a_human_only_kind_and_for_a_computed_value() {
    let p = project().file("o.spec", "override O1 \"O\" {\n  phase v1.0/03\n}\n");
    let on_override = verdict_of(
        "O1",
        "person",
        "pass",
        "judge",
        &[],
        json!({"cited": "sha256:1"}),
    );
    assert_eq!(with_rules(&p, "record", on_override), "human_only");
    let mut computed = verdict(
        "computed",
        "pass",
        "judge",
        &[],
        json!({"cited": "sha256:1"}),
    );
    computed[0]["values"] = json!({"status": "passed"});
    assert_eq!(with_rules(&p, "record", computed), "computed_field");
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict compares its inputs with the disk under the lock"
)]
fn set_verdict_compares_its_inputs_with_the_disk_under_the_lock() {
    let p = project();
    let ops = verdict(
        "quality",
        "pass",
        "judge",
        &[],
        json!({"cited": "sha256:1"}),
    );
    // The inputs were as judged at the compile; a writer lands an entity they depend on before the write.
    let reason = run_with_rules(&p, "record", ops, false, || {
        p.write("z.spec", "verification Z \"Z\" {\n  phase 03\n}\n");
    });
    assert_eq!(reason, "stale_revision");
}

/// The host's judged rules over the GSD-shaped project (`specforge_project::testing::gsd_judged_project`): K7 T9.
fn host_rules() -> Arc<dyn JudgedRules> {
    panic!("K7 T9: HostJudgedRules")
}

#[ignore = "K7 T9"]
#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict refuses a value its field refuses"
)]
fn set_verdict_refuses_a_value_its_field_refuses() {
    // E202 binds `phase_spec.scores`, whose members are integers from 0 to 100: 140 is refused, `value`.
    let rules = host_rules();
    let rule = rules.rule("gsd/E202").expect("the host judges E202");
    assert!(rule.values.contains(&"scores".to_string()));
    let ops = json!([{"op": "set_verdict", "entity": "v1.0/03/S", "rule": "gsd/E202", "outcome": "passed",
        "judge": "gsd-spec-agent", "model": "m", "findings": [], "cited": [], "basis": {},
        "values": {"scores": {"goal": 140}}}]);
    let p = specforge_project::testing::gsd_judged_project("");
    assert_eq!(with_rules(&p, "record", ops), "value");
}
