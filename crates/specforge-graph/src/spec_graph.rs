use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use specforge_common::{EdgeType, EntityId, EntityKind, SourceSpan};
use std::collections::HashMap;

/// A node in the spec graph representing a declared entity.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: EntityId,
    pub kind: EntityKind,
    pub title: Option<String>,
    pub file: String,
    pub span: SourceSpan,
}

/// An edge in the spec graph representing a relationship between entities.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub edge_type: EdgeType,
    pub field_name: String,
    /// Set when `edge_type == Enhanced`; holds the plugin-contributed edge label.
    pub enhanced_label: Option<String>,
}

/// The spec graph: a directed graph of entities connected by typed edges.
///
/// Wraps `petgraph::DiGraph` with entity-specific query helpers.
/// Mutable API supports incremental updates for watch mode / LSP.
#[derive(Debug)]
pub struct SpecGraph {
    graph: DiGraph<GraphNode, GraphEdge>,
    id_to_node: HashMap<String, NodeIndex>,
}

impl SpecGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            id_to_node: HashMap::new(),
        }
    }

    /// Add an entity node. Returns the node index.
    /// If a node with this ID already exists, returns its existing index.
    pub fn add_node(&mut self, node: GraphNode) -> NodeIndex {
        let raw = node.id.raw().to_string();
        if let Some(&idx) = self.id_to_node.get(&raw) {
            return idx;
        }
        let idx = self.graph.add_node(node);
        self.id_to_node.insert(raw, idx);
        idx
    }

    /// Add a typed edge between two nodes (by raw ID strings).
    /// Returns true if both nodes exist and the edge was added.
    pub fn add_edge(&mut self, from_id: &str, to_id: &str, edge: GraphEdge) -> bool {
        let from_idx = match self.id_to_node.get(from_id) {
            Some(&idx) => idx,
            None => return false,
        };
        let to_idx = match self.id_to_node.get(to_id) {
            Some(&idx) => idx,
            None => return false,
        };
        self.graph.add_edge(from_idx, to_idx, edge);
        true
    }

    /// Remove a node by raw ID. Returns the removed node if it existed.
    pub fn remove_node(&mut self, raw_id: &str) -> Option<GraphNode> {
        if let Some(idx) = self.id_to_node.remove(raw_id) {
            self.graph.remove_node(idx)
        } else {
            None
        }
    }

    /// Look up a node by raw ID string.
    pub fn get_node(&self, raw_id: &str) -> Option<&GraphNode> {
        self.id_to_node
            .get(raw_id)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    /// Get the node index for a raw ID.
    pub fn node_index(&self, raw_id: &str) -> Option<NodeIndex> {
        self.id_to_node.get(raw_id).copied()
    }

    /// Number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Iterate over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.graph.node_weights()
    }

    /// Iterate over all edges with source and target node references.
    pub fn edges(&self) -> impl Iterator<Item = (&GraphNode, &GraphNode, &GraphEdge)> {
        self.graph.edge_indices().filter_map(move |edge_idx| {
            let (src, tgt) = self.graph.edge_endpoints(edge_idx)?;
            let edge = self.graph.edge_weight(edge_idx)?;
            let src_node = self.graph.node_weight(src)?;
            let tgt_node = self.graph.node_weight(tgt)?;
            Some((src_node, tgt_node, edge))
        })
    }

    /// Get all nodes of a specific kind.
    pub fn nodes_of_kind(&self, kind: EntityKind) -> Vec<&GraphNode> {
        self.graph
            .node_weights()
            .filter(|n| n.kind == kind)
            .collect()
    }

    /// Get incoming edges to a node (who points to this node).
    pub fn incoming_edges(&self, raw_id: &str) -> Vec<(&GraphNode, &GraphEdge)> {
        let Some(&idx) = self.id_to_node.get(raw_id) else {
            return Vec::new();
        };
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .filter_map(|edge_ref| {
                let src = self.graph.node_weight(edge_ref.source())?;
                Some((src, edge_ref.weight()))
            })
            .collect()
    }

    /// Get outgoing edges from a node (what this node points to).
    pub fn outgoing_edges(&self, raw_id: &str) -> Vec<(&GraphNode, &GraphEdge)> {
        let Some(&idx) = self.id_to_node.get(raw_id) else {
            return Vec::new();
        };
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .filter_map(|edge_ref| {
                let tgt = self.graph.node_weight(edge_ref.target())?;
                Some((tgt, edge_ref.weight()))
            })
            .collect()
    }

    /// Find orphan nodes — nodes with no incoming or outgoing edges.
    /// Excludes singleton kinds (spec, glossary) which are expected to be standalone.
    pub fn orphans(&self) -> Vec<&GraphNode> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                let node = &self.graph[idx];
                // Singletons are expected to have no edges
                if node.kind.is_singleton() {
                    return false;
                }
                let incoming = self
                    .graph
                    .edges_directed(idx, Direction::Incoming)
                    .count();
                let outgoing = self
                    .graph
                    .edges_directed(idx, Direction::Outgoing)
                    .count();
                incoming == 0 && outgoing == 0
            })
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    /// Get the underlying petgraph for advanced queries.
    pub fn inner(&self) -> &DiGraph<GraphNode, GraphEdge> {
        &self.graph
    }
}

