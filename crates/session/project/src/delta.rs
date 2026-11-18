//! The delta between two graphs (ADR 0376 D5, D6): the graph's own delta ([`compute_graph_delta`]), each changed node
//! named by its kind with the digests of its content before and after ([`entity_digest`]), sorted, and filtered by
//! kind or by the file a node is declared in with no graph at hand. What `specforge diff` names and what a pass that
//! declares the `delta` input is sent.

use specforge_graph::{FieldForms, Graph, NodeAspect, compute_graph_delta};
use specforge_protocol_types::{
    DeltaAspect, DeltaBase, DeltaChange, DeltaEdge, DeltaNode, DeltaPayload,
};

use crate::judged::entity_digest;

/// The change between two graphs, with where each changed node is declared and the kind and file of each end of a
/// changed edge, so it filters with no graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    payload: DeltaPayload,
    /// The file each of `payload.nodes` is declared in, in the same order.
    files: Vec<String>,
    /// The two ends of each of `payload.added_edges`, in the same order.
    added_ends: Vec<[End; 2]>,
    /// The two ends of each of `payload.removed_edges`, in the same order.
    removed_ends: Vec<[End; 2]>,
}

/// An end of a changed edge: the kind and the declaring file of the node it names, empty when no node has its id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct End {
    kind: String,
    file: String,
}

