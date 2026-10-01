//! The stateless revision (MCP 2026-07-28), served beside the handshake
//! revisions (ADR 0004 D4-c).
//!
//! A request whose `_meta` names a protocol version is served on its own:
//! no `initialize` before it, and nothing a prior request said changes how
//! it is answered. Its result carries `resultType`, the server's identity
//! in `_meta`, and on a list or a read the caching hints. A request without
//! that `_meta` keeps the handshake rules of the revision `initialize`
//! negotiated.

use serde_json::{Value, json};

use crate::lifecycle::{MODERN_PROTOCOL_VERSIONS, server_info};
use crate::protocol::{JsonRpcError, JsonRpcResponse, error_codes};
use crate::state::{Listen, McpState, ServerPhase};

/// The `_meta` key naming the revision a request is made under.
pub const PROTOCOL_VERSION_META: &str = "io.modelcontextprotocol/protocolVersion";
/// The `_meta` key carrying the client's capabilities, required with it.
pub const CLIENT_CAPABILITIES_META: &str = "io.modelcontextprotocol/clientCapabilities";
/// The `_meta` key a result names the server in.
pub const SERVER_INFO_META: &str = "io.modelcontextprotocol/serverInfo";
/// The `_meta` key tying a notification to its `subscriptions/listen`.
pub const SUBSCRIPTION_ID_META: &str = "io.modelcontextprotocol/subscriptionId";

/// The revision's error for a version the server does not speak.
pub const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// The results the revision makes cacheable. The server's are about the
/// project on disk, which any request may recompile: nothing stays fresh
/// (`ttlMs` 0), and nothing is shared beyond this client (`private`).
const CACHEABLE: [&str; 6] = [
    "server/discover",
    "tools/list",
    "prompts/list",
    "resources/list",
    "resources/templates/list",
    "resources/read",
];

/// Whether `method` with `params` is a stateless request: `server/discover`,
/// which exists only in that revision, or any request whose `_meta` names a
/// protocol version.
pub fn is_modern(method: &str, params: &Value) -> bool {
    method == "server/discover" || params["_meta"].get(PROTOCOL_VERSION_META).is_some()
}

/// Serve one stateless request or notification. A `subscriptions/listen`
/// gets no response now: its acknowledgement is queued, and the stream
/// stays open until the client cancels it.
pub fn handle(
    state: &mut McpState,
    method: &str,
    params: Value,
    id: Option<Value>,
) -> Option<JsonRpcResponse> {
    if id.is_none() {
        // Cancellation is the revision's one client notification.
        if method == "notifications/cancelled" {
            crate::lifecycle::handle_cancel(state, params, None);
        }
        return None;
    }
    let version = match requested_version(&params) {
        Ok(version) => version,
        Err(error) => return Some(JsonRpcResponse::from_error(id, error)),
    };
    if state.phase == ServerPhase::ShuttingDown {
        return Some(JsonRpcResponse::error(
            id,
            error_codes::INVALID_REQUEST,
            "Server shutting down",
        ));
    }
    if !state.served {
        let root = state.default_project_root.clone();
        crate::lifecycle::serve_project(state, root);
    }

    state.request_revision = Some(version);
    let response = match method {
        "server/discover" => Some(crate::lifecycle::handle_discover(id)),
        "subscriptions/listen" => listen(state, &params, id),
        "tools/list"
        | "tools/call"
        | "resources/list"
        | "resources/templates/list"
        | "resources/read"
        | "prompts/list"
        | "prompts/get" => Some(crate::protocol::router::route(state, method, params, id)),
        // ping, logging/setLevel, resources/subscribe and the rest are not
        // methods of this revision; completion/complete is not offered.
        _ => Some(JsonRpcResponse::error(
            id,
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {method}"),
        )),
    };
    state.request_revision = None;
    response.map(|response| decorate(response, method))
}

