//! `registry_build_rules`: the build collects every extension's rules,
//! ordered by code, each with the extension that declared it, then a
//! host-generated E006 rule for every required field.

use specforge_common::Severity;
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::validation_engine::ValidationPatternKind;
use specforge_test_macros::test as spec;

use crate::support::{build, coded, declare, diagnostics, rule_codes, software};

/// An extension `name` declaring warning rules `(code, message, check)`.
fn rules(name: &str, declared: &[(&str, &str, CheckKind)]) -> ExtensionDeclaration {
    declare(name, |c| {
        for (code, message, check) in declared {
            c.rule(code, |r| {
                r.severity(ValidationSeverity::Warning)
                    .message_template(message)
                    .check(*check);
            });
        }
    })
}

/// The build's rules as (code, message template), in order.
fn templates(build: &specforge_registry::RegistryBuild) -> Vec<(&str, &str)> {
    build
        .rules
        .iter()
        .map(|(rule, _)| (rule.code.as_str(), rule.message_template.as_str()))
        .collect()
}

#[spec(
    behavior = "registry_build_rules",
    verify = "every declared rule is in the build's rules with the extension that declared it"
)]
fn every_declared_rule_is_in_the_build_with_its_extension() {
    let a = declare("@ext/a", |c| {
        c.rule("W100", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("orphan {kind} '{id}'")
                .check(CheckKind::NoIncomingEdges)
                .target_kind("behavior");
        });
    });
    let b = rules("@ext/b", &[("W200", "b rule", CheckKind::NoOutgoingEdges)]);
    let build = build([software(), a, b]);

    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
    assert_eq!(rule_codes(&build), [("W100", "@ext/a"), ("W200", "@ext/b")]);
    let (w100, _) = &build.rules[0];
    assert_eq!(w100.check, ValidationPatternKind::NoIncomingEdges);
    assert_eq!(w100.check.as_str(), "no_incoming_edges");
    assert_eq!(w100.severity, Severity::Warning);
    assert_eq!(w100.message_template, "orphan {kind} '{id}'");
    assert_eq!(w100.target_kind.as_deref(), Some("behavior"));
    assert_eq!(
        build.rules[1].0.check,
        ValidationPatternKind::NoOutgoingEdges
    );
}

#[spec(
    behavior = "registry_build_rules",
    verify = "the extensions' rules are ordered by code"
)]
fn the_extensions_rules_are_ordered_by_code() {
    let a = rules(
        "@ext/a",
        &[
            ("W300", "a300", CheckKind::NoIncomingEdges),
            ("W100", "a100", CheckKind::NoIncomingEdges),
        ],
    );
    let b = rules(
        "@ext/b",
        &[
            ("W200", "b200", CheckKind::NoOutgoingEdges),
            ("W100", "b100", CheckKind::NoOutgoingEdges),
        ],
    );
    let build = build([a.clone(), b.clone()]);
    assert_eq!(
        rule_codes(&build),
        [
            ("W100", "@ext/a"),
            ("W100", "@ext/b"),
            ("W200", "@ext/b"),
            ("W300", "@ext/a"),
        ]
    );
    assert_eq!(
        templates(&build),
        [
            ("W100", "a100"),
            ("W100", "b100"),
            ("W200", "b200"),
            ("W300", "a300")
        ]
    );

    // Rules sharing a code keep load order.
    let swapped = crate::support::build([b, a]);
    assert_eq!(
        rule_codes(&swapped),
        [
            ("W100", "@ext/b"),
            ("W100", "@ext/a"),
            ("W200", "@ext/b"),
            ("W300", "@ext/a"),
        ]
    );

    // The host's E006 rules come after the extensions' rules.
    let required = declare("@ext/c", |c| {
        c.kind("Task", |k| {
            k.keyword("task");
            k.field("owner", |f| {
                f.field_type(FieldType::String).required();
            });
        });
        c.rule("W050", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("c050")
                .check(CheckKind::NoIncomingEdges);
        });
    });
    let with_e006 = crate::support::build([required]);
    assert_eq!(rule_codes(&with_e006), [("W050", "@ext/c"), ("E006", "")]);
}

#[spec(
    behavior = "registry_build_rules",
    verify = "the build keeps a rule whose target kind or edge type no loaded extension declares, and reports nothing for it"
)]
fn the_build_keeps_a_rule_whose_targets_no_extension_declares() {
    let ghostly = declare("@t/e", |c| {
        c.rule("W101", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("orphan {id}")
                .check(CheckKind::NoIncomingEdges)
                .target_kind("ghost")
                .edge_type("GhostEdge");
        });
        c.rule("W102", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("lonely {id}")
                .check(CheckKind::NoOutgoingEdges)
                .target_kind("nonexistent_kind");
        });
    });
    let build = build([software(), ghostly]);

    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
    assert_eq!(rule_codes(&build), [("W101", "@t/e"), ("W102", "@t/e")]);
    let (w101, _) = &build.rules[0];
    assert_eq!(w101.target_kind.as_deref(), Some("ghost"));
    assert_eq!(w101.edge_type.as_deref(), Some("GhostEdge"));
    assert_eq!(w101.edge_peer_kind, None);
    assert_eq!(
        build.rules[1].0.target_kind.as_deref(),
        Some("nonexistent_kind")
    );
}

