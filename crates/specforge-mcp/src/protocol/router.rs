use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn route(
    state: &mut McpState,
    method: &str,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    match method {
        // Lifecycle
        "initialize" => crate::lifecycle::handle_initialize(state, params, id),
        "shutdown" => crate::lifecycle::handle_shutdown(state, id),
        "ping" => JsonRpcResponse::success(id, serde_json::json!({})),

        // Listing
        "tools/list" => crate::registry::handle_list_tools(state, id),
        "resources/list" => crate::registry::handle_list_resources(state, id),
        "prompts/list" => crate::registry::handle_list_prompts(state, id),

        // Resources
        "resources/read" => {
            // C9-07: serve a fresh graph when watch has produced a newer
            // snapshot.
            state.refresh_if_stale();
            crate::resources::handle_resource_read(state, params, id)
        }

        "resources/subscribe" => crate::resources::handle_resource_subscribe(state, params, id),
        "resources/unsubscribe" => crate::resources::handle_resource_unsubscribe(state, params, id),

        // Tools
        "tools/call" => {
            state.refresh_if_stale();
            crate::tools::handle_tool_call(state, params, id)
        }

        // Prompts
        "prompts/get" => crate::prompts::handle_prompt_get(state, params, id),

        // Notifications (no response for notifications — id is None)
        "notifications/initialized" => {
            // Client acknowledges initialization, no-op
            JsonRpcResponse::success(id, serde_json::json!({}))
        }
        // MCP's cancellation notification, and the LSP-style request.
        "notifications/cancelled" | "$/cancelRequest" => {
            crate::lifecycle::handle_cancel(state, params, id)
        }

        _ => JsonRpcResponse::error(
            id,
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", method),
        ),
    }
}
