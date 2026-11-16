//! A judged rule (ADR 0290): a rule whose `check` is `judged` holds while a fresh verdict from its judge says so. It
//! is declared with a judge, outcomes and, per target kind, its inputs by selector; a rule that cannot work as
//! declared is W112 and is not registered.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::{FieldType, Vocabulary};
use specforge_protocol_types::ValidationRuleDescriptor;
use specforge_test_macros::test as spec;

use super::{Built, rules_of, rules_with};
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

/// `@test` declaring a `plan` that keeps `complete_when` on its computed `status` and binds a verdict field, a `task`
/// scoped by it and a `phase` with no owner state, and one judged rule over `targets` binding `outcome_fields`.
fn owner_built(targets: Value, outcome_fields: Value) -> Built {
    let mut declaration = declare("@test", |c| {
        c.agent("checker", |a| {
            a.description("Judges");
        });
        c.kind("phase", |k| {
            k.keyword("phase");
        });
        c.kind("plan", |k| {
            k.keyword("plan")
                .complete_when("status", &["done"])
                .field("status", |f| {
                    f.vocabulary(&Vocabulary::new(&["draft", "done"]))
                        .computed();
                })
                .field("check", |f| {
                    f.vocabulary(&Vocabulary::new(&["passed", "issues"]))
                        .verdict();
                });
        });
        c.kind("task", |k| {
            k.keyword("task").scoped_by("plan").field("plan", |f| {
                f.field_type(FieldType::Reference).target_kind("plan");
            });
        });
    });
    let mut judged = json!({"judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
                            "targets": targets});
    judged["outcome_fields"] = outcome_fields;
    declaration.validation_rules = vec![descriptor(judged)];
    rules_of(vec![declaration])
}

/// A judged target of `kind` reading itself, judged in `judged_while`.
fn waiting(kind: &str, judged_while: &str) -> Value {
    json!({"kind": kind, "judged_while": judged_while, "inputs": [{"name": "it", "selector": "self"}]})
}

#[spec(
    behavior = "declare_judged_rule",
    verify = "a judged target waits for an owner state its kind has, and binds no verdict value of that kind"
)]
fn a_judged_target_waits_for_an_owner_state_its_kind_has() {
    let built = owner_built(
        json!([waiting("plan", "complete"), waiting("task", "pending")]),
        json!([]),
    );
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    let rule = built.rules.iter().next().expect("registered");
    let targets = rule.describe()["judged"]["targets"].clone();
    assert_eq!(targets[0]["judged_while"], "complete", "{targets}");
    assert_eq!(targets[1]["judged_while"], "pending", "{targets}");
    for (targets, outcome_fields, why) in [
        (
            json!([waiting("plan", "later")]),
            json!([]),
            "neither 'complete' nor 'pending'",
        ),
        (
            json!([waiting("phase", "complete")]),
            json!([]),
            "declares complete_when",
        ),
        (
            json!([waiting("plan", "complete")]),
            json!(["plan.check"]),
            "'plan.check' is given before them",
        ),
    ] {
        let built = owner_built(targets.clone(), outcome_fields);
        let w112 = built.coded("W112");
        assert_eq!(w112.len(), 1, "{targets}: {:?}", built.diagnostics);
        assert!(
            w112[0].message.contains(why),
            "{targets}: {}",
            w112[0].message
        );
        assert!(built.rules.is_empty(), "{targets}");
    }
    // a verdict field of a kind no target waits on stays bound
    let plain = json!({"kind": "plan", "inputs": [{"name": "it", "selector": "self"}]});
    let built = owner_built(
        json!([waiting("task", "complete"), plain]),
        json!(["plan.check"]),
    );
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
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

/// `@test` declaring an audit (a `milestone` reference and a `path` string), a phase of a milestone naming
/// requirements, and a rule whose audit input is `selector`.
fn audit_built(selector: &str) -> Built {
    let mut declaration = declare("@test", |c| {
        c.agent("checker", |a| {
            a.description("Judges");
        });
        for keyword in ["milestone", "requirement"] {
            c.kind(keyword, |k| {
                k.keyword(keyword);
            });
        }
        c.kind("phase", |k| {
            k.keyword("phase");
            k.field("milestone", |f| {
                f.field_type(FieldType::Reference).target_kind("milestone");
            });
            k.field("requirements", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("requirement");
            });
        });
        c.kind("audit", |k| {
            k.keyword("audit");
            k.field("milestone", |f| {
                f.field_type(FieldType::Reference).target_kind("milestone");
            });
            k.field("path", |f| {
                f.field_type(FieldType::String);
            });
        });
    });
    let judged = json!({
        "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
        "targets": [{"kind": "audit", "inputs": [{"name": "input", "selector": selector}]}],
    });
    declaration.validation_rules = vec![descriptor(judged)];
    rules_of(vec![declaration])
}

