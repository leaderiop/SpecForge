//! A judged rule (ADR 0290): a rule whose `check` is `judged` holds while a fresh verdict from its judge says so. It
//! is declared with a judge, outcomes and, per target kind, its inputs by selector; a rule that cannot work as
//! declared is W112 and is not registered.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::FieldType;
use specforge_protocol_types::ValidationRuleDescriptor;
use specforge_test_macros::test as spec;

use super::{Built, rules_of};
use crate::support::declare;

fn descriptor(judged: Value) -> ValidationRuleDescriptor {
    serde_json::from_value(json!({
        "code": "W311", "severity": "warning", "check": "judged",
        "message_template": "the check of {id} does not hold", "judged": judged,
    }))
    .unwrap()
}

/// The well-formed `judged` object: the plan check reads the plan, its phase and its tasks.
fn plan_check() -> Value {
    json!({
        "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
        "targets": [{"kind": "plan", "inputs": [
            {"name": "plan", "selector": "self"},
            {"name": "phase", "selector": "self.phase"},
            {"name": "body", "selector": "self<task.plan"},
            {"name": "instructions", "selector": "file(\"CLAUDE.md\")"},
        ]}],
    })
}

/// `@test` declaring phase, plan and task, and `rule`.
fn built(rule: ValidationRuleDescriptor) -> Built {
    let mut declaration = declare("@test", |c| {
        c.kind("phase", |k| {
            k.keyword("phase");
        });
        c.kind("plan", |k| {
            k.keyword("plan");
            k.field("phase", |f| {
                f.field_type(FieldType::Reference).target_kind("phase");
            });
            k.field("status", |f| {
                f.field_type(FieldType::String).computed();
            });
        });
        c.kind("task", |k| {
            k.keyword("task");
            k.field("plan", |f| {
                f.field_type(FieldType::Reference).target_kind("plan");
            });
        });
    });
    declaration.validation_rules = vec![rule];
    rules_of(vec![declaration])
}

#[ignore = "K7 T4"]
#[spec(
    behavior = "declare_judged_rule",
    verify = "a judged rule is declared with its judge, outcomes and per-target selectors"
)]
fn a_judged_rule_is_declared_with_selectors() {
    let built = built(descriptor(plan_check()));
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    let rule = built.rules.iter().next().expect("the rule is registered");
    assert_eq!(rule.describe()["check"], "judged");
    assert_eq!(rule.describe()["judged"]["judge"], "checker");
    assert_eq!(
        rule.describe()["judged"]["targets"][0]["inputs"][1]["selector"],
        "self.phase"
    );
}

#[ignore = "K7 T4"]
#[spec(
    behavior = "declare_judged_rule",
    verify = "a malformed judged rule is W112 and is not registered"
)]
fn a_malformed_judged_rule_is_w112() {
    let with = |edit: &dyn Fn(&mut Value)| {
        let mut judged = plan_check();
        edit(&mut judged);
        judged
    };
    let input = |judged: &mut Value, i: usize, selector: &str| {
        judged["targets"][0]["inputs"][i]["selector"] = json!(selector);
    };
    let malformed = [
        with(&|j| j["judge"] = json!("")),
        with(&|j| j["outcomes"] = json!([])),
        with(&|j| j["outcomes"] = json!(["passed", "passed"])),
        with(&|j| j["passing"] = json!(["missing"])),
        with(&|j| j["targets"][0]["kind"] = json!("nothing")),
        with(&|j| j["targets"][0]["inputs"][0]["name"] = json!("cited")),
        with(&|j| input(j, 1, "self.nothing")),
        with(&|j| input(j, 1, "self.")),
        with(&|j| input(j, 1, "self.status")),
        with(&|j| input(j, 3, "file(\"/etc/passwd\")")),
        with(&|j| input(j, 3, "file(\"../x\")")),
        with(&|j| input(j, 3, "file(\"src/*.rs\")")),
        with(&|j| input(j, 3, "file(self.phase)")),
        with(&|j| j["targets"][0]["unless_set"] = json!(["nothing"])),
        with(&|j| j["values"] = json!(["plan.phase"])),
    ];
    for judged in malformed {
        let built = built(descriptor(judged.clone()));
        assert_eq!(
            built.coded("W112").len(),
            1,
            "{judged}: {:?}",
            built.diagnostics
        );
        assert!(built.rules.is_empty(), "{judged}");
    }
    // `judged` on another check is W112 too; and a judged rule without it.
    let mut other = descriptor(plan_check());
    other.check = "no_edges".to_string();
    assert_eq!(built(other).coded("W112").len(), 1);
    let bare: ValidationRuleDescriptor = serde_json::from_value(json!({
        "code": "W311", "severity": "warning", "check": "judged", "message_template": "x",
    }))
    .unwrap();
    assert_eq!(built(bare).coded("W112").len(), 1);
}