#[spec(
    behavior = "registry_build_rules",
    verify = "extensions produce E006 rules for required fields"
)]
fn required_fields_get_e006_rules() {
    let declared = declare("@specforge/software", |c| {
        c.kind("Behavior", |k| {
            k.keyword("behavior");
            k.field("contract", |f| {
                f.field_type(FieldType::String).required();
            });
            k.field("category", |f| {
                f.field_type(FieldType::String);
            });
        });
        c.kind("Invariant", |k| {
            k.keyword("invariant");
            k.field("guarantee", |f| {
                f.field_type(FieldType::String).required();
            });
        });
    });
    let build = build([declared]);

    assert_eq!(rule_codes(&build), [("E006", ""), ("E006", "")]);
    for (rule, origin) in &build.rules {
        assert_eq!(origin, "", "a host rule is owned by no extension");
        assert_eq!(rule.severity, Severity::Error);
        assert_eq!(rule.check, ValidationPatternKind::MissingRequiredField);
    }
    let targets: Vec<(&str, &str)> = build
        .rules
        .iter()
        .map(|(rule, _)| {
            (
                rule.target_kind.as_deref().unwrap(),
                rule.field.as_deref().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        targets,
        [("behavior", "contract"), ("invariant", "guarantee")]
    );

    // No required field, no rule.
    assert!(crate::support::build([software()]).rules.is_empty());
}

#[spec(
    behavior = "registry_build_rules",
    verify = "Registry Build Collects Rules: rule collection holds — declarations_in_load_order, rules_collected, duplicates_warned, required_enforced, unloaded_targets_inert"
)]
fn rule_collection_holds() {
    // declarations_in_load_order: each extension's rules out of code order;
    // both declare W100; `b` has a required field and a rule for a kind no
    // extension declares.
    let a = rules(
        "@ext/a",
        &[
            ("W300", "a300", CheckKind::NoIncomingEdges),
            ("W100", "a100", CheckKind::NoIncomingEdges),
        ],
    );
    let b = declare("@ext/b", |c| {
        c.kind("Task", |k| {
            k.keyword("task");
            k.field("owner", |f| {
                f.field_type(FieldType::String).required();
            });
        });
        for (code, message) in [("W200", "b200"), ("W100", "b100")] {
            c.rule(code, |r| {
                r.severity(ValidationSeverity::Warning)
                    .message_template(message)
                    .check(CheckKind::NoOutgoingEdges);
            });
        }
        c.rule("W400", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("b400")
                .check(CheckKind::NoIncomingEdges)
                .target_kind("ghost");
        });
    });
    let build = build([a, b]);

    // rules_collected: one set, ordered by code, each with its extension.
    assert_eq!(
        rule_codes(&build),
        [
            ("W100", "@ext/a"),
            ("W100", "@ext/b"),
            ("W200", "@ext/b"),
            ("W300", "@ext/a"),
            ("W400", "@ext/b"),
            ("E006", ""),
        ]
    );

    // duplicates_warned: one W023 naming the code and both extensions.
    let w023 = coded(&build, "W023");
    assert_eq!(w023.len(), 1, "{:?}", diagnostics(&build));
    assert_eq!(w023[0].severity, Severity::Warning);
    for part in ["'W100'", "'@ext/a'", "'@ext/b'"] {
        assert!(w023[0].message.contains(part), "{}", w023[0].message);
    }
    assert!(build.registry_diagnostics.contains(w023[0]));

    // required_enforced: the required field has its E006 rule.
    let (e006, _) = build.rules.last().unwrap();
    assert_eq!(e006.target_kind.as_deref(), Some("task"));
    assert_eq!(e006.field.as_deref(), Some("owner"));

    // unloaded_targets_inert: the rule for `ghost` costs no diagnostic.
    assert_eq!(diagnostics(&build).len(), 1, "{:?}", diagnostics(&build));

    // A code one extension repeats is not a duplicate across extensions.
    let repeated = crate::support::build([rules(
        "@ext/solo",
        &[
            ("W100", "first", CheckKind::NoIncomingEdges),
            ("W100", "again", CheckKind::NoIncomingEdges),
        ],
    )]);
    assert!(coded(&repeated, "W023").is_empty());
    assert_eq!(repeated.rules.len(), 2);
}
