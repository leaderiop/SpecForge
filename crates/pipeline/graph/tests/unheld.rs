//! A reference in a fact scheme that names no fact (ADR 0327 D6): unheld, with no edge and no core diagnostic, kept on
//! its node; `Mistyped` (E022) and `Unavailable` (I004) are decided first (behavior `link_entity_references`).

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::Sym;
use specforge_graph::{
    FieldForm, FieldForms, Graph, GraphConfig, GraphInputs, build_graph_with_config,
};
use specforge_parser::{SpecFile, parse};
use specforge_protocol_types::{DanglingReason, FieldType};
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

/// A fact's own references (ADR 0346 D4): `test.covers` names anything, `test.proves` names `truth`s; `truth` is
/// scoped by its `plan`. The facts are served from a fact store, the path that makes their holder a fact.
fn build_facts(files: &[SpecFile]) -> (Graph, Vec<specforge_graph::Diagnostic>) {
    let list = || FieldForm::new(FieldType::ReferenceList, Vec::new());
    let reference = |kind: &str| FieldForm::new(FieldType::Reference, Vec::new()).targeting(kind);
    let config = GraphConfig {
        inputs: GraphInputs {
            field_forms: FieldForms::new(
                [
                    (("test", "covers"), list()),
                    (("test", "proves"), list().targeting("truth")),
                    (("truth", "plan"), reference("plan")),
                    (("truth", "covers"), list()),
                ]
                .into_iter()
                .map(|((kind, field), form)| ((kind.to_string(), field.to_string()), form))
                .collect(),
                Default::default(),
                ["test", "truth", "plan"].map(String::from).into(),
            ),
            scopes: BTreeMap::from([("truth".to_string(), "plan".to_string())]),
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

const STORE: &str = ".specforge/facts/k/node.json";

/// A served test fact writing `fields`, as its store gives it.
fn served(fields: &str) -> SpecFile {
    let mut file = parse(
        &format!("test placeholder \"P\" {{\n  {fields}\n}}\n"),
        STORE,
    );
    file.entities[0].id.raw = Sym::new(FACT);
    file
}

const PLANS: &str = "plan v1 \"One\" {}\nplan v2 \"Two\" {}\ntruth TR1 \"T\" {\n  plan v1\n}\n";

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fact's reference that names no entity, or one of another kind, is dangling: no E003 or E022, and the passes are sent it at its name span"
)]
fn a_facts_reference_naming_no_entity_is_dangling_with_no_diagnostic() {
    let (graph, diagnostics) = build_facts(&[parse(PLANS, "a.spec"), served("covers [nobody]")]);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let fact = graph.node(FACT).unwrap();
    let dangling: Vec<(&str, &str, DanglingReason)> = fact
        .dangling
        .iter()
        .map(|d| (d.field.as_str(), d.id.as_str(), d.reason))
        .collect();
    assert_eq!(dangling, [("covers", "nobody", DanglingReason::Unresolved)]);
    let span = fact.dangling[0].span.as_ref().expect("its name span");
    assert_eq!((span.file.as_str(), span.start_line), (STORE, 2));
    assert!(fact.unheld.is_empty());
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fact's reference that names no entity, or one of another kind, is dangling: no E003 or E022, and the passes are sent it at its name span"
)]
fn a_facts_mistyped_reference_is_dangling_and_keeps_its_edge() {
    let (graph, diagnostics) = build_facts(&[parse(PLANS, "a.spec"), served("proves [v1]")]);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let fact = graph.node(FACT).unwrap();
    assert_eq!(fact.dangling[0].reason, DanglingReason::Mistyped);
    assert_eq!(fact.dangling[0].actual.as_deref(), Some("plan"));
    let targets: Vec<&str> = graph
        .edges_from(FACT)
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(targets, ["v1"], "a mistyped reference keeps its edge");
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fact's short name links by its unique suffix"
)]
fn a_facts_short_name_links_by_its_unique_suffix() {
    let two =
        format!("{PLANS}truth TR2 \"T\" {{\n  plan v1\n}}\ntruth TR2 \"T\" {{\n  plan v2\n}}\n");

    let (graph, diagnostics) = build_facts(&[parse(&two, "a.spec"), served("covers [TR1, TR2]")]);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let fact = graph.node(FACT).unwrap();
    let targets: Vec<&str> = graph
        .edges_from(FACT)
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(targets, ["v1/TR1"]);
    assert_eq!(fact.dangling[0].reason, DanglingReason::Ambiguous);
    assert_eq!(fact.dangling[0].candidates, ["v1/TR2", "v2/TR2"]);
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fact's reference that names no entity, or one of another kind, is dangling: no E003 or E022, and the passes are sent it at its name span"
)]
fn an_authored_reference_naming_no_entity_is_still_e003() {
    let (graph, diagnostics) = build_facts(&[parse(
        &format!("{PLANS}truth TR3 \"T\" {{\n  plan v1\n  covers [nobody]\n}}\n"),
        "a.spec",
    )]);

    assert_eq!(codes(&diagnostics), ["E003"], "{diagnostics:?}");
    assert!(graph.node("v1/TR3").unwrap().dangling.is_empty());
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "a fact id two stores hold is E117 naming both collectors, not E002"
)]
fn a_fact_id_two_stores_hold_is_e117_not_e002() {
    let at = |store: &str| {
        let mut file = parse("test placeholder \"P\" {}\n", store);
        file.entities[0].id.raw = Sym::new(FACT);
        file
    };
    let (node, unit) = (
        ".specforge/facts/k/node.json",
        ".specforge/facts/k/unit.json",
    );

    let (graph, diagnostics) = build_facts(&[at(node), at(unit)]);

    assert_eq!(codes(&diagnostics), ["E117"], "{diagnostics:?}");
    assert_eq!(
        diagnostics[0].message,
        format!(
            "fact '{FACT}' is collected by both 'node' ({node}) and 'unit' ({unit}); the first is served"
        )
    );
    assert_eq!(diagnostics[0].span.as_ref().unwrap().file.as_str(), unit);
    assert_eq!(graph.node(FACT).unwrap().source_span.file.as_str(), node);
    // Two authored blocks, or a block and a fact, are still E002.
    let (_, authored) = build_facts(&[at("a.spec"), at("b.spec")]);
    assert_eq!(codes(&authored), ["E002"]);
}
