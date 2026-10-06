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

        // Listing: an environment change on disk changes the extension
        // tools, resources and prompts listed.
        "tools/list" => {
            fresh(state);
            crate::registry::handle_list_tools(state, id)
        }
        "resources/list" => {
            fresh(state);
            crate::registry::handle_list_resources(state, id)
        }
        "resources/templates/list" => crate::registry::handle_list_resource_templates(state, id),
        "prompts/list" => {
            fresh(state);
            crate::registry::handle_list_prompts(state, id)
        }

        // Resources: the read's target brings the project up to date.
        "resources/read" => crate::resources::handle_resource_read(state, params, id),

        "resources/subscribe" => crate::resources::handle_resource_subscribe(state, params, id),
        "resources/unsubscribe" => crate::resources::handle_resource_unsubscribe(state, params, id),

        // Tools
        // Tools: each call's target (its entry's reach and freshness)
        // brings the project it acts on up to date.
        "tools/call" => crate::tools::handle_tool_call(state, params, id),

        // Prompts: as tools.
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

/// Every request that reads the project first brings it up to date with
/// disk (`mcp_served_project_consistency`), once initialized.
fn fresh(state: &mut McpState) {
    if state.is_initialized() {
        state.ensure_fresh();
    }
}
