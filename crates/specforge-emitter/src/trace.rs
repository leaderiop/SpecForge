use crate::types::{TraceChain, TraceGap, TraceLink, TraceReport};
use specforge_common::{EdgeType, EntityKind};
use specforge_graph::SpecGraph;
use std::collections::HashSet;
use std::fmt::Write;

/// Canonical chain edge types for downstream traversal.
/// Deliverable → Capability → Feature → Behavior → Invariant.
const CHAIN_EDGES_DOWNSTREAM: &[(EntityKind, EdgeType)] = &[
    (EntityKind::Deliverable, EdgeType::Bundles),
    (EntityKind::Capability, EdgeType::TracesTo),
    (EntityKind::Feature, EdgeType::Implements),
    (EntityKind::Behavior, EdgeType::References),
];

/// Entity kinds that participate in the canonical trace chain.
const CHAIN_KINDS: &[EntityKind] = &[
    EntityKind::Deliverable,
    EntityKind::Capability,
    EntityKind::Feature,
    EntityKind::Behavior,
    EntityKind::Invariant,
];

fn is_chain_kind(kind: &EntityKind) -> bool {
    CHAIN_KINDS.contains(kind)
}

/// Returns the expected downstream edge type for a chain kind, if any.
fn chain_downstream_edge(kind: &EntityKind) -> Option<EdgeType> {
    CHAIN_EDGES_DOWNSTREAM
        .iter()
        .find(|(k, _)| k == kind)
        .map(|(_, e)| *e)
}

/// Returns true if this kind is the top of the chain (no expected upstream).
fn is_chain_top(kind: &EntityKind) -> bool {
    kind == &EntityKind::Deliverable
}

/// Walk upstream from a node, following incoming edges.
fn walk_upstream(
    graph: &SpecGraph,
    id: &str,
    visited: &mut HashSet<String>,
) -> (Vec<TraceLink>, Vec<TraceGap>) {
    let mut links = Vec::new();
    let mut gaps = Vec::new();

    let node = match graph.get_node(id) {
        Some(n) => n,
        None => return (links, gaps),
    };
    let kind = &node.kind;

    let incoming = graph.incoming_edges(id);

    if is_chain_kind(kind) {
        // For chain kinds, check if there's at least one upstream chain connection
        let has_chain_upstream = incoming
            .iter()
            .any(|(src, _)| is_chain_kind(&src.kind));

        if !has_chain_upstream && !is_chain_top(kind) {
            gaps.push(TraceGap {
                entity_id: id.to_string(),
                entity_kind: kind.keyword().to_string(),
                missing_direction: "upstream".to_string(),
            });
        }
    }

    // Collect and sort for determinism
    let mut upstream_targets: Vec<_> = incoming
        .iter()
        .map(|(src, edge)| {
            (
                src.id.raw().to_string(),
                src.kind.clone(),
                edge.edge_type.label().to_string(),
            )
        })
        .collect();
    upstream_targets.sort_by(|a, b| a.0.cmp(&b.0));

    for (src_id, _src_kind, edge_label) in &upstream_targets {
        if visited.contains(src_id.as_str()) {
            continue;
        }

        links.push(TraceLink {
            from: src_id.clone(),
            to: id.to_string(),
            edge_type: edge_label.clone(),
            depth: 0, // will be set later
        });

        visited.insert(src_id.clone());
        let (mut parent_links, parent_gaps) = walk_upstream(graph, src_id, visited);
        links.append(&mut parent_links);
        gaps.extend(parent_gaps);
    }

    (links, gaps)
}

