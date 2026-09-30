use serde::Serialize;
use specforge_graph::Graph;
use specforge_registry::{FieldRegistry, KindRegistry, ManifestFieldType};
use std::collections::{BTreeMap, HashSet, VecDeque};

use crate::error::EmitterError;
use crate::json::SCHEMA_VERSION;

#[derive(Debug, Serialize)]
pub struct TraceChain {
    pub entity_id: String,
    pub entity_kind: String,
    pub upstream: Vec<TraceLink>,
    pub downstream: Vec<TraceLink>,
    /// Expected edges the traced entity does not have. Only the traced
    /// entity's: each entity's own trace reports its gaps, so a trace of
    /// every entity lists each gap once, and a trace of one entity is not
    /// buried under the gaps of everything below it.
    pub missing: Vec<MissingLink>,
}

/// Whether a link of a chain exists in the graph (`TraceLinkStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceLinkStatus {
    Resolved,
    Missing,
}

#[derive(Debug, Serialize)]
pub struct TraceLink {
    pub entity_id: String,
    pub entity_kind: String,
    pub edge_label: String,
    pub depth: usize,
    pub status: TraceLinkStatus,
}

/// An edge the registries lead an entity of the chain to have, which the
/// graph does not instantiate. Always has status `missing`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingLink {
    /// The entity that lacks the edge.
    pub from: String,
    pub from_kind: String,
    /// The field that would declare the edge — the graph's edge label.
    pub edge_label: String,
    /// The registered edge type the field instantiates, if it names one.
    pub edge_type: Option<String>,
    /// The kind the edge would point at.
    pub expected_kind: String,
    /// Always 1: the link would leave the traced entity (depth 0).
    pub depth: usize,
    /// The field is required, not just declared.
    pub required: bool,
    pub status: TraceLinkStatus,
}

/// An outgoing edge one kind is expected to have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedEdge {
    /// The field that declares the edge, used as the graph's edge label.
    pub label: String,
    pub edge_type: Option<String>,
    pub target_kind: String,
    pub required: bool,
    /// Labels of the reverse field: an edge with one of them, from an
    /// entity of `target_kind` to this one, is the same relationship
    /// declared from the other side.
    pub inverse_labels: Vec<String>,
}

/// The edges each kind is expected to have, derived from the registries
/// only — the core knows nothing about any kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceExpectations {
    by_kind: BTreeMap<String, Vec<ExpectedEdge>>,
}

impl TraceExpectations {
    /// No expectations: traces flag nothing as missing.
    pub fn new() -> Self {
        Self::default()
    }

    /// A required reference field is always an expected edge. Otherwise a
    /// reference field is expected when:
    /// - the kind it points at is loaded: a field whose target no loaded
    ///   extension declares is an optional peer;
    /// - the extension that declares the kind declares the field: fields
    ///   another extension adds to the kind (entity enhancements) are
    ///   opt-in;
    /// - it points at another kind: a self-relation (`depends_on`,
    ///   `extends`, `superseded_by`...) is a hierarchy or ordering whose
    ///   roots or leaves lack it by construction.
    pub fn from_registries(fields: &FieldRegistry, kinds: &KindRegistry) -> Self {
        let mut expectations = Self::new();
        for (kind, field, entry) in fields.iter() {
            let Some(target) = entry.target_kind.as_deref() else {
                continue;
            };
            let is_reference = matches!(
                entry.field_type,
                ManifestFieldType::Reference | ManifestFieldType::ReferenceList
            );
            let own_field = kinds
                .get(kind)
                .is_some_and(|k| k.source_extension == entry.source_extension);
            if !is_reference
                || !kinds.contains(target)
                || !(own_field || entry.required)
                || (target == kind && !entry.required)
            {
                continue;
            }
            let mut inverse_labels: Vec<String> = entry.inverse_of.iter().cloned().collect();
            inverse_labels.extend(
                fields
                    .fields_for_kind(target)
                    .into_iter()
                    .filter(|other| other.inverse_of.as_deref() == Some(field))
                    .map(|other| other.field_name.clone()),
            );
            inverse_labels.sort();
            inverse_labels.dedup();
            expectations.expect(
                kind,
                ExpectedEdge {
                    label: field.to_string(),
                    edge_type: entry.edge.clone(),
                    target_kind: target.to_string(),
                    required: entry.required,
                    inverse_labels,
                },
            );
        }
        expectations
    }

