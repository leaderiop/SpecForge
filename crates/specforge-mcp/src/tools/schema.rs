use serde_json::Value;
use std::collections::BTreeMap;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let kind_filter = args.get("kind").and_then(|v| v.as_str());

    let mut kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in state.graph.nodes() {
        if let Some(filter) = kind_filter
            && node.kind.raw != filter {
                continue;
            }
        let entry = kinds.entry(node.kind.raw.to_string()).or_default();
        for field_entry in node.fields.entries() {
            let key_str = field_entry.key.to_string();
            if !entry.contains(&key_str) {
                entry.push(key_str);
            }
        }
    }

    // Sort field names within each kind
    for fields in kinds.values_mut() {
        fields.sort();
    }

    let mut edge_labels: Vec<String> = state.graph.edges().iter()
        .map(|e| e.label.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    edge_labels.sort();

    let schema = serde_json::json!({
        "schema_version": "0.1.0",
        "entity_kinds": kinds,
        "edge_labels": edge_labels
    });

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": schema.to_string()
        }]
    }))
}
