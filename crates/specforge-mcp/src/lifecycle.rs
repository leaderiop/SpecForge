use serde_json::Value;
use std::path::PathBuf;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::{McpState, ServerPhase};
use crate::types::{
    McpCapabilities, McpCapabilityFlags, McpPromptCapability, McpResourceCapability, McpServerInfo,
    McpToolCapability,
};

/// The stateless revisions the server speaks, latest first: a request
/// names one in its `_meta` and needs no `initialize` (MCP 2026-07-28).
pub const MODERN_PROTOCOL_VERSIONS: [&str; 1] = ["2026-07-28"];
/// The handshake revisions the server speaks, latest first.
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

    serve_project(state, project_root);
    state.phase = ServerPhase::Initialized;
    state.protocol_version =
        negotiate_protocol_version(params.get("protocolVersion").and_then(Value::as_str));

    let capabilities = McpCapabilities {
        protocol_version: state.protocol_version.into(),
        capabilities: capability_flags(),
        server_info: server_info(),
        tools: crate::registry::listed_tools(state).collect(),
        resources: crate::registry::listed_resources(state).collect(),
        prompts: crate::prompts::descriptors(),
    };

    // The extension surfaces are the table's entries.
    let surfaces = state.surfaces();
    let surface_tools = surfaces.tools().len();
    let surface_resources = surfaces.resources().len();
    let auto_promoted_tools = surfaces
        .tools()
        .iter()
        .filter(|tool| matches!(tool.kind, crate::surface_table::ToolKind::Command(_)))
        .count();
    state.push_event(
        "mcp_initialized",
        serde_json::json!({
            "tools_registered": crate::tools::CORE_TOOLS.len() + surface_tools,
            "resources_registered": crate::resources::CORE_RESOURCES.len() + surface_resources,
            "prompts_registered": crate::prompts::CORE_PROMPTS.len(),
            "extensions_loaded": state.registries().extension_info().count(),
            "surface_tools_registered": surface_tools,
            "surface_resources_registered": surface_resources,
            "auto_promoted_tools": auto_promoted_tools,
        }),
    );

    let value = serde_json::to_value(capabilities).unwrap();
    JsonRpcResponse::success(id, value)
}

/// Compile and serve the project at `project_root` when there is one: its
/// registries, and the core tools, resources and prompts plus what its
/// extensions contribute; the core surface alone otherwise. Subscribed
/// clients learn what it changed (C9-01). A root that does not exist has
/// no config to read: nothing is served (a call's `path` may serve a
/// project later).
pub fn serve_project(state: &mut McpState, project_root: Option<PathBuf>) {
    if let Some(root) = project_root.filter(|root| root.exists()) {
        state.serve(&root);
    }
    state.served = true;
}

/// The capabilities the server offers, in either era.
fn capability_flags() -> McpCapabilityFlags {
    McpCapabilityFlags {
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
    }
}

/// The server's name and version, as `serverInfo` gives them.
pub fn server_info() -> McpServerInfo {
    McpServerInfo {
        name: "specforge-mcp".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// `server/discover` (MCP 2026-07-28): the stateless revisions the server
/// speaks and its capabilities, answered with or without `initialize`. The
/// handshake revisions are not listed: a client reaches them through
/// `initialize`, never through per-request `_meta`.
pub fn handle_discover(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "supportedVersions": MODERN_PROTOCOL_VERSIONS,
            "capabilities": capability_flags(),
            "instructions": "SpecForge compiles .spec files into a graph of entities. Query it with specforge.query, specforge.search and specforge.inspect; check it with specforge.validate.",
        }),
    )
}

pub fn handle_shutdown(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if state.phase == ServerPhase::ShuttingDown {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_REQUEST,
            "Server already shutting down",
        );
    }

    let pending_notifications = state.subscriptions().pending();
    // The served project's runtime goes with its session: no engine
    // outlives shutdown.
    let engines = usize::from(state.session().runtime().is_some());
    let subscriptions = state.shutdown();
    state.push_event(
        "mcp_server_shutdown",
        serde_json::json!({
            "pending_notifications_flushed": pending_notifications,
            "subscriptions_released": subscriptions,
            "wasm_engines_released": engines,
        }),
    );
    JsonRpcResponse::success(id, serde_json::json!({}))
}

pub fn handle_cancel(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    // Cancelling a subscriptions/listen request ends its stream.
    if let Some(listened) = params.get("requestId") {
        state.subscriptions.end(listened, &mut state.events);
    }
    // JSON-RPC ids are strings or numbers; the event names either as a string.
    let request_id = params
        .get("requestId")
        .or_else(|| params.get("id"))
        .map(crate::protocol::id_text)
        .unwrap_or_default();
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
