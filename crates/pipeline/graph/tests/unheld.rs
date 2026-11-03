//! A reference in a fact scheme that names no fact (ADR 0327 D6): unheld, with no edge and no core diagnostic, kept on
//! its node; `Mistyped` (E022) and `Unavailable` (I004) are decided first (behavior `link_entity_references`).

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::Sym;
use specforge_graph::{
    FieldForm, FieldForms, Graph, GraphConfig, GraphInputs, build_graph_with_config,
};
use specforge_parser::{SpecFile, parse};
use specforge_protocol_types::FieldType;
use specforge_test_macros::test as specforge_test;

const FACT: &str = "js:a.test.js#adds";

/// `truth.proof` names `test`s, `truth.owner` names `person`s, `truth.note` names anything. `loaded` are the kinds a
/// loaded extension declares; the scheme `js` is `test`'s, a collector-written kind's.
fn build(files: &[SpecFile], loaded: &[&str]) -> (Graph, Vec<specforge_graph::Diagnostic>) {
    let form = |field: &str, target: Option<&str>| {
        let form = FieldForm::new(FieldType::ReferenceList, Vec::new());
        (
            ("truth".to_string(), field.to_string()),
            match target {
                Some(kind) => form.targeting(kind),
                None => form,
            },
        )
    };
    let config = GraphConfig {
        inputs: GraphInputs {
            field_forms: FieldForms::new(
                [
                    form("proof", Some("test")),
                    form("owner", Some("person")),
                    form("note", None),
                ]
                .into_iter()
                .collect(),
                Default::default(),
                loaded.iter().map(|k| k.to_string()).collect(),
            ),
            fact_schemes: BTreeMap::from([(
                "js".to_string(),
                BTreeSet::from(["test".to_string()]),
            )]),
            ..Default::default()
        },
        ..Default::default()
    };
    build_graph_with_config(files, &config)
}

const ALL: &[&str] = &["truth", "test", "person"];

fn truth(fields: &str) -> SpecFile {
    parse(&format!("truth t1 \"T\" {{\n  {fields}\n}}\n"), "main.spec")
}

/// A fact of `kind` whose id is `id`, as a served store gives it.
fn fact(kind: &str, id: &str) -> SpecFile {
    let mut file = parse(&format!("{kind} placeholder \"P\" {{}}\n"), "facts.spec");
    file.entities[0].id.raw = Sym::new(id);
    file
}

fn codes(diagnostics: &[specforge_graph::Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(|d| d.code.as_str()).collect()
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a reference in a fact scheme that names no fact is unheld: no edge and no core diagnostic"
)]
fn a_reference_in_a_fact_scheme_that_names_no_fact_is_unheld_with_no_diagnostic() {
    let (graph, diagnostics) = build(
        &[truth(&format!("proof [\"{FACT}\"]\n  note [\"js:b\"]"))],
        ALL,
    );

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(graph.edge_count(), 0);
    let unheld: Vec<(&str, &str)> = graph
        .node("t1")
        .unwrap()
        .unheld
        .iter()
        .map(|u| (u.field.as_str(), u.id.as_str()))
        .collect();
    assert_eq!(unheld, [("proof", FACT), ("note", "js:b")]);
    assert!(graph.node("t1").unwrap().unheld[0].span.is_some());
}

#[test]
fn a_reference_in_a_fact_scheme_naming_a_fact_links() {
    let (graph, diagnostics) = build(
        &[truth(&format!("proof [\"{FACT}\"]")), fact("test", FACT)],
        ALL,
    );

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let targets: Vec<&str> = graph
        .edges_from("t1")
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(targets, [FACT]);
    assert!(graph.node("t1").unwrap().unheld.is_empty());
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fact scheme reference whose target kind is absent stays I004, and one of the wrong kind E022"
)]
fn a_fact_scheme_reference_of_another_kind_is_still_mistyped() {
    let (graph, diagnostics) = build(
        &[truth(&format!("proof [\"{FACT}\"]")), fact("person", FACT)],
        ALL,
    );

    assert_eq!(codes(&diagnostics), ["E022"], "{diagnostics:?}");
    assert!(graph.node("t1").unwrap().unheld.is_empty());
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fact scheme reference whose target kind is absent stays I004, and one of the wrong kind E022"
)]
fn a_fact_scheme_reference_whose_target_kind_is_not_loaded_is_still_i004() {
    let (graph, diagnostics) = build(
        &[truth(&format!("proof [\"{FACT}\"]"))],
        &["truth", "person"],
    );

    assert_eq!(codes(&diagnostics), ["I004"], "{diagnostics:?}");
    assert!(graph.node("t1").unwrap().unheld.is_empty());
}

#[test]
fn a_scheme_no_collected_kind_declares_is_still_an_unresolved_reference() {
    let (graph, diagnostics) = build(&[truth("proof [\"rust:a::b\"]")], ALL);

    assert_eq!(codes(&diagnostics), ["E003"], "{diagnostics:?}");
    assert!(graph.node("t1").unwrap().unheld.is_empty());
}

#[test]
fn a_fact_scheme_reference_in_a_field_targeting_another_kind_is_an_unresolved_reference() {
    let (graph, diagnostics) = build(&[truth(&format!("owner [\"{FACT}\"]"))], ALL);

    assert_eq!(codes(&diagnostics), ["E003"], "{diagnostics:?}");
    assert!(graph.node("t1").unwrap().unheld.is_empty());
}