#[spec(
    behavior = "digest_judged_inputs",
    verify = "a referrer selector goes on past its reference"
)]
fn a_referrer_selector_goes_on_past_its_reference() {
    let built = audit_built("self.milestone<phase.milestone.requirements");
    assert!(built.coded("W112").is_empty(), "{:?}", built.diagnostics);
    assert_eq!(built.rules.iter().count(), 1);
}

#[spec(
    behavior = "digest_judged_inputs",
    verify = "a file input may name a path held in a string field"
)]
fn file_of_a_string_field_is_admitted() {
    let built = audit_built("file(self.path)");
    assert!(built.coded("W112").is_empty(), "{:?}", built.diagnostics);
    assert_eq!(built.rules.iter().count(), 1);
    // A field that is no string is still no file field.
    let built = audit_built("file(self.milestone)");
    assert_eq!(built.coded("W112").len(), 1, "{:?}", built.diagnostics);
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

#[spec(
    behavior = "declare_judged_rule",
    verify = "a chain into an unloaded optional peer's kind is inert, and into any other unloaded kind W112"
)]
fn a_chain_into_an_unloaded_optional_peers_kind_is_inert() {
    let judged = json!({
        "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
        "targets": [{"kind": "plan", "inputs": [
            {"name": "tests", "selector": "test(self, self.proof)"},
            {"name": "proved", "selector": "self.proof"},
        ]}],
    });
    // `@ghosts`, which the host could enable, declares `ghost`.
    let ghosts = crate::support::candidates(&[declare("@ghosts", |c| {
        c.kind("ghost", |_| {});
    })]);
    let built = |peer: bool, target: &str| {
        let mut declaration = declare("@test", |c| {
            if peer {
                c.optional_peer("@ghosts", "^1.0");
            }
            c.agent("checker", |a| {
                a.description("Judges");
            });
            c.kind("plan", |k| {
                k.keyword("plan");
                k.field("proof", |f| {
                    f.field_type(FieldType::ReferenceList).target_kind(target);
                });
            });
        });
        declaration.validation_rules = vec![descriptor(judged.clone())];
        rules_with(vec![declaration], ghosts.clone())
    };
    let inert = built(true, "ghost");
    assert!(inert.coded("W112").is_empty(), "{:?}", inert.diagnostics);
    assert_eq!(inert.rules.iter().count(), 1);
    // No optional peer, or a kind the peer does not declare (a typo): the rule can never work.
    for (peer, target) in [(false, "ghost"), (true, "gost")] {
        let refused = built(peer, target);
        assert_eq!(refused.coded("W112").len(), 1, "{:?}", refused.diagnostics);
        assert_eq!(refused.rules.iter().count(), 0);
    }
}

