use serde_json::Value;
use std::path::PathBuf;

use specforge_common::load_project_config;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::registry::register_defaults;
use crate::state::{McpState, ServerPhase};
use crate::types::{
    McpCapabilities, McpCapabilityFlags, McpPromptCapability, McpResourceCapability, McpServerInfo,
    McpToolCapability,
};

/// The protocol revisions the server speaks, latest first.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];
/// The revision the server answers a client it can't match with.
pub const LATEST_PROTOCOL_VERSION: &str = PROTOCOL_VERSIONS[0];
/// The one revision with JSON-RPC batching; 2025-06-18 removed it.
pub const BATCHING_PROTOCOL_VERSION: &str = "2025-03-26";
/// The first revision with `structuredContent` in tool results.
pub const STRUCTURED_CONTENT_PROTOCOL_VERSION: &str = "2025-06-18";

/// The revision to speak with a client that asked for `requested`: that
/// one when the server speaks it, else the latest (MCP lifecycle, version
/// negotiation).
pub fn negotiate_protocol_version(requested: Option<&str>) -> &'static str {
    PROTOCOL_VERSIONS
        .into_iter()
        .find(|v| Some(*v) == requested)
        .unwrap_or(LATEST_PROTOCOL_VERSION)
}

pub fn handle_initialize(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    if state.phase == ServerPhase::Initialized {
        state.push_event(
            "mcp_initialization_failed",
            serde_json::json!({
                "error_kind": "already_initialized",
                "message": "Server already initialized",
            }),
        );
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_REQUEST,
            "Server already initialized",
        );
    }

    let project_root = params
        .get("projectRoot")
        .or_else(|| params.get("project_root"))
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.default_project_root.clone());

    // Compile and serve the project when there is one: its registries,
    // and the core tools, resources and prompts plus what its extensions
    // contribute. Subscribed clients learn what it changed (C9-01).
    match &project_root {
        Some(root) if root.exists() => state.recompile(root),
        _ => {
            register_defaults(state);
            if let Some(root) = &project_root {
                state.project_config = load_project_config(root);
            }
        }
    }
    state.project_root = project_root;
    state.phase = ServerPhase::Initialized;
    state.protocol_version =
        negotiate_protocol_version(params.get("protocolVersion").and_then(Value::as_str));

    let capabilities = McpCapabilities {
        protocol_version: state.protocol_version.into(),
        capabilities: McpCapabilityFlags {
            tools: McpToolCapability {
                list_changed: false,
            },
            resources: McpResourceCapability {
                subscribe: true,
                list_changed: false,
            },
            prompts: McpPromptCapability {
                list_changed: false,
            },
        },
        server_info: McpServerInfo {
            name: "specforge-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tools: state.tool_registry.clone(),
        resources: state.resource_registry.clone(),
        prompts: state.prompt_registry.clone(),
    };

    // Extension surfaces are what registration added past the defaults.
    let default_tools = crate::registry::default_tool_count();
    let default_resources = crate::registry::default_resource_count();
    let auto_promoted_tools = state
        .surface_entries
        .iter()
        .filter(|e| e.surface_type == specforge_registry::SurfaceType::AutoPromotedTool)
        .count();
    state.push_event(
        "mcp_initialized",
        serde_json::json!({
            "tools_registered": state.tool_registry.len(),
            "resources_registered": state.resource_registry.len(),
            "prompts_registered": state.prompt_registry.len(),
            "extensions_loaded": state.extension_info.len(),
            "surface_tools_registered": state.tool_registry.len().saturating_sub(default_tools),
            "surface_resources_registered": state
                .resource_registry
                .len()
                .saturating_sub(default_resources),
            "auto_promoted_tools": auto_promoted_tools,
        }),
    );

    let value = serde_json::to_value(capabilities).unwrap();
    JsonRpcResponse::success(id, value)
}

pub fn handle_shutdown(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if state.phase == ServerPhase::ShuttingDown {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_REQUEST,
            "Server already shutting down",
        );
    }

    let pending_notifications = state.notification_outbox.len();
    let subscriptions: usize = state.subscriptions.values().map(Vec::len).sum();
    state.shutdown();
    // Compiles build and drop their own Wasm runtime, so no engine outlives
    // a request.
    state.push_event(
        "mcp_server_shutdown",
        serde_json::json!({
            "pending_notifications_flushed": pending_notifications,
            "subscriptions_released": subscriptions,
            "wasm_engines_released": 0,
        }),
    );
    JsonRpcResponse::success(id, serde_json::json!({}))
}

pub fn handle_cancel(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    // JSON-RPC ids are strings or numbers; the event names either as a string.
    let request_id = match params.get("requestId").or_else(|| params.get("id")) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    // Requests run one at a time, so the one named has already completed:
    // it was never in progress, and cancelling it changes nothing.
    state.push_event(
        "mcp_request_cancelled",
        serde_json::json!({
            "requestId": request_id,
            "wasInProgress": false,
        }),
    );
    JsonRpcResponse::success(id, serde_json::json!({}))
}
