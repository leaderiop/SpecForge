//! The connectivity rule of the project view (ADR 0015 "Prompt read
//! views"): one `Degree` per entity, stats' unconnected count and the count
//! of entities per kind.

use specforge_common::{SourceSpan, Sym};
use specforge_graph::Degree;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_registry::RegistryBuild;
use specforge_test::prelude::*;

use crate::view_support::{Assembled, R1_SOURCE, Viewed};

fn node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: None,
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
        unheld: Vec::new(),
        located: Vec::new(),
        dangling: Vec::new(),
    }
}

#[specforge_test(requirement = "FEATURE-025")]
#[specforge_test(requirement = "FEATURE-064")]
#[specforge_test(requirement = "FEATURE-130")]
fn an_unresolved_or_self_reference_links_nothing() {
    let project = Assembled::over(RegistryBuild::default()).source(R1_SOURCE);
    let connectivity = project.view().connectivity();

    assert_eq!(
        connectivity.unconnected().collect::<Vec<_>>(),
        ["alone", "dangling", "lonely", "selfish"]
    );
    assert_eq!(connectivity.degree("selfish"), Degree::default());
    assert_eq!(
        connectivity.degree("hub"),
        Degree {
            incoming: 1,
            outgoing: 0
        }
    );
    assert_eq!(connectivity.degree("ghost"), Degree::default());
    assert_eq!(
        Degree {
            incoming: 1,
            outgoing: 3
        }
        .lead(),
        2
    );
}

#[specforge_test(requirement = "FEATURE-025")]
#[specforge_test(requirement = "FEATURE-064")]
#[specforge_test(requirement = "FEATURE-130")]
fn entities_are_counted_by_kind() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior"));
    graph.add_node(node("b", "behavior"));
    graph.add_node(node("w", "widget")); // a kind no extension declares
    graph.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("needs"),
    });

    let project = Assembled::over(RegistryBuild::default()).graph(graph);
    let counts = project.view().entities_by_kind();
    assert_eq!(
        counts.into_iter().collect::<Vec<_>>(),
        [("behavior", 2), ("widget", 1)]
    );
}