/// Walk downstream from a node, following outgoing edges.
fn walk_downstream(
    graph: &SpecGraph,
    id: &str,
    visited: &mut HashSet<String>,
) -> (Vec<TraceLink>, Vec<TraceGap>) {
    let mut links = Vec::new();
    let mut gaps = Vec::new();

    let node = match graph.get_node(id) {
        Some(n) => n,
        None => return (links, gaps),
    };
    let kind = &node.kind;

    let outgoing = graph.outgoing_edges(id);

    if is_chain_kind(kind) {
        // For chain kinds, check if there's expected downstream
        if let Some(expected_edge) = chain_downstream_edge(kind) {
            let has_chain_downstream = outgoing
                .iter()
                .any(|(_, edge)| edge.edge_type == expected_edge);

            if !has_chain_downstream {
                gaps.push(TraceGap {
                    entity_id: id.to_string(),
                    entity_kind: kind.keyword().to_string(),
                    missing_direction: "downstream".to_string(),
                });
            }
        }
        // Invariant is terminal — no gap check needed (no chain_downstream_edge)
    }

    // Collect and sort for determinism
    let mut downstream_targets: Vec<_> = outgoing
        .iter()
        .map(|(tgt, edge)| {
            (
                tgt.id.raw().to_string(),
                tgt.kind.clone(),
                edge.edge_type.label().to_string(),
            )
        })
        .collect();
    downstream_targets.sort_by(|a, b| a.0.cmp(&b.0));

    for (tgt_id, _tgt_kind, edge_label) in &downstream_targets {
        if visited.contains(tgt_id.as_str()) {
            continue;
        }

        links.push(TraceLink {
            from: id.to_string(),
            to: tgt_id.clone(),
            edge_type: edge_label.clone(),
            depth: 0,
        });

        visited.insert(tgt_id.clone());
        let (mut child_links, child_gaps) = walk_downstream(graph, tgt_id, visited);
        links.append(&mut child_links);
        gaps.extend(child_gaps);
    }

    (links, gaps)
}

/// Compute trace chain for a single entity (BEH-SF-063).
/// Returns None if entity_id not found in graph.
pub fn compute_trace(graph: &SpecGraph, entity_id: &str) -> Option<TraceChain> {
    let node = graph.get_node(entity_id)?;
    let root_kind = node.kind.clone();

    let mut up_visited = HashSet::new();
    up_visited.insert(entity_id.to_string());
    let (upstream, mut gaps) = walk_upstream(graph, entity_id, &mut up_visited);

    let mut down_visited = HashSet::new();
    down_visited.insert(entity_id.to_string());
    let (downstream, down_gaps) = walk_downstream(graph, entity_id, &mut down_visited);
    gaps.extend(down_gaps);

    // Deduplicate gaps by entity_id + direction
    let mut seen_gaps = HashSet::new();
    gaps.retain(|g| seen_gaps.insert((g.entity_id.clone(), g.missing_direction.clone())));

    Some(TraceChain {
        root: entity_id.to_string(),
        root_kind: root_kind.keyword().to_string(),
        upstream,
        downstream,
        gaps,
    })
}

/// Compute full traceability report over all deliverables (BEH-SF-067).
/// Also includes orphan chains (entities not reachable from any deliverable).
pub fn compute_full_trace(graph: &SpecGraph) -> TraceReport {
    let mut chains = Vec::new();
    let mut all_visited = HashSet::new();

    // Start from all deliverables
    let mut deliverables: Vec<_> = graph
        .nodes_of_kind(EntityKind::Deliverable)
        .iter()
        .map(|n| n.id.raw().to_string())
        .collect();
    deliverables.sort();

    for dlv_id in &deliverables {
        let mut down_visited = HashSet::new();
        down_visited.insert(dlv_id.clone());
        let (downstream, gaps) = walk_downstream(graph, dlv_id, &mut down_visited);

        all_visited.extend(down_visited);

        let node = graph.get_node(dlv_id).unwrap();
        chains.push(TraceChain {
            root: dlv_id.clone(),
            root_kind: node.kind.keyword().to_string(),
            upstream: Vec::new(),
            downstream,
            gaps,
        });
    }

    // Find canonical-chain entities not visited by any deliverable chain
    for kind in CHAIN_KINDS {
        let mut nodes: Vec<_> = graph
            .nodes_of_kind(kind.clone())
            .iter()
            .map(|n| n.id.raw().to_string())
            .collect();
        nodes.sort();

        for id in nodes {
            if all_visited.contains(&id) {
                continue;
            }
            all_visited.insert(id.clone());

            if let Some(chain) = compute_trace(graph, &id) {
                chains.push(chain);
            }
        }
    }

    let total_gaps: usize = chains.iter().map(|c| c.gaps.len()).sum();

    TraceReport { chains, total_gaps }
}

