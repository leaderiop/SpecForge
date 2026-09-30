use std::collections::BTreeMap;

use crate::resources::{ReadOutcome, ResourceText};
use crate::state::McpState;

pub fn read(state: &McpState) -> ReadOutcome {
    // Derive schema from the graph (same approach as tools/schema.rs)
    // This ensures the schema resource returns actual data even without extension registries
    let mut kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in state.graph.nodes() {
        let entry = kinds.entry(node.kind.raw.to_string()).or_default();
        for field_entry in node.fields.entries() {
            let key_str = field_entry.key.to_string();
            if !entry.contains(&key_str) {
                entry.push(key_str);
            }
        }
    }

    for fields in kinds.values_mut() {
        fields.sort();
    }

    let mut edge_labels: Vec<String> = state
        .graph
        .edges()
        .iter()
        .map(|e| e.label.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    edge_labels.sort();

    let schema = serde_json::json!({
        "schema_version": specforge_emitter::SCHEMA_VERSION,
        "entity_kinds": kinds,
        "edge_labels": edge_labels
    });

    let schema_json = schema.to_string();

    Ok(ResourceText::json("specforge://schema", schema_json))
}
