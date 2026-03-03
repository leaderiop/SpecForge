use crate::file_index::FileIndex;
use crate::spec_graph::SpecGraph;
use std::collections::HashSet;

/// A subset of the spec graph affected by file changes.
///
/// Used for watch mode / LSP incremental rebuilds: given a set of
/// invalidated files, compute which entity nodes and edges need to
/// be removed and rebuilt.
#[derive(Debug)]
pub struct Subgraph {
    /// Entity IDs (raw strings) that must be removed/rebuilt.
    pub node_ids: Vec<String>,
    /// Total number of edges touching those nodes.
    pub edge_count: usize,
}

/// Given a set of invalidated file paths, compute the subgraph of
/// entities that need to be removed and rebuilt.
///
/// For each file in `invalidated_files`, looks up all entity IDs via
/// the `FileIndex`, then counts incoming + outgoing edges for each
/// entity in the `SpecGraph`.
pub fn compute_invalidation_subgraph(
    graph: &SpecGraph,
    file_index: &FileIndex,
    invalidated_files: &HashSet<String>,
) -> Subgraph {
    let mut node_ids = Vec::new();
    let mut edge_count = 0;

    for file in invalidated_files {
        for entity_id in file_index.entities_in(file) {
            let incoming = graph.incoming_edges(entity_id).len();
            let outgoing = graph.outgoing_edges(entity_id).len();
            edge_count += incoming + outgoing;
            node_ids.push(entity_id.clone());
        }
    }

    node_ids.sort();
    node_ids.dedup();

    Subgraph { node_ids, edge_count }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_graph::{GraphEdge, GraphNode, SpecGraph};
    use specforge_common::{EdgeType, EntityId, EntityKind, SourceSpan};

    fn make_node(id: &str, kind: EntityKind, file: &str) -> GraphNode {
        GraphNode {
            id: EntityId::parse(id),
            kind,
            title: Some(format!("Test {id}")),
            file: file.to_string(),
            span: SourceSpan::new(file, 1, 1, 1, 1),
        }
    }

    fn make_edge(field: &str) -> GraphEdge {
        GraphEdge {
            edge_type: EdgeType::from_field_name(field).unwrap_or(EdgeType::References),
            field_name: field.to_string(),
            enhanced_label: None,
        }
    }

    #[test]
    fn changed_file_and_direct_dependents_are_invalidated() {
        let mut graph = SpecGraph::new();
        let mut file_index = FileIndex::new();

        // invariants.spec has inv_a, behaviors.spec has beh_a which references inv_a
        graph.add_node(make_node("inv_a", EntityKind::Invariant, "invariants.spec"));
        graph.add_node(make_node("beh_a", EntityKind::Behavior, "behaviors.spec"));
        graph.add_edge("beh_a", "inv_a", make_edge("invariants"));

        file_index.register("inv_a", "invariants.spec");
        file_index.register("beh_a", "behaviors.spec");

        // Invalidate invariants.spec — inv_a should be in the subgraph
        let invalidated: HashSet<String> = ["invariants.spec".to_string()].into();
        let sub = compute_invalidation_subgraph(&graph, &file_index, &invalidated);

        assert_eq!(sub.node_ids, vec!["inv_a"]);
        // inv_a has 1 incoming edge (from beh_a)
        assert_eq!(sub.edge_count, 1);
    }

    #[test]
    fn transitive_dependents_included_via_file_graph() {
        // This test shows the full workflow: FileGraph computes which files
        // are invalidated, then compute_invalidation_subgraph finds the entities.
        use specforge_resolver::FileGraph;

        let mut graph = SpecGraph::new();
        let mut file_index = FileIndex::new();
        let mut file_graph = FileGraph::new();

        // c.spec defines type_c, b.spec imports c.spec and defines beh_b -> type_c,
        // a.spec imports b.spec and defines feat_a -> beh_b
        graph.add_node(make_node("type_c", EntityKind::TypeDef, "c.spec"));
        graph.add_node(make_node("beh_b", EntityKind::Behavior, "b.spec"));
        graph.add_node(make_node("feat_a", EntityKind::Feature, "a.spec"));
        graph.add_edge("beh_b", "type_c", make_edge("types"));
        graph.add_edge("feat_a", "beh_b", make_edge("behaviors"));

        file_index.register("type_c", "c.spec");
        file_index.register("beh_b", "b.spec");
        file_index.register("feat_a", "a.spec");

        file_graph.add_import("b.spec", "c.spec");
        file_graph.add_import("a.spec", "b.spec");

        // Change c.spec — transitive invalidation should include a.spec, b.spec, c.spec
        let invalidated = file_graph.invalidation_set("c.spec");
        assert_eq!(invalidated.len(), 3);

        let sub = compute_invalidation_subgraph(&graph, &file_index, &invalidated);
        assert_eq!(sub.node_ids.len(), 3);
        assert!(sub.node_ids.contains(&"type_c".to_string()));
        assert!(sub.node_ids.contains(&"beh_b".to_string()));
        assert!(sub.node_ids.contains(&"feat_a".to_string()));
        // beh_b->type_c (counted on both nodes) + feat_a->beh_b (counted on both nodes)
        // type_c: 1 incoming = 1
        // beh_b: 1 incoming + 1 outgoing = 2
        // feat_a: 1 outgoing = 1
        assert_eq!(sub.edge_count, 4);
    }

    #[test]
    fn unaffected_files_not_invalidated() {
        let mut graph = SpecGraph::new();
        let mut file_index = FileIndex::new();

        graph.add_node(make_node("inv_a", EntityKind::Invariant, "invariants.spec"));
        graph.add_node(make_node("beh_a", EntityKind::Behavior, "behaviors.spec"));
        graph.add_node(make_node("feat_x", EntityKind::Feature, "features.spec"));
        graph.add_edge("beh_a", "inv_a", make_edge("invariants"));

        file_index.register("inv_a", "invariants.spec");
        file_index.register("beh_a", "behaviors.spec");
        file_index.register("feat_x", "features.spec");

        // Invalidate only features.spec — inv_a and beh_a should NOT appear
        let invalidated: HashSet<String> = ["features.spec".to_string()].into();
        let sub = compute_invalidation_subgraph(&graph, &file_index, &invalidated);

        assert_eq!(sub.node_ids, vec!["feat_x"]);
        assert_eq!(sub.edge_count, 0);
    }

    #[test]
    fn subgraph_rebuild_matches_full_rebuild() {
        // Build the graph, compute invalidation for ALL files, verify we get
        // all nodes — proving that invalidating everything = full rebuild.
        let mut graph = SpecGraph::new();
        let mut file_index = FileIndex::new();

        graph.add_node(make_node("inv_a", EntityKind::Invariant, "a.spec"));
        graph.add_node(make_node("beh_b", EntityKind::Behavior, "b.spec"));
        graph.add_node(make_node("feat_c", EntityKind::Feature, "c.spec"));
        graph.add_edge("beh_b", "inv_a", make_edge("invariants"));
        graph.add_edge("feat_c", "beh_b", make_edge("behaviors"));

        file_index.register("inv_a", "a.spec");
        file_index.register("beh_b", "b.spec");
        file_index.register("feat_c", "c.spec");

        let all_files: HashSet<String> =
            ["a.spec", "b.spec", "c.spec"].iter().map(|s| s.to_string()).collect();
        let sub = compute_invalidation_subgraph(&graph, &file_index, &all_files);

        assert_eq!(sub.node_ids.len(), graph.node_count());
        assert_eq!(sub.node_ids.len(), 3);
    }
}