/// Format a single trace chain for terminal output.
pub fn format_trace(chain: &TraceChain) -> String {
    let mut out = String::new();
    writeln!(out, "Trace: {} ({})", chain.root, chain.root_kind).unwrap();

    if chain.upstream.is_empty() {
        writeln!(out, "  upstream: (none)").unwrap();
    } else {
        writeln!(out, "  upstream:").unwrap();
        for link in &chain.upstream {
            let to_kind = link.to.as_str();
            let from_kind = link.from.as_str();
            // Show: FROM (kind) --edge--> TO
            writeln!(
                out,
                "    {} --{}--> {}",
                from_kind, link.edge_type, to_kind
            )
            .unwrap();
        }
    }

    if chain.downstream.is_empty() {
        writeln!(out, "  downstream: (none)").unwrap();
    } else {
        writeln!(out, "  downstream:").unwrap();
        for link in &chain.downstream {
            writeln!(
                out,
                "    {} --{}--> {}",
                link.from, link.edge_type, link.to
            )
            .unwrap();
        }
    }

    if chain.gaps.is_empty() {
        writeln!(out, "  gaps: none").unwrap();
    } else {
        writeln!(out, "  gaps:").unwrap();
        for gap in &chain.gaps {
            writeln!(
                out,
                "    [GAP] {} ({}): no {} connection",
                gap.entity_id, gap.entity_kind, gap.missing_direction
            )
            .unwrap();
        }
    }

    out
}