impl Default for SpecGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::SourceSpan;

    fn make_node(id: &str, kind: EntityKind) -> GraphNode {
        GraphNode {
            id: EntityId::parse(id),
            kind,
            title: Some(format!("Test {id}")),
            file: "test.spec".to_string(),
            span: SourceSpan::new("test.spec", 1, 1, 1, 1),
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
    fn add_and_get_node() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        assert_eq!(graph.node_count(), 1);
        let node = graph.get_node("INV-SF-1").unwrap();
        assert_eq!(node.kind, EntityKind::Invariant);
    }

    #[test]
    fn duplicate_node_returns_existing() {
        let mut graph = SpecGraph::new();
        let idx1 = graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        let idx2 = graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        assert_eq!(idx1, idx2);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn add_edge_between_nodes() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("BEH-SF-1", EntityKind::Behavior));
        graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        let added = graph.add_edge("BEH-SF-1", "INV-SF-1", make_edge("invariants"));
        assert!(added);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn add_edge_missing_node_fails() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("BEH-SF-1", EntityKind::Behavior));
        let added = graph.add_edge("BEH-SF-1", "INV-SF-99", make_edge("invariants"));
        assert!(!added);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn incoming_outgoing_edges() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("BEH-SF-1", EntityKind::Behavior));
        graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        graph.add_edge("BEH-SF-1", "INV-SF-1", make_edge("invariants"));

        let incoming = graph.incoming_edges("INV-SF-1");
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].0.id.raw(), "BEH-SF-1");
        assert_eq!(incoming[0].1.edge_type, EdgeType::References);

        let outgoing = graph.outgoing_edges("BEH-SF-1");
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].0.id.raw(), "INV-SF-1");
    }

    #[test]
    fn nodes_of_kind() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        graph.add_node(make_node("INV-SF-2", EntityKind::Invariant));
        graph.add_node(make_node("BEH-SF-1", EntityKind::Behavior));

        let invariants = graph.nodes_of_kind(EntityKind::Invariant);
        assert_eq!(invariants.len(), 2);
        let behaviors = graph.nodes_of_kind(EntityKind::Behavior);
        assert_eq!(behaviors.len(), 1);
    }

    #[test]
    fn orphan_detection() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        graph.add_node(make_node("BEH-SF-1", EntityKind::Behavior));
        graph.add_node(make_node("INV-SF-2", EntityKind::Invariant));
        // Only BEH-SF-1 -> INV-SF-1, so INV-SF-2 is an orphan
        graph.add_edge("BEH-SF-1", "INV-SF-1", make_edge("invariants"));

        let orphans = graph.orphans();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id.raw(), "INV-SF-2");
    }

    #[test]
    fn singletons_not_orphans() {
        let mut graph = SpecGraph::new();
        graph.add_node(GraphNode {
            id: EntityId::Named { name: "specforge".to_string() },
            kind: EntityKind::Spec,
            title: Some("specforge".to_string()),
            file: "specforge.spec".to_string(),
            span: SourceSpan::new("specforge.spec", 1, 1, 1, 1),
        });
        // Spec is a singleton — should not appear in orphans even with no edges
        assert!(graph.orphans().is_empty());
    }

    #[test]
    fn remove_node() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        assert_eq!(graph.node_count(), 1);
        let removed = graph.remove_node("INV-SF-1");
        assert!(removed.is_some());
        assert_eq!(graph.node_count(), 0);
        assert!(graph.get_node("INV-SF-1").is_none());
    }

    #[test]
    fn edges_iterator() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("BEH-SF-1", EntityKind::Behavior));
        graph.add_node(make_node("INV-SF-1", EntityKind::Invariant));
        graph.add_node(make_node("INV-SF-2", EntityKind::Invariant));
        graph.add_edge("BEH-SF-1", "INV-SF-1", make_edge("invariants"));
        graph.add_edge("BEH-SF-1", "INV-SF-2", make_edge("invariants"));

        let edges: Vec<_> = graph.edges().collect();
        assert_eq!(edges.len(), 2);
    }
}