    /// Expect entities of `kind` to have `edge`.
    pub fn expect(&mut self, kind: &str, edge: ExpectedEdge) {
        let edges = self.by_kind.entry(kind.to_string()).or_default();
        edges.push(edge);
        edges.sort_by(|a, b| a.label.cmp(&b.label));
        edges.dedup_by(|a, b| a.label == b.label);
    }

    /// The edges `kind` is expected to have, ordered by label.
    pub fn for_kind(&self, kind: &str) -> &[ExpectedEdge] {
        self.by_kind.get(kind).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.by_kind.is_empty()
    }
}

/// The chain of `entity_id`, with no expectations: nothing is missing.
pub fn trace(graph: &Graph, entity_id: &str) -> Result<TraceChain, EmitterError> {
    trace_with_expectations(graph, entity_id, &TraceExpectations::new())
}

/// The chain of `entity_id` both ways, with every expected edge the traced
/// entity or anything downstream of it lacks flagged as missing.
pub fn trace_with_expectations(
    graph: &Graph,
    entity_id: &str,
    expectations: &TraceExpectations,
) -> Result<TraceChain, EmitterError> {
    let root = graph.node(entity_id).ok_or_else(|| {
        EmitterError::EntityNotFound(format!(
            "E003: unresolved entity '{}' — not found in graph",
            entity_id
        ))
    })?;

    let upstream = directed_bfs(graph, entity_id, Direction::Upstream);
    let downstream = directed_bfs(graph, entity_id, Direction::Downstream);

    let missing = missing_links(graph, entity_id, expectations);

    Ok(TraceChain {
        entity_id: entity_id.to_string(),
        entity_kind: root.kind.raw.to_string(),
        upstream,
        downstream,
        missing,
    })
}

pub fn trace_all(graph: &Graph) -> Vec<TraceChain> {
    trace_all_with_expectations(graph, &TraceExpectations::new())
}

/// One chain per entity, ordered by entity ID.
pub fn trace_all_with_expectations(
    graph: &Graph,
    expectations: &TraceExpectations,
) -> Vec<TraceChain> {
    let mut chains: Vec<TraceChain> = graph
        .nodes()
        .iter()
        .filter_map(|n| trace_with_expectations(graph, n.id.raw.as_str(), expectations).ok())
        .collect();
    chains.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    chains
}

/// The expected edges `entity_id` does not have, one hop from it. An edge
/// with the expected label counts even when its target is undeclared: that
/// is a broken reference (E003), not a missing link.
fn missing_links(
    graph: &Graph,
    entity_id: &str,
    expectations: &TraceExpectations,
) -> Vec<MissingLink> {
    let Some(node) = graph.node(entity_id) else {
        return Vec::new();
    };
    let kind = node.kind.raw.as_str();
    let outgoing = graph.edges_from(entity_id);
    let incoming = graph.edges_to(entity_id);
    expectations
        .for_kind(kind)
        .iter()
        .filter(|expected| {
            let declared = outgoing.iter().any(|e| e.label == *expected.label);
            let declared_back = incoming.iter().any(|e| {
                expected.inverse_labels.iter().any(|l| e.label == **l)
                    && graph
                        .node(e.source.as_str())
                        .is_some_and(|source| source.kind.raw == *expected.target_kind)
            });
            !declared && !declared_back
        })
        .map(|expected| MissingLink {
            from: entity_id.to_string(),
            from_kind: kind.to_string(),
            edge_label: expected.label.clone(),
            edge_type: expected.edge_type.clone(),
            expected_kind: expected.target_kind.clone(),
            depth: 1,
            required: expected.required,
            status: TraceLinkStatus::Missing,
        })
        .collect()
}

