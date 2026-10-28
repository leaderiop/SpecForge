//! Every export writes an entity id holding a dash or a dot as it was written (ADR 0283, plan K1).

use specforge_common::{SourceSpan, Sym};
use specforge_emitter::EmitFormat;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;

fn node(id: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: Some("A GSD entity".to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        methods: Vec::new(),
        obligations: Vec::new(),
        verdicts: Vec::new(),
        scope: None,
    }
}

#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "every export format writes an entity id holding a dash or a dot as written"
)]
fn every_export_writes_an_id_holding_a_dash_or_a_dot_as_written() {
    let ids = ["AUTH-01", "v1.0", "03-02", "03.1", "G-03-1"];
    let mut graph = Graph::new();
    for id in ids {
        graph.add_node(node(id));
    }
    graph.add_edge(Edge {
        source: Sym::new("AUTH-01"),
        target: Sym::new("v1.0"),
        label: Sym::new("behaviors"),
    });
    let registries = crate::support::headline_build(&["behavior"]);
    for format in [
        EmitFormat::Json,
        EmitFormat::Context,
        EmitFormat::Brief,
        EmitFormat::Dot,
        EmitFormat::Markdown,
    ] {
        let out = crate::export(&graph, format, Some(&registries));
        for id in ids {
            assert!(
                out.contains(id),
                "{format:?} drops or respells {id}:\n{out}"
            );
        }
    }
}

#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "an export of a project with no scoped kind writes each id as its name and each edge as written"
)]
fn an_unscoped_export_is_unchanged() {
    // The matrix golden (`emit_matrix`) pins the whole text; this states the link K2 must keep:
    // an unscoped entity's id is its name and an edge target is the written reference.
    let (graph, _) = specforge_graph::build_graph(&[specforge_parser::parse(
        "behavior alpha \"A\" { contract \"x\" invariants [inv_one] }\n\
         invariant inv_one \"I\" { contract \"y\" }\n",
        "main.spec",
    )]);
    let json = crate::support::emit_json(&graph);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let ids: Vec<&str> = value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["alpha", "inv_one"], "{json}");
    assert_eq!(value["edges"][0]["target"], "inv_one", "{json}");
}
