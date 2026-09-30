use serde_json::Value;
use specforge_ops::export::{self, Format, Schema};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

/// `specforge.export`: the export `specforge export` writes, through the
/// same function and schema policy (ADR 0004 D3-a). `with_schema` and
/// `no_schema` are the CLI's `--with-schema` and `--no-schema`.
pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("graph");
    // The tool serves the agent formats; dot is `specforge.render`'s.
    let format = match format.parse::<Format>() {
        Ok(Format::Dot) | Err(_) => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                format!("Unknown format: {}", format),
            );
        }
        Ok(format) => format,
    };
    let no_schema = args.get("no_schema").and_then(|v| v.as_bool());
    let with_schema = args.get("with_schema").and_then(|v| v.as_bool());
    let schema = match (no_schema == Some(true), with_schema == Some(true)) {
        (true, _) => Schema::Without,
        (_, true) => Schema::With,
        _ => Schema::Default,
    };
    let request = export::Request {
        format: Some(format),
        scope: args.get("scope").and_then(|v| v.as_str()),
        max_tokens: args
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
        schema,
        ..export::Request::default()
    };

    match crate::operations::export_graph(state, &request) {
        Ok(json_str) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": json_str
                }]
            }),
        ),
        Err(err) => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, err.message),
    }
}
