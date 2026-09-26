use serde_json::Value;
use specforge_emitter::{EmitFormat, EmitOptions, emit};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn read(state: &McpState, uri: &str, id: Option<Value>) -> JsonRpcResponse {
    // C9-03: context/brief accept an optional token budget via the resource
    // URI (specforge://context?max_tokens=4000). The graph is trimmed to fit
    // (least-connected nodes dropped first) before emission.
    let max_tokens = uri
        .split('?')
        .nth(1)
        .and_then(|query| query.split('&').find(|kv| kv.starts_with("max_tokens=")))
        .and_then(|kv| kv.split('=').nth(1))
        .and_then(|v| v.parse::<usize>().ok());

    let graph_snapshot = match max_tokens {
        Some(budget) => specforge_emitter::filter_graph_within_budget(&state.graph, budget, |g| {
            emit(
                g,
                &EmitOptions {
                    format: EmitFormat::Context,
                    ..EmitOptions::default()
                },
            )
            .expect("budgeted emit cannot fail")
        }),
        None => state.graph.clone(),
    };

    let options = EmitOptions {
        format: EmitFormat::Context,
        ..EmitOptions::default()
    };
    let json_str = emit(&graph_snapshot, &options).expect("budgeted graph emit cannot fail");
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "contents": [{
                "uri": "specforge://context",
                "mimeType": "application/json",
                "text": json_str
            }]
        }),
    )
}