/// Format a full trace report for terminal output.
pub fn format_trace_report(report: &TraceReport) -> String {
    let mut out = String::new();
    writeln!(out, "Traceability Report").unwrap();
    writeln!(out, "===================").unwrap();

    for chain in &report.chains {
        writeln!(out).unwrap();
        writeln!(out, "Chain: {} ({})", chain.root, chain.root_kind).unwrap();

        if chain.downstream.is_empty() && chain.upstream.is_empty() {
            writeln!(out, "  (no connections)").unwrap();
        }

        for link in &chain.upstream {
            writeln!(
                out,
                "  {} --{}--> {}",
                link.from, link.edge_type, link.to
            )
            .unwrap();
        }
        for link in &chain.downstream {
            writeln!(
                out,
                "  {} --{}--> {}",
                link.from, link.edge_type, link.to
            )
            .unwrap();
        }

        for gap in &chain.gaps {
            writeln!(
                out,
                "  [GAP] {} ({}): no {} connection",
                gap.entity_id, gap.entity_kind, gap.missing_direction
            )
            .unwrap();
        }
    }

    writeln!(out).unwrap();
    writeln!(
        out,
        "Summary: {} chains, {} gaps",
        report.chains.len(),
        report.total_gaps
    )
    .unwrap();

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{EntityId, SourceSpan};
    use specforge_graph::{GraphEdge, GraphNode, SpecGraph};

    fn make_node(id: &str, kind: EntityKind, title: &str) -> GraphNode {
        GraphNode {
            id: EntityId::parse(id),
            kind,
            title: Some(title.to_string()),
            file: "test.spec".to_string(),
            span: SourceSpan::new("test.spec", 1, 1, 1, 1),
        }
    }

    fn make_edge(edge_type: EdgeType, field: &str) -> GraphEdge {
        GraphEdge {
            edge_type,
            field_name: field.to_string(),
        }
    }

    /// Build a full chain: DLV → CAP → FEAT → BEH → INV
    fn make_full_chain() -> SpecGraph {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("data_integrity", EntityKind::Invariant, "Data Integrity"));
        graph.add_node(make_node("validate_input", EntityKind::Behavior, "Validate"));
        graph.add_node(make_node("input_validation", EntityKind::Feature, "Validation"));
        graph.add_node(make_node("chain_data_entry", EntityKind::Capability, "Data Entry"));
        graph.add_node(make_node("chain_cli_tool", EntityKind::Deliverable, "CLI Tool"));

        graph.add_edge("validate_input", "data_integrity", make_edge(EdgeType::References, "invariants"));
        graph.add_edge("input_validation", "validate_input", make_edge(EdgeType::Implements, "behaviors"));
        graph.add_edge("chain_data_entry", "input_validation", make_edge(EdgeType::TracesTo, "features"));
        graph.add_edge("chain_cli_tool", "chain_data_entry", make_edge(EdgeType::Bundles, "capabilities"));
        graph
    }

    #[test]
    fn trace_behavior_shows_upstream_and_downstream() {
        let graph = make_full_chain();
        let chain = compute_trace(&graph, "validate_input").unwrap();
        assert_eq!(chain.root, "validate_input");
        assert_eq!(chain.root_kind, "behavior");

        // Upstream: FEAT → BEH, CAP → FEAT, DLV → CAP
        assert!(!chain.upstream.is_empty());
        assert!(chain.upstream.iter().any(|l| l.from == "input_validation" && l.to == "validate_input"));

        // Downstream: BEH → INV
        assert!(!chain.downstream.is_empty());
        assert!(chain.downstream.iter().any(|l| l.from == "validate_input" && l.to == "data_integrity"));
    }

    #[test]
    fn trace_invariant_upstream_only() {
        let graph = make_full_chain();
        let chain = compute_trace(&graph, "data_integrity").unwrap();
        assert_eq!(chain.root_kind, "invariant");
        assert!(!chain.upstream.is_empty());
        // Invariant is terminal — no downstream
        assert!(chain.downstream.is_empty());
    }

    #[test]
    fn trace_deliverable_downstream_only() {
        let graph = make_full_chain();
        let chain = compute_trace(&graph, "chain_cli_tool").unwrap();
        assert_eq!(chain.root_kind, "deliverable");
        assert!(chain.upstream.is_empty());
        assert!(!chain.downstream.is_empty());
    }

    #[test]
    fn trace_entity_not_found() {
        let graph = make_full_chain();
        assert!(compute_trace(&graph, "NONEXISTENT").is_none());
    }

    #[test]
    fn trace_orphan_entity() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("validate_input", EntityKind::Behavior, "Orphan"));
        let chain = compute_trace(&graph, "validate_input").unwrap();
        assert!(chain.upstream.is_empty());
        assert!(chain.downstream.is_empty());
        // Should have gaps in both directions
        assert!(chain.gaps.iter().any(|g| g.missing_direction == "upstream"));
        assert!(chain.gaps.iter().any(|g| g.missing_direction == "downstream"));
    }

    #[test]
    fn trace_gap_missing_invariant() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("input_validation", EntityKind::Feature, "F1"));
        graph.add_node(make_node("validate_input", EntityKind::Behavior, "B1"));
        graph.add_edge("input_validation", "validate_input", make_edge(EdgeType::Implements, "behaviors"));

        let chain = compute_trace(&graph, "validate_input").unwrap();
        // BEH has no invariant downstream — gap
        assert!(chain.gaps.iter().any(|g| {
            g.entity_id == "validate_input" && g.missing_direction == "downstream"
        }));
    }

    #[test]
    fn trace_gap_missing_upstream() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("input_validation", EntityKind::Feature, "F1"));
        graph.add_node(make_node("validate_input", EntityKind::Behavior, "B1"));
        graph.add_edge("input_validation", "validate_input", make_edge(EdgeType::Implements, "behaviors"));

        let chain = compute_trace(&graph, "input_validation").unwrap();
        // FEAT has no upstream capability — gap
        assert!(chain.gaps.iter().any(|g| {
            g.entity_id == "input_validation" && g.missing_direction == "upstream"
        }));
    }

    #[test]
    fn trace_deterministic() {
        let graph = make_full_chain();
        let a = compute_trace(&graph, "validate_input").unwrap();
        let b = compute_trace(&graph, "validate_input").unwrap();
        let a_json = serde_json::to_string(&a).unwrap();
        let b_json = serde_json::to_string(&b).unwrap();
        assert_eq!(a_json, b_json);
    }

    #[test]
    fn full_trace_covers_all_deliverables() {
        let graph = make_full_chain();
        let report = compute_full_trace(&graph);
        assert!(report.chains.iter().any(|c| c.root == "chain_cli_tool"));
    }

    #[test]
    fn full_trace_flags_gaps() {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("chain_cli_tool", EntityKind::Deliverable, "CLI Tool"));
        graph.add_node(make_node("chain_data_entry", EntityKind::Capability, "Data Entry"));
        graph.add_node(make_node("search_feature", EntityKind::Feature, "Orphan Feature"));
        graph.add_edge("chain_cli_tool", "chain_data_entry", make_edge(EdgeType::Bundles, "capabilities"));

        let report = compute_full_trace(&graph);
        assert!(report.total_gaps > 0);
        // search_feature should appear as an orphan chain
        assert!(report.chains.iter().any(|c| c.root == "search_feature"));
    }

    #[test]
    fn full_trace_empty_graph() {
        let graph = SpecGraph::new();
        let report = compute_full_trace(&graph);
        assert!(report.chains.is_empty());
        assert_eq!(report.total_gaps, 0);
    }

    #[test]
    fn format_trace_snapshot() {
        let graph = make_full_chain();
        let chain = compute_trace(&graph, "validate_input").unwrap();
        let output = format_trace(&chain);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn format_report_snapshot() {
        let graph = make_full_chain();
        let report = compute_full_trace(&graph);
        let output = format_trace_report(&report);
        insta::assert_snapshot!(output);
    }
}
