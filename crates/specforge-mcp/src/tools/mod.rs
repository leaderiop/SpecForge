mod query;
mod validate;
mod export;
mod trace;
mod search;
mod schema;
mod coverage;
mod stats;
mod inspect;
mod find_definition;
mod find_references;
mod outline;
mod suggest_fixes;
mod list;

use serde_json::{json, Value};
use specforge_registry::SurfaceType;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn handle_tool_call(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }

    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: name"),
    };

    let arguments = params.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));

    state.push_event("mcp_tool_invoked", serde_json::json!({"tool": name}));

    let is_mutation = matches!(name,
        "specforge.format" | "specforge.rename" | "specforge.init"
        | "specforge.add_extension" | "specforge.remove_extension" | "specforge.migrate"
    );

    let response = match name {
        // Core tools
        "specforge.query" => query::call(state, arguments, id),
        "specforge.validate" => validate::call(state, arguments, id),
        "specforge.export" => export::call(state, arguments, id),
        "specforge.trace" => trace::call(state, arguments, id),
        "specforge.search" => search::call(state, arguments, id),
        "specforge.schema" => schema::call(state, arguments, id),
        "specforge.coverage" => coverage::call(state, arguments, id),
        "specforge.stats" => stats::call(state, arguments, id),
        // Navigation tools
        "specforge.list" => list::call(state, arguments, id),
        "specforge.inspect" => inspect::call(state, arguments, id),
        "specforge.find_definition" => find_definition::call(state, arguments, id),
        "specforge.find_references" => find_references::call(state, arguments, id),
        "specforge.outline" => outline::call(state, arguments, id),
        "specforge.suggest_fixes" => suggest_fixes::call(state, arguments, id),
        // Operations
        "specforge.format" | "specforge.rename" | "specforge.init"
        | "specforge.add_extension" | "specforge.remove_extension" | "specforge.migrate"
        | "specforge.extensions" | "specforge.providers" | "specforge.doctor"
        | "specforge.collect" | "specforge.render" => {
            crate::operations::handle_operation(state, name, arguments, id)
        }
        _ => {
            // Check if this is a registered extension tool from surface contributions
            let is_extension_tool = state.surface_entries.iter().any(|e| {
                (e.surface_type == SurfaceType::McpTool || e.surface_type == SurfaceType::AutoPromotedTool)
                    && e.contribution_name == name
                    && e.enabled
            });

            if is_extension_tool {
                // Extension tool recognized but requires Wasm runtime for execution
                let msg = format!(
                    "Extension tool '{}' is registered but requires a Wasm runtime for execution. \
                     Use specforge.list or specforge.query for graph-based queries instead.",
                    name
                );
                JsonRpcResponse::success(id, json!({
                    "content": [{ "type": "text", "text": msg }],
                    "isError": true,
                }))
            } else {
                JsonRpcResponse::error(id, error_codes::METHOD_NOT_FOUND, format!("Unknown tool: {}", name))
            }
        }
    };

    if is_mutation && response.error.is_none() {
        state.push_event("mcp_mutation_completed", serde_json::json!({"tool": name}));
    }

    response
}
