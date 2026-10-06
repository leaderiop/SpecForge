//! `cycle_detection` rules: the entities on a cycle of the edges their edge
//! type is written as, over the input's edge records.

use specforge_protocol_types::{ValidationRuleDescriptor, ValidationSeverity};
use specforge_registry::entity::{EdgeRecord, EntityRecord};
use specforge_registry::rules::NoVerdicts;
use specforge_test_macros::test as spec;

use super::{entity, messages, rules_of, with_edges};
use crate::support::declare;
use specforge_extension_sdk::prelude::*;

/// `@test` declaring `module` with `depends_on` (writing the edge
/// `ModuleDependsOn` to `module`) and `uses` (a plain reference list), plus
/// `rules`.
fn modules(rules: Vec<ValidationRuleDescriptor>) -> super::Built {
    let mut declaration = declare("@test", |c| {
        c.kind("module", |k| {
            k.description("m");
            k.field("depends_on", |f| {
                f.field_type(FieldType::ReferenceList)
                    .edge("ModuleDependsOn")
                    .target_kind("module");
            });
            k.field("uses", |f| {
                f.field_type(FieldType::ReferenceList).target_kind("module");
            });
        });
        c.kind("other", |k| {
            k.description("o");
        });
        c.edge("ModuleDependsOn", |e| {
            e.source_kind("module").target_kind("module");
        });
    });
    declaration.validation_rules = rules;
    rules_of(vec![declaration])
}

/// A cycle rule `code` on `edge_type`, targeting `target`.
pub fn cycle_rule(
    code: &str,
    target: Option<&str>,
    edge_type: Option<&str>,
) -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: code.to_string(),
        severity: ValidationSeverity::Error,
        message_template: "{kind} '{id}' is on a dependency cycle".to_string(),
        check: "cycle_detection".to_string(),
        target_kind: target.map(str::to_string),
        edge_type: edge_type.map(str::to_string),
        ..Default::default()
    }
}

fn edge(source: &str, target: &str, label: &str) -> EdgeRecord {
    EdgeRecord {
        source: source.to_string(),
        target: target.to_string(),
        label: label.to_string(),
    }
}

/// `a ⇄ b` and `c → a` over `depends_on`, `x ⇄ y` over `uses`, and the
/// `other` entity `o` on a `depends_on` loop with `a`.
fn project() -> (Vec<EntityRecord>, Vec<EdgeRecord>) {
    let entities = vec![
        entity("a", "module", 0, 0),
        entity("b", "module", 0, 0),
        entity("c", "module", 0, 0),
        entity("o", "other", 0, 0),
        entity("x", "module", 0, 0),
        entity("y", "module", 0, 0),
    ];
    let edges = vec![
        edge("b", "a", "depends_on"),
        edge("a", "b", "depends_on"),
        edge("c", "a", "depends_on"),
        edge("o", "c", "depends_on"),
        edge("c", "o", "depends_on"),
        edge("x", "y", "uses"),
        edge("y", "x", "uses"),
    ];
    (entities, edges)
}

#[spec(
    behavior = "execute_validation_pattern",
    verify = "cycle_detection finds cycles in edge type"
)]
fn cycle_detection_finds_cycles_in_edge_type() {
    let built = modules(vec![cycle_rule(
        "E007",
        Some("module"),
        Some("ModuleDependsOn"),
    )]);
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    let (entities, edges) = project();

    let diagnostics = built
        .rules
        .check(&with_edges(&entities, &edges), &NoVerdicts);

    // a and b, by id; not c (it only leads into the cycle), not x and y
    // (another edge type), not the loop through the `other` entity.
    assert_eq!(
        messages(&diagnostics),
        [
            "module 'a' is on a dependency cycle",
            "module 'b' is on a dependency cycle",
        ]
    );
    assert!(
        diagnostics
            .iter()
            .all(|d| d.code == "E007" && d.span.is_some())
    );
}

#[test]
fn a_cycle_rule_runs_once_per_check_whatever_the_entity_order() {
    let built = modules(vec![cycle_rule(
        "E007",
        Some("module"),
        Some("ModuleDependsOn"),
    )]);
    let (mut entities, edges) = project();
    let forward = built
        .rules
        .check(&with_edges(&entities, &edges), &NoVerdicts);
    entities.reverse();
    let reversed = built
        .rules
        .check(&with_edges(&entities, &edges), &NoVerdicts);
    assert_eq!(forward, reversed);
}

// PIN (T5): an edge type no field writes is followed as the raw label.
#[test]
fn a_cycle_rule_on_an_edge_type_no_field_writes_follows_the_raw_label() {
    let built = modules(vec![cycle_rule("E008", Some("module"), Some("uses"))]);
    let (entities, edges) = project();
    let diagnostics = built
        .rules
        .check(&with_edges(&entities, &edges), &NoVerdicts);
    assert_eq!(
        messages(&diagnostics),
        [
            "module 'x' is on a dependency cycle",
            "module 'y' is on a dependency cycle",
        ]
    );
}

// PIN (T4): a cycle rule without an edge type registers and reports
// nothing; without a target kind it reports nothing either.
#[test]
fn a_cycle_rule_missing_its_edge_type_or_target_kind_reports_nothing() {
    let built = modules(vec![
        cycle_rule("E009", Some("module"), None),
        cycle_rule("E010", None, Some("ModuleDependsOn")),
    ]);
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    assert_eq!(built.codes(), ["E009", "E010"]);
    let (entities, edges) = project();
    assert!(
        built
            .rules
            .check(&with_edges(&entities, &edges), &NoVerdicts)
            .is_empty()
    );
}