#[spec(
    behavior = "declare_judged_rule",
    verify = "a file() of a chain into an unloaded optional peer's kind is inert, and of any other unloaded kind W112"
)]
fn a_file_of_a_chain_into_an_unloaded_optional_peers_kind_is_inert() {
    // `@ghosts`, which the host could enable, declares `ghost`.
    let ghosts = crate::support::candidates(&[declare("@ghosts", |c| {
        c.kind("ghost", |_| {});
    })]);
    let built = |selector: &str, peer: bool, target: &str| {
        let judged = json!({
            "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
            "targets": [{"kind": "plan", "inputs": [{"name": "files", "selector": selector}]}],
        });
        let mut declaration = declare("@test", |c| {
            if peer {
                c.optional_peer("@ghosts", "^1.0");
            }
            c.agent("checker", |a| {
                a.description("Judges");
            });
            c.kind("plan", |k| {
                k.keyword("plan");
                k.field("proof", |f| {
                    f.field_type(FieldType::ReferenceList).target_kind(target);
                });
            });
        });
        declaration.validation_rules = vec![descriptor(judged)];
        rules_with(vec![declaration], ghosts.clone())
    };
    // Through the reference into `ghost`, and from `ghost` referrers.
    for selector in ["file(self.proof.file)", "file(self<ghost.plan.file)"] {
        let inert = built(selector, true, "ghost");
        assert!(
            inert.coded("W112").is_empty(),
            "{selector}: {:?}",
            inert.diagnostics
        );
        assert_eq!(inert.rules.iter().count(), 1, "{selector}");
    }
    // No optional peer, or a kind the peer does not declare (a typo): the rule can never work.
    for (selector, peer, target) in [
        ("file(self.proof.file)", false, "ghost"),
        ("file(self.proof.file)", true, "gost"),
        ("file(self<ghost.plan.file)", false, "ghost"),
        ("file(self<gost.plan.file)", true, "ghost"),
    ] {
        let refused = built(selector, peer, target);
        assert_eq!(
            refused.coded("W112").len(),
            1,
            "{selector}: {:?}",
            refused.diagnostics
        );
        assert_eq!(refused.rules.iter().count(), 0, "{selector}");
    }
}

/// `@test` declaring a phase and a decision of a phase with an `origin` enum, a computed `status` and a `related`
/// reference list, and a rule over the phase whose input is `selector`.
fn filter_built(selector: &str) -> Built {
    use specforge_extension_sdk::prelude::Vocabulary;
    const ORIGIN: Vocabulary = Vocabulary::new(&["context", "summary"]);
    let mut declaration = declare("@test", |c| {
        c.agent("checker", |a| {
            a.description("Judges");
        });
        c.kind("phase", |k| {
            k.keyword("phase");
        });
        c.kind("decision", |k| {
            k.keyword("decision");
            k.field("phase", |f| {
                f.field_type(FieldType::Reference).target_kind("phase");
            });
            k.field("origin", |f| {
                f.vocabulary(&ORIGIN);
            });
            k.field("status", |f| {
                f.field_type(FieldType::String).computed();
            });
            k.field("related", |f| {
                f.field_type(FieldType::ReferenceList).target_kind("phase");
            });
        });
    });
    let judged = json!({
        "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
        "targets": [{"kind": "phase", "inputs": [{"name": "input", "selector": selector}]}],
    });
    declaration.validation_rules = vec![descriptor(judged)];
    rules_of(vec![declaration])
}

#[spec(
    behavior = "digest_judged_inputs",
    verify = "a selector's kind filter names an authored scalar field of the kind and values it declares"
)]
fn a_kind_filter_names_an_authored_scalar_field_and_declared_values() {
    for selector in [
        "self<decision[origin=context].phase",
        "self<decision[origin=context|summary].phase",
        "decision[origin=summary]",
    ] {
        let built = filter_built(selector);
        assert!(
            built.coded("W112").is_empty(),
            "{selector}: {:?}",
            built.diagnostics
        );
        assert_eq!(built.rules.iter().count(), 1, "{selector}");
    }
    for (selector, why) in [
        (
            "self<decision[kind=context].phase",
            "declares no field 'kind'",
        ),
        (
            "self<decision[status=done].phase",
            "computed or written by a verdict",
        ),
        (
            "self<decision[related=x].phase",
            "not a string, enum, bool or integer",
        ),
        (
            "self<decision[origin=checkpoint].phase",
            "'checkpoint' is no value",
        ),
    ] {
        let built = filter_built(selector);
        let w112 = built.coded("W112");
        assert_eq!(w112.len(), 1, "{selector}: {:?}", built.diagnostics);
        assert!(
            w112[0].message.contains(why),
            "{selector}: {}",
            w112[0].message
        );
        assert_eq!(built.rules.iter().count(), 0, "{selector}");
    }
}
