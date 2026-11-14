//! The value bytes a ledger digest, a history check and `previous` share (ADR 0287 U2).

use std::collections::{BTreeMap, BTreeSet};

use specforge_graph::wire::{bytes, resolved};
use specforge_graph::{
    FieldForm, FieldForms, Graph, GraphConfig, GraphInputs, build_graph_with_config,
};
use specforge_parser::parse;
use specforge_protocol_types::FieldType;
use specforge_test_macros::test as specforge_test;

fn forms() -> FieldForms {
    let reference = |kind: &str| FieldForm::new(FieldType::Reference, Vec::new()).targeting(kind);
    FieldForms::new(
        [
            (("phase", "milestone"), reference("milestone")),
            (("uat", "phase"), reference("phase")),
            (
                ("uat", "peers"),
                FieldForm::new(FieldType::ReferenceList, Vec::new()).targeting("phase"),
            ),
        ]
        .into_iter()
        .map(|((kind, field), form)| ((kind.to_string(), field.to_string()), form))
        .collect(),
        BTreeSet::new(),
        ["milestone", "phase", "uat"].map(String::from).into(),
    )
}

fn graph(source: &str) -> Graph {
    let config = GraphConfig {
        inputs: GraphInputs {
            field_forms: forms(),
            scopes: BTreeMap::from([
                ("phase".to_string(), "milestone".to_string()),
                ("uat".to_string(), "phase".to_string()),
            ]),
            ..Default::default()
        },
        ..Default::default()
    };
    build_graph_with_config(&[parse(source, "a.spec")], &config).0
}

const BASE: &str = "milestone v1.0 \"M\" {}\nphase 03 \"P\" {\n  milestone v1.0\n}\n";

fn bytes_of(graph: &Graph, field: &str) -> Vec<u8> {
    let node = graph.node("v1.0/03/U1").expect("the uat is in the graph");
    bytes(&resolved(graph, &forms(), node, field).expect("the uat writes the field"))
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "the value digest reads the target of a reference, not its spelling"
)]
fn the_value_digest_reads_the_target_not_the_spelling() {
    let short = graph(&format!("{BASE}uat U1 \"U\" {{\n  phase 03\n}}\n"));
    let long = graph(&format!("{BASE}uat U1 \"U\" {{\n  phase v1.0/03\n}}\n"));
    assert_eq!(bytes_of(&short, "phase"), b"\"v1.0/03\"");
    assert_eq!(bytes_of(&short, "phase"), bytes_of(&long, "phase"));

    // A list resolves each item; an unresolved one keeps its text.
    let list = graph(&format!(
        "{BASE}uat U1 \"U\" {{\n  phase 03\n  peers [03, ghost]\n}}\n"
    ));
    assert_eq!(bytes_of(&list, "peers"), b"[\"v1.0/03\",\"ghost\"]");
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "an entity digest leaves out a context-written value as it leaves out computed and verdict values"
)]
fn a_context_written_value_is_not_in_the_entity_bytes() {
    use specforge_graph::wire::entity_bytes;
    use specforge_protocol_types::FieldWriter;
    let written = |writer: FieldWriter| {
        FieldForms::new(
            [(
                ("uat".to_string(), "pack".to_string()),
                FieldForm::new(FieldType::Block, Vec::new()).written_by(Some(writer)),
            )]
            .into_iter()
            .collect(),
            BTreeSet::new(),
            ["milestone", "phase", "uat"].map(String::from).into(),
        )
    };
    let bytes = |source: &str, writer: FieldWriter| {
        let graph = graph(source);
        let node = graph.node("v1.0/03/U1").expect("the uat is in the graph");
        entity_bytes(&graph, &written(writer), node)
    };
    let bare = format!("{BASE}uat U1 \"U\" {{\n  phase 03\n}}\n");
    let recorded = format!(
        "{BASE}uat U1 \"U\" {{\n  phase 03\n  pack {{ route \"x/y\" digest \"sha256:1\" max_tokens 10 }}\n}}\n"
    );
    assert_eq!(
        bytes(&bare, FieldWriter::Context),
        bytes(&recorded, FieldWriter::Context)
    );
    assert_ne!(
        bytes(&bare, FieldWriter::Harness),
        bytes(&recorded, FieldWriter::Harness),
        "a harness value is in the entity bytes"
    );
}
