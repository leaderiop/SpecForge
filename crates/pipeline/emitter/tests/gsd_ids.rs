//! Every export writes an entity id holding a dash or a dot as it was written (ADR 0285, plan K1).

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
