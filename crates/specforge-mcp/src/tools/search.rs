use serde_json::Value;
use strsim::jaro_winkler;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: query"),
    };

    let kind_filter: Vec<&str> = args.get("kinds")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let field_filter = args.get("field").and_then(|v| v.as_str());
    let value_filter = args.get("value").and_then(|v| v.as_str());
    let references_target = args.get("references").and_then(|v| v.as_str());

    // If references parameter is set, find entities with edges to that target
    if let Some(target) = references_target {
        let refs = state.graph.edges_to(target);
        let results: Vec<Value> = refs.iter()
            .filter_map(|e| state.graph.node(e.source.as_str()))
            .map(|n| {
                serde_json::json!({
                    "entity_id": n.id.raw,
                    "kind": n.kind.raw,
                    "title": n.title,
                    "file_path": n.source_span.file,
                    "line": n.source_span.start_line,
                    "score": 1.0
                })
            })
            .collect();

        return JsonRpcResponse::success(id, serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results).unwrap()
            }]
        }));
    }

    let query_lower = query.to_lowercase();

    let mut scored: Vec<(f64, &specforge_graph::Node)> = state.graph.nodes().into_iter()
        .filter(|n| kind_filter.is_empty() || kind_filter.contains(&n.kind.raw.as_str()))
        .filter(|n| {
            if let (Some(f), Some(v)) = (field_filter, value_filter) {
                n.fields.get(f)
                    .map(|fv| format!("{:?}", fv).to_lowercase().contains(&v.to_lowercase()))
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .map(|n| {
            let id_score = jaro_winkler(&query_lower, &n.id.raw.as_str().to_lowercase());
            let title_score = n.title.as_ref()
                .map(|t| jaro_winkler(&query_lower, &t.to_lowercase()))
                .unwrap_or(0.0);
            let score = id_score.max(title_score);
            (score, n)
        })
        .filter(|(score, _)| *score > 0.6)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let results: Vec<Value> = scored.iter().map(|(score, n)| {
        serde_json::json!({
            "entity_id": n.id.raw,
            "kind": n.kind.raw,
            "title": n.title,
            "file_path": n.source_span.file,
            "line": n.source_span.start_line,
            "score": score
        })
    }).collect();

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&results).unwrap()
        }]
    }))
}