/// The revision a request names, checked: a request without the version or
/// the client capabilities is malformed (-32602); a version the server does
/// not speak is -32022 naming those it does.
fn requested_version(params: &Value) -> Result<&'static str, JsonRpcError> {
    let meta = &params["_meta"];
    let (Some(requested), true) = (
        meta.get(PROTOCOL_VERSION_META).and_then(Value::as_str),
        meta.get(CLIENT_CAPABILITIES_META)
            .is_some_and(Value::is_object),
    ) else {
        return Err(JsonRpcError::new(
            error_codes::INVALID_PARAMS,
            format!(
                "Invalid params: a request without initialize needs _meta[\"{PROTOCOL_VERSION_META}\"] and _meta[\"{CLIENT_CAPABILITIES_META}\"]"
            ),
        ));
    };
    MODERN_PROTOCOL_VERSIONS
        .into_iter()
        .find(|v| *v == requested)
        .ok_or_else(|| {
            JsonRpcError::new(UNSUPPORTED_PROTOCOL_VERSION, "Unsupported protocol version")
                .with_data(json!({ "supported": MODERN_PROTOCOL_VERSIONS, "requested": requested }))
        })
}

/// A successful result as the revision gives it: `resultType`, the server
/// in `_meta`, and the caching hints on a list or a read.
fn decorate(mut response: JsonRpcResponse, method: &str) -> JsonRpcResponse {
    let Some(Value::Object(result)) = response.result.as_mut() else {
        return response;
    };
    result
        .entry("resultType")
        .or_insert_with(|| Value::from("complete"));
    let meta = result.entry("_meta").or_insert_with(|| json!({}));
    if let Some(meta) = meta.as_object_mut() {
        meta.insert(
            SERVER_INFO_META.into(),
            serde_json::to_value(server_info()).unwrap_or_default(),
        );
    }
    if CACHEABLE.contains(&method) {
        result.insert("ttlMs".into(), Value::from(0));
        result.insert("cacheScope".into(), Value::from("private"));
    }
    response
}

/// `subscriptions/listen`: open a stream on which the server tells the
/// client when the resources it names change. The server honours resource
/// subscriptions to any resource it serves, and no list-changed types (it
/// offers none); the acknowledgement, queued first, names the subset
/// honoured. No response follows until the stream ends.
fn listen(state: &mut McpState, params: &Value, id: Option<Value>) -> Option<JsonRpcResponse> {
    let Some(filter) = params.get("notifications").filter(|f| f.is_object()) else {
        return Some(JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            "Invalid params: subscriptions/listen needs notifications",
        ));
    };
    let id = id.expect("a request has an id");
    let uris: Vec<String> = filter["resourceSubscriptions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|uri| crate::resources::is_served(state, uri))
        .map(str::to_string)
        .collect();
    state.notification_outbox.push(json!({
        "jsonrpc": "2.0",
        "method": "notifications/subscriptions/acknowledged",
        "params": {
            "_meta": { SUBSCRIPTION_ID_META: id },
            "notifications": { "resourceSubscriptions": uris },
        },
    }));
    for uri in &uris {
        state.push_event(
            "mcp_subscription_created",
            json!({"subscriptionType": uri, "clientId": id.to_string()}),
        );
    }
    state.listens.retain(|open| open.id != id);
    state.listens.push(Listen { id, uris });
    None
}

/// End the `subscriptions/listen` stream whose request id is `request_id`,
/// when one is open. The client cancelled it, so nothing more is sent on
/// it, not even a response.
pub fn end_listen(state: &mut McpState, request_id: &Value) -> bool {
    let Some(position) = state.listens.iter().position(|l| &l.id == request_id) else {
        return false;
    };
    let ended = state.listens.remove(position);
    for uri in &ended.uris {
        state.push_event(
            "mcp_subscription_removed",
            json!({"subscriptionType": uri, "clientId": ended.id.to_string()}),
        );
    }
    true
}

/// Queue `notifications/resources/updated` on every open stream for each
/// resource it listens to whose content a recompile changed: the graph's
/// views when `graph_changed`, the diagnostics when `diagnostics_changed`.
pub fn enqueue_resource_updates(
    state: &mut McpState,
    graph_changed: bool,
    diagnostics_changed: bool,
) {
    let mut updates = Vec::new();
    for listen in &state.listens {
        for uri in &listen.uris {
            let changed = if uri == "specforge://diagnostics" {
                diagnostics_changed
            } else {
                graph_changed
            };
            if changed {
                updates.push(json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/resources/updated",
                    "params": {
                        "_meta": { SUBSCRIPTION_ID_META: listen.id },
                        "uri": uri,
                    },
                }));
            }
        }
    }
    state.notification_outbox.extend(updates);
}
