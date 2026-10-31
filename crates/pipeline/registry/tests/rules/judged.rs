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
    built_all(vec![rule])
}

/// [`built`] declaring each of `rules`, in order.
fn built_all(rules: Vec<ValidationRuleDescriptor>) -> Built {
    rules_of(vec![declaration_of(rules)])
}

/// The `@test` declaration of [`built_all`].
fn declaration_of(
    rules: Vec<ValidationRuleDescriptor>,
) -> specforge_protocol_types::ExtensionDeclaration {
    let mut declaration = declare("@test", |c| {
        c.agent("checker", |a| {
            a.description("Judges");
        });
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
    declaration.validation_rules = rules;
    declaration
}

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

#[spec(
    behavior = "declare_judged_rule",
    verify = "no other rule of the extension reports a judged rules code"
)]
fn a_declarative_rule_cannot_share_a_judged_rules_code() {
    let declarative: ValidationRuleDescriptor = serde_json::from_value(json!({
        "code": "W311", "severity": "warning", "check": "no_edges", "message_template": "x",
    }))
    .unwrap();
    // Before or after the judged rule, the declarative one is not registered.
    for rules in [
        vec![declarative.clone(), descriptor(plan_check())],
        vec![descriptor(plan_check()), declarative],
    ] {
        let built = built_all(rules);
        assert_eq!(built.coded("W112").len(), 1, "{:?}", built.diagnostics);
        assert_eq!(built.rules.iter().count(), 1);
        assert_eq!(
            built.rules.iter().next().unwrap().describe()["check"],
            "judged"
        );
    }
}

#[spec(
    behavior = "declare_judged_rule",
    verify = "a guard pass cannot report a judged rule's code"
)]
#[ignore = "pins K20a; T3 un-ignores"]
fn a_guard_pass_cannot_report_a_judged_rules_code() {
    let mut declaration = declaration_of(vec![descriptor(plan_check())]);
    declaration.passes = declare("@test", |c| {
        c.pass("g", |p| {
            p.guard().reports(&["W311"]).run(|_| Vec::new());
        });
    })
    .passes;
    let built = rules_of(vec![declaration]);
    let w021: Vec<_> = built
        .build
        .diagnostics()
        .filter(|d| d.code == "W021" && d.message.contains("'g'") && d.message.contains("'W311'"))
        .collect();
    assert_eq!(w021.len(), 1, "{:?}", built.diagnostics);
}

/// The build of `@test` declaring a plan with verdict-written `result` and `owner` fields, and `rule`.
fn with_result_field(
    rule: ValidationRuleDescriptor,
) -> specforge_registry::compilation::RegistryBuild {
    let mut declaration = declare("@test", |c| {
        c.agent("checker", |a| {
            a.description("Judges");
        });
        c.kind("plan", |k| {
            k.keyword("plan");
            k.field("result", |f| {
                f.field_type(FieldType::String).verdict();
            });
            k.field("owner", |f| {
                f.field_type(FieldType::String).verdict().required();
            });
        });
    });
    declaration.validation_rules = vec![rule];
    crate::support::build([declaration])
}

#[spec(
    behavior = "declare_judged_rule",
    verify = "a verdict-written field is set by exactly one judged rule, and is never required"
)]
fn a_verdict_field_is_bound_by_one_judged_rule() {
    let bound = |values: Value| {
        let mut judged = json!({
            "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
            "targets": [{"kind": "plan", "inputs": [{"name": "plan", "selector": "self"}]}],
        });
        judged["values"] = values;
        judged
    };
    let with = with_result_field(descriptor(bound(json!(["plan.result"]))));
    let w021: Vec<_> = with.diagnostics().filter(|d| d.code == "W021").collect();
    // `owner` is verdict-written and `required`, and bound by no rule; `result` is bound.
    assert!(
        w021.iter().any(|d| d.message.contains("'plan.owner'")),
        "{w021:?}"
    );
    assert!(
        !w021.iter().any(|d| d.message.contains("'plan.result'")),
        "{w021:?}"
    );
    assert_eq!(with.judged_rules().count(), 1);
    assert!(with.judged_rule("test/W311").is_some());
    let unbound = with_result_field(descriptor(bound(json!([]))));
    assert!(
        unbound
            .diagnostics()
            .any(|d| d.code == "W021" && d.message.contains("'plan.result'"))
    );
}
