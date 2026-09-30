use serde_json::Value;
use specforge_emitter::{EmitFormat, EmitOptions, emit};

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &McpState, args: Value) -> ToolOutcome {
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("graph");
    let scope = args.get("scope").and_then(|v| v.as_str());

    let fmt = match format {
        "context" => EmitFormat::Context,
        "brief" => EmitFormat::Brief,
        "graph" => EmitFormat::Json,
        _ => {
            return ToolOutcome::invalid_params(format!("Unknown format: {}", format));
        }
    };

    let max_tokens = args
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let options = EmitOptions {
        format: fmt,
        scope,
        token_budget: max_tokens,
        field_registry: Some(&state.field_registry),
        ..EmitOptions::default()
    };

    match emit(&state.graph, &options) {
        Ok(json_str) => ToolOutcome::text(json_str),
        Err(err) => ToolOutcome::invalid_params(err.to_string()),
    }
}