/// Edges whose source or target is not a node of the graph. These are
/// broken references (E003 in check), not missing links of a chain.
pub fn detect_trace_gaps(graph: &Graph) -> Vec<String> {
    let node_ids: HashSet<&str> = graph.nodes().iter().map(|n| n.id.raw.as_str()).collect();
    let mut gaps = Vec::new();
    for edge in graph.edges() {
        if !node_ids.contains(edge.source.as_str()) {
            gaps.push(format!(
                "dangling edge source '{}' in edge {} -> {} ({})",
                edge.source, edge.source, edge.target, edge.label
            ));
        }
        if !node_ids.contains(edge.target.as_str()) {
            gaps.push(format!(
                "dangling edge target '{}' in edge {} -> {} ({})",
                edge.target, edge.source, edge.target, edge.label
            ));
        }
    }
    gaps.sort();
    gaps.dedup();
    gaps
}

pub fn serialize_trace_all(chains: &[TraceChain]) -> Result<String, crate::error::EmitterError> {
    #[derive(Serialize)]
    struct TraceAllOutput<'a> {
        schema_version: &'static str,
        traces: &'a [TraceChain],
    }

    let output = TraceAllOutput {
        schema_version: SCHEMA_VERSION,
        traces: chains,
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| crate::error::EmitterError::SerializationError(e.to_string()))
}

pub fn serialize_trace(chain: &TraceChain) -> Result<String, crate::error::EmitterError> {
    #[derive(Serialize)]
    struct TraceOutput<'a> {
        schema_version: &'static str,
        entity_id: &'a str,
        entity_kind: &'a str,
        upstream: &'a [TraceLink],
        downstream: &'a [TraceLink],
        missing: &'a [MissingLink],
    }

    let output = TraceOutput {
        schema_version: SCHEMA_VERSION,
        entity_id: &chain.entity_id,
        entity_kind: &chain.entity_kind,
        upstream: &chain.upstream,
        downstream: &chain.downstream,
        missing: &chain.missing,
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| crate::error::EmitterError::SerializationError(e.to_string()))
}

/// A chain as terminal text: the entity, then its upstream, downstream and
/// missing links, one per line. Missing links are marked `MISSING`.
pub fn render_trace_human(chain: &TraceChain) -> String {
    let mut out = format!("{} [{}]\n", chain.entity_id, chain.entity_kind);
    let section = |out: &mut String, name: &str, links: &[TraceLink], arrow: fn(&str) -> String| {
        out.push_str(&format!("  {name}:\n"));
        if links.is_empty() {
            out.push_str("    (none)\n");
        }
        for link in links {
            out.push_str(&format!(
                "    {} {} [{}] (depth {})\n",
                arrow(&link.edge_label),
                link.entity_id,
                link.entity_kind,
                link.depth
            ));
        }
    };
    section(&mut out, "upstream", &chain.upstream, |l| format!("<-{l}-"));
    section(&mut out, "downstream", &chain.downstream, |l| {
        format!("-{l}->")
    });
    if !chain.missing.is_empty() {
        out.push_str("  missing:\n");
        for link in &chain.missing {
            out.push_str(&format!(
                "    MISSING {} -{}-> [{}]{} (depth {})\n",
                link.from,
                link.edge_label,
                link.expected_kind,
                if link.required { " required" } else { "" },
                link.depth
            ));
        }
    }
    out
}

enum Direction {
    Upstream,
    Downstream,
}

fn directed_bfs(graph: &Graph, start: &str, direction: Direction) -> Vec<TraceLink> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut links = Vec::new();

    visited.insert(start.to_string());
    queue.push_back((start.to_string(), 0usize));

    while let Some((current, depth)) = queue.pop_front() {
        for edge in graph.edges() {
            let (neighbor, label) = match direction {
                Direction::Upstream => {
                    if edge.target == *current {
                        (&edge.source, &edge.label)
                    } else {
                        continue;
                    }
                }
                Direction::Downstream => {
                    if edge.source == *current {
                        (&edge.target, &edge.label)
                    } else {
                        continue;
                    }
                }
            };

            if visited.insert(neighbor.to_string())
                && let Some(node) = graph.node(neighbor.as_str())
            {
                links.push(TraceLink {
                    entity_id: neighbor.to_string(),
                    entity_kind: node.kind.raw.to_string(),
                    edge_label: label.to_string(),
                    depth: depth + 1,
                    status: TraceLinkStatus::Resolved,
                });
                queue.push_back((neighbor.to_string(), depth + 1));
            }
        }
    }

    links.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.entity_id.cmp(&b.entity_id)));
    links
}