impl Delta {
    /// What changed from `base` to `graph`, labelled `label`: every node added (with the digest of its content now),
    /// removed (its kind and id only) or modified (its changed fields and aspects, both digests), sorted by kind then
    /// id, and the edges added and removed, sorted by source, target then label, all compared as bytes. A node whose
    /// kind changed is its old kind removed and its new kind added. `forms` reads the values a digest covers.
    pub fn between(base: &Graph, graph: &Graph, forms: &FieldForms, label: DeltaBase) -> Delta {
        let change = compute_graph_delta(base, graph);
        let node = |kind: &str, id: &str, change: DeltaChange| DeltaNode {
            kind: kind.to_string(),
            id: id.to_string(),
            change,
            fields: Vec::new(),
            aspects: Vec::new(),
            before: None,
            after: None,
        };
        let digest =
            |graph: &Graph, id: &str| graph.node(id).map(|node| entity_digest(graph, forms, node));
        let mut nodes: Vec<(DeltaNode, String)> = Vec::new();
        for added in &change.added_nodes {
            let mut entry = node(&added.kind, &added.id, DeltaChange::Added);
            entry.after = digest(graph, &added.id);
            nodes.push((entry, added.file.clone()));
        }
        for removed in &change.removed_nodes {
            let entry = node(&removed.kind, &removed.id, DeltaChange::Removed);
            nodes.push((entry, removed.file.clone()));
        }
        for modified in &change.modified_nodes {
            let (Some(was), Some(is)) = (base.node(&modified.id), graph.node(&modified.id)) else {
                continue;
            };
            let file = is.source_span.file.as_str().to_string();
            // an entity is its kind and its id: another kind is another entity
            if modified.changed_aspects.contains(&NodeAspect::Kind) {
                let removed = node(was.kind.raw.as_str(), &modified.id, DeltaChange::Removed);
                nodes.push((removed, was.source_span.file.as_str().to_string()));
                let mut added = node(is.kind.raw.as_str(), &modified.id, DeltaChange::Added);
                added.after = digest(graph, &modified.id);
                nodes.push((added, file));
                continue;
            }
            let mut entry = node(is.kind.raw.as_str(), &modified.id, DeltaChange::Modified);
            entry.fields = modified.changed_fields.clone();
            entry.aspects = modified
                .changed_aspects
                .iter()
                .filter_map(|aspect| aspect_of(*aspect))
                .collect();
            entry.before = digest(base, &modified.id);
            entry.after = digest(graph, &modified.id);
            nodes.push((entry, file));
        }
        nodes.sort_by(|(a, _), (b, _)| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        let ends = |graph: &Graph, id: &str| match graph.node(id) {
            Some(node) => End {
                kind: node.kind.raw.as_str().to_string(),
                file: node.source_span.file.as_str().to_string(),
            },
            None => End::default(),
        };
        let edges = |changes: &[specforge_graph::EdgeChange], graph: &Graph| {
            let mut edges: Vec<(DeltaEdge, [End; 2])> = changes
                .iter()
                .map(|e| {
                    let edge = DeltaEdge {
                        source: e.source.clone(),
                        target: e.target.clone(),
                        label: e.label.clone(),
                    };
                    (edge, [ends(graph, &e.source), ends(graph, &e.target)])
                })
                .collect();
            edges.sort_by(|(a, _), (b, _)| a.cmp(b));
            edges.into_iter().unzip::<_, _, Vec<_>, Vec<_>>()
        };
        let (added_edges, added_ends) = edges(&change.added_edges, graph);
        let (removed_edges, removed_ends) = edges(&change.removed_edges, base);
        let (nodes, files) = nodes.into_iter().unzip();
        Delta {
            payload: DeltaPayload {
                base: label,
                nodes,
                added_edges,
                removed_edges,
            },
            files,
            added_ends,
            removed_ends,
        }
    }

    /// The change as it is sent.
    pub fn payload(&self) -> &DeltaPayload {
        &self.payload
    }

    /// Whether nothing changed.
    pub fn is_empty(&self) -> bool {
        self.payload.nodes.is_empty()
            && self.payload.added_edges.is_empty()
            && self.payload.removed_edges.is_empty()
    }

    /// The change a reader of `kinds` is sent: the nodes of those kinds and the edges with an end of one, under the
    /// same base. No kind: no node and no edge.
    pub fn restricted_to(&self, kinds: &[String]) -> DeltaPayload {
        let named = |kind: &str| kinds.iter().any(|k| k == kind);
        let edges = |edges: &[DeltaEdge], ends: &[[End; 2]]| -> Vec<DeltaEdge> {
            edges
                .iter()
                .zip(ends)
                .filter(|(_, ends)| ends.iter().any(|end| named(&end.kind)))
                .map(|(edge, _)| edge.clone())
                .collect()
        };
        DeltaPayload {
            base: self.payload.base.clone(),
            nodes: self
                .payload
                .nodes
                .iter()
                .filter(|node| named(&node.kind))
                .cloned()
                .collect(),
            added_edges: edges(&self.payload.added_edges, &self.added_ends),
            removed_edges: edges(&self.payload.removed_edges, &self.removed_ends),
        }
    }

    /// The change without the nodes declared in a file `dropped` holds and every edge with an end declared in one.
    pub fn without(&self, dropped: impl Fn(&str) -> bool) -> Delta {
        let mut kept = Delta {
            payload: DeltaPayload {
                base: self.payload.base.clone(),
                ..DeltaPayload::default()
            },
            files: Vec::new(),
            added_ends: Vec::new(),
            removed_ends: Vec::new(),
        };
        for (node, file) in self.payload.nodes.iter().zip(&self.files) {
            if !dropped(file) {
                kept.payload.nodes.push(node.clone());
                kept.files.push(file.clone());
            }
        }
        let keeps = |ends: &[End; 2]| !ends.iter().any(|end| dropped(&end.file));
        for (edge, ends) in self.payload.added_edges.iter().zip(&self.added_ends) {
            if keeps(ends) {
                kept.payload.added_edges.push(edge.clone());
                kept.added_ends.push(ends.clone());
            }
        }
        for (edge, ends) in self.payload.removed_edges.iter().zip(&self.removed_ends) {
            if keeps(ends) {
                kept.payload.removed_edges.push(edge.clone());
                kept.removed_ends.push(ends.clone());
            }
        }
        kept
    }
}

/// The wire's name of a graph aspect; a kind change is no aspect (a removal and an addition).
fn aspect_of(aspect: NodeAspect) -> Option<DeltaAspect> {
    match aspect {
        NodeAspect::Kind => None,
        NodeAspect::Title => Some(DeltaAspect::Title),
        NodeAspect::Methods => Some(DeltaAspect::Methods),
        NodeAspect::Obligations => Some(DeltaAspect::Obligations),
        NodeAspect::Verdicts => Some(DeltaAspect::Verdicts),
        NodeAspect::Edges => Some(DeltaAspect::Edges),
    }
}
