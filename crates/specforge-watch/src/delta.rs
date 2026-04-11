use serde::Serialize;
use specforge_graph::Graph;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
pub struct NodeChange {
    pub id: String,
    pub kind: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModifiedNodeChange {
    pub id: String,
    pub changed_fields: Vec<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeChange {
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphDelta {
    pub added_nodes: Vec<NodeChange>,
    pub removed_nodes: Vec<NodeChange>,
    pub modified_nodes: Vec<ModifiedNodeChange>,
    pub added_edges: Vec<EdgeChange>,
    pub removed_edges: Vec<EdgeChange>,
    pub affected_files: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DeltaConfig {
    pub include_values: bool,
}

pub fn compute_graph_delta(old: &Graph, new: &Graph) -> GraphDelta {
    compute_graph_delta_with_config(old, new, &DeltaConfig::default())
}

pub fn compute_graph_delta_with_config(old: &Graph, new: &Graph, config: &DeltaConfig) -> GraphDelta {
    let old_ids: BTreeSet<&str> = old.nodes().iter().map(|n| n.id.raw.as_str()).collect();
    let new_ids: BTreeSet<&str> = new.nodes().iter().map(|n| n.id.raw.as_str()).collect();

    let mut added_nodes = Vec::new();
    let mut removed_nodes = Vec::new();
    let mut modified_nodes = Vec::new();
    let mut affected_files = BTreeSet::new();

    // Added nodes
    for &id in new_ids.difference(&old_ids) {
        if let Some(node) = new.node(id) {
            affected_files.insert(node.source_span.file.to_string());
            added_nodes.push(NodeChange {
                id: node.id.raw.to_string(),
                kind: node.kind.raw.to_string(),
                file: Some(node.source_span.file.to_string()),
                line: Some(node.source_span.start_line),
            });
        }
    }

    // Removed nodes
    for &id in old_ids.difference(&new_ids) {
        if let Some(node) = old.node(id) {
            affected_files.insert(node.source_span.file.to_string());
            removed_nodes.push(NodeChange {
                id: node.id.raw.to_string(),
                kind: node.kind.raw.to_string(),
                file: Some(node.source_span.file.to_string()),
                line: Some(node.source_span.start_line),
            });
        }
    }

    // Modified nodes: same ID in both, but different fields
    for &id in old_ids.intersection(&new_ids) {
        if let (Some(old_node), Some(new_node)) = (old.node(id), new.node(id)) {
            let mut changed_fields = Vec::new();

            // Collect all field keys from both
            let mut all_keys = BTreeSet::new();
            for entry in old_node.fields.entries() {
                all_keys.insert(entry.key.to_string());
            }
            for entry in new_node.fields.entries() {
                all_keys.insert(entry.key.to_string());
            }

            for key in &all_keys {
                let old_val = old_node.fields.get(key);
                let new_val = new_node.fields.get(key);
                // Compare serialized form for simplicity
                let old_json = old_val.map(|v| serde_json::to_value(v).unwrap_or_default());
                let new_json = new_val.map(|v| serde_json::to_value(v).unwrap_or_default());
                if old_json != new_json {
                    changed_fields.push(key.clone());
                }
            }

            // Also check title change
            if old_node.title != new_node.title {
                changed_fields.push("title".to_string());
            }

            if !changed_fields.is_empty() {
                affected_files.insert(new_node.source_span.file.to_string());
                changed_fields.sort();

                let (old_value, new_value) = if config.include_values {
                    (
                        serde_json::to_value(&old_node.fields).ok(),
                        serde_json::to_value(&new_node.fields).ok(),
                    )
                } else {
                    (None, None)
                };

                modified_nodes.push(ModifiedNodeChange {
                    id: id.to_string(),
                    changed_fields,
                    old_value,
                    new_value,
                    file: Some(new_node.source_span.file.to_string()),
                    line: Some(new_node.source_span.start_line),
                });
            }
        }
    }

    // Edge diffing
    let old_edge_set: BTreeSet<EdgeChange> = old
        .edges()
        .iter()
        .map(|e| EdgeChange {
            source: e.source.to_string(),
            target: e.target.to_string(),
            label: e.label.to_string(),
        })
        .collect();

    let new_edge_set: BTreeSet<EdgeChange> = new
        .edges()
        .iter()
        .map(|e| EdgeChange {
            source: e.source.to_string(),
            target: e.target.to_string(),
            label: e.label.to_string(),
        })
        .collect();

    let added_edges: Vec<EdgeChange> = new_edge_set.difference(&old_edge_set).cloned().collect();
    let removed_edges: Vec<EdgeChange> = old_edge_set.difference(&new_edge_set).cloned().collect();

    // Sort by id for deterministic output
    added_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    removed_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    modified_nodes.sort_by(|a, b| a.id.cmp(&b.id));

    GraphDelta {
        added_nodes,
        removed_nodes,
        modified_nodes,
        added_edges,
        removed_edges,
        affected_files: affected_files.into_iter().collect(),
    }
}

/// Successful delta validation result with summary counts.
#[derive(Debug, Clone)]
pub struct DeltaValidationResult {
    pub node_count: usize,
    pub edge_count: usize,
}

/// Conditionally verify delta correctness. When `enabled` is false, returns Ok immediately
/// without performing any validation. In production, this is gated by `cfg(debug_assertions)`
/// or the `--verify-incremental` CLI flag.
pub fn validate_delta_correctness_if_enabled(
    old: &Graph,
    new: &Graph,
    delta: &GraphDelta,
    enabled: bool,
) -> Result<DeltaValidationResult, String> {
    if !enabled {
        return Ok(DeltaValidationResult {
            node_count: new.node_count(),
            edge_count: new.edge_count(),
        });
    }
    validate_delta_correctness(old, new, delta)
}

/// Verify that a delta correctly describes the difference between old and new graphs.
/// Returns Ok(result) with counts if consistent, or Err(message) describing the discrepancy.
pub fn validate_delta_correctness(
    old: &Graph,
    new: &Graph,
    delta: &GraphDelta,
) -> Result<DeltaValidationResult, String> {
    // Simulate applying delta to old graph and compare with new graph
    let old_count = old.node_count();
    let expected_count = old_count + delta.added_nodes.len() - delta.removed_nodes.len();
    let actual_count = new.node_count();

    if expected_count != actual_count {
        return Err(format!(
            "node count mismatch: old({}) + added({}) - removed({}) = {}, but new graph has {}",
            old_count,
            delta.added_nodes.len(),
            delta.removed_nodes.len(),
            expected_count,
            actual_count,
        ));
    }

    let old_edge_count = old.edge_count();
    let expected_edge_count = old_edge_count + delta.added_edges.len() - delta.removed_edges.len();
    let actual_edge_count = new.edge_count();

    if expected_edge_count != actual_edge_count {
        return Err(format!(
            "edge count mismatch: old({}) + added({}) - removed({}) = {}, but new graph has {}",
            old_edge_count,
            delta.added_edges.len(),
            delta.removed_edges.len(),
            expected_edge_count,
            actual_edge_count,
        ));
    }

    // Verify added nodes exist in new graph
    for node_change in &delta.added_nodes {
        if new.node(&node_change.id).is_none() {
            return Err(format!(
                "added node '{}' not found in new graph",
                node_change.id
            ));
        }
    }

    // Verify removed nodes don't exist in new graph
    for node_change in &delta.removed_nodes {
        if new.node(&node_change.id).is_some() {
            return Err(format!(
                "removed node '{}' still present in new graph",
                node_change.id
            ));
        }
    }

    Ok(DeltaValidationResult {
        node_count: actual_count,
        edge_count: actual_edge_count,
    })
}
