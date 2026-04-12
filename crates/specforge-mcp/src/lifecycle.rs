use serde_json::Value;
use std::path::PathBuf;

use crate::compile::compile_project;
use crate::protocol::{JsonRpcResponse, error_codes};
use crate::registry::{register_defaults, register_extension_surfaces};
use crate::state::{McpState, ServerPhase};
use crate::types::{McpCapabilities, McpCapabilityFlags, McpToolCapability, McpResourceCapability, McpPromptCapability, McpServerInfo};

pub fn handle_initialize(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    if state.phase == ServerPhase::Initialized {
        state.push_event("mcp_initialization_failed", serde_json::json!({"reason": "already_initialized"}));
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server already initialized");
    }

    if state.phase == ServerPhase::ShuttingDown {
        state.push_event("mcp_initialization_failed", serde_json::json!({"reason": "shutting_down"}));
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server is shutting down");
    }

    let project_root = params.get("projectRoot")
        .or_else(|| params.get("project_root"))
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    // Register tool/resource/prompt descriptors
    register_defaults(state);

    // Compile if project root is provided
    if let Some(root) = &project_root
        && root.exists() {
            let result = compile_project(root);
            state.graph = result.graph;
            state.diagnostics = result.diagnostics;
            state.kind_registry = result.kind_registry;
            state.field_registry = result.field_registry;
            state.edge_registry = result.edge_registry;
            state.extension_info = result.extension_info;
            state.surface_entries = result.surface_entries;

            // Register extension MCP tools and resources from manifest surfaces
            register_extension_surfaces(state, &result.manifest_surfaces);
        }

    state.project_root = project_root;
    state.phase = ServerPhase::Initialized;

    let capabilities = McpCapabilities {
        protocol_version: "2025-03-26".into(),
        capabilities: McpCapabilityFlags {
            tools: McpToolCapability { list_changed: false },
            resources: McpResourceCapability { subscribe: false, list_changed: false },
            prompts: McpPromptCapability { list_changed: false },
        },
        server_info: McpServerInfo {
            name: "specforge-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tools: state.tool_registry.clone(),
        resources: state.resource_registry.clone(),
        prompts: state.prompt_registry.clone(),
    };

    state.push_event("mcp_initialized", serde_json::json!({
        "server_name": "specforge-mcp",
        "tools_count": state.tool_registry.len(),
        "resources_count": state.resource_registry.len(),
        "prompts_count": state.prompt_registry.len(),
    }));

    let value = serde_json::to_value(capabilities).unwrap();
    JsonRpcResponse::success(id, value)
}

pub fn handle_shutdown(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if state.phase == ServerPhase::ShuttingDown {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server already shutting down");
    }

    state.push_event("mcp_server_shutdown", serde_json::json!({}));
    state.shutdown();
    JsonRpcResponse::success(id, serde_json::json!({}))
}

pub fn handle_cancel(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    let cancelled_id = params.get("id").cloned().unwrap_or(serde_json::Value::Null);
    state.push_event("mcp_request_cancelled", serde_json::json!({"cancelled_id": cancelled_id}));
    // Cancellation is best-effort; we acknowledge but can't cancel synchronous operations
    JsonRpcResponse::success(id, serde_json::json!({}))
}
