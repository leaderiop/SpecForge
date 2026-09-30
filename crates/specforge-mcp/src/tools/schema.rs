use serde_json::Value;
use std::collections::BTreeMap;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let kind_filter = args.get("kind").and_then(|v| v.as_str());

    let mut kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in state.graph.nodes() {
        if let Some(filter) = kind_filter
            && node.kind.raw != filter
        {
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

    let mut edge_labels: Vec<String> = state
        .graph
        .edges()
        .iter()
        .map(|e| e.label.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    edge_labels.sort();

    let include_edges = args
        .get("include_edges")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let include_validation_rules = args
        .get("include_validation_rules")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut schema = serde_json::json!({
        "schema_version": specforge_emitter::SCHEMA_VERSION,
        "entity_kinds": kinds,
    });
    if include_edges {
        schema["edge_labels"] = serde_json::json!(edge_labels);
    }
    if include_validation_rules {
        // The rules each loaded extension declares, tagged with its name.
        let rules: Vec<Value> = state
            .manifests
            .iter()
            .flat_map(|manifest| {
                manifest.validation_rules.iter().filter_map(|rule| {
                    let mut rule = serde_json::to_value(rule).ok()?;
                    rule["extension"] = Value::from(manifest.name.as_str());
                    Some(rule)
                })
            })
            .filter(|rule| kind_filter.is_none_or(|kind| rule["targetKind"].as_str() == Some(kind)))
            .collect();
        schema["validation_rules"] = Value::Array(rules);
    }

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": schema.to_string()
            }]
        }),
    )
}
