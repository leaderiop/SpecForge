use serde_json::Value;
use specforge_emitter::outline::{
    render, DependencyDepth, OutlineDetail, OutlineFormat, OutlineIntermediate_from_manifests,
    OutlineOptions,
};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("json");
    let fields = args.get("fields").and_then(|v| v.as_str()).unwrap_or("keys");
    let deps = args.get("deps").and_then(|v| v.as_str()).unwrap_or("direct");

    let outline_format = match format {
        "markdown" => OutlineFormat::Markdown,
        "mermaid" => OutlineFormat::Mermaid,
        "dot" => OutlineFormat::Dot,
        "json" => OutlineFormat::Json,
        _ => return JsonRpcResponse::error(
            id, error_codes::INVALID_PARAMS,
            format!("Unknown format: {}. Expected: markdown, mermaid, dot, json", format),
        ),
    };

    let detail = match fields {
        "none" => OutlineDetail::None,
        "keys" => OutlineDetail::Keys,
        "all" => OutlineDetail::All,
        _ => return JsonRpcResponse::error(
            id, error_codes::INVALID_PARAMS,
            format!("Unknown fields: {}. Expected: none, keys, all", fields),
        ),
    };

    let dep_depth = match deps {
        "direct" => DependencyDepth::Direct,
        "effective" => DependencyDepth::Effective,
        "full" => DependencyDepth::Full,
        _ => return JsonRpcResponse::error(
            id, error_codes::INVALID_PARAMS,
            format!("Unknown deps: {}. Expected: direct, effective, full", deps),
        ),
    };

    let options = OutlineOptions {
        format: outline_format,
        detail,
        deps: dep_depth,
    };

    let outline = OutlineIntermediate_from_manifests(&state.manifests);
    let output = render(&outline, &options);

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}
