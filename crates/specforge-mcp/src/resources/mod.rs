mod brief;
mod context;
mod diagnostics;
mod entities_by_kind;
mod entity;
mod graph;
mod schema;

use serde_json::Value;

use crate::protocol::{JsonRpcError, JsonRpcResponse, error_codes};
use crate::state::McpState;

/// The one text content a resource read returns.
pub(crate) struct ResourceText {
    pub uri: String,
    pub mime_type: String,
    pub text: String,
}

impl ResourceText {
    pub fn json(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: "application/json".into(),
            text: text.into(),
        }
    }

    /// The `resources/read` result: the only place that builds `contents`.
    fn into_result(self) -> Value {
        serde_json::json!({
            "contents": [{
                "uri": self.uri,
                "mimeType": self.mime_type,
                "text": self.text,
            }]
        })
    }
}

/// What a resource read produced, or why it was refused.
pub(crate) type ReadOutcome = Result<ResourceText, JsonRpcError>;

pub(crate) fn invalid_params(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError::new(error_codes::INVALID_PARAMS, message)
}

pub fn handle_resource_read(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }

    let uri = match params.get("uri").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: uri",
            );
        }
    };

    match read(state, &uri) {
        Ok(content) => {
            // A read that returned content: its format is its MIME type.
            state.push_event(
                "mcp_resource_read",
                serde_json::json!({"resourceUri": uri, "format": content.mime_type}),
            );
            JsonRpcResponse::success(id, content.into_result())
        }
        Err(error) => JsonRpcResponse::from_error(id, error),
    }
}

fn read(state: &mut McpState, uri: &str) -> ReadOutcome {
    let uri = uri.to_string();
    // Query strings (?root=&depth=&kinds=&max_tokens=) ride on the
    // resource URIs (C9-06); specforge://context/{entity_id} scopes via its
    // path segment.
    match uri.as_str() {
        u if u == "specforge://graph" || u.starts_with("specforge://graph?") => {
            graph::read(state, u)
        }
        "specforge://schema" => schema::read(state),
        u if u == "specforge://context"
            || u.starts_with("specforge://context?")
            || u.starts_with("specforge://context/") =>
        {
            context::read(state, u)
        }
        u if u == "specforge://brief" || u.starts_with("specforge://brief?") => {
            brief::read(state, u)
        }
        "specforge://diagnostics" => diagnostics::read(state),
        _ if uri.starts_with("specforge://graph/") => {
            let entity_id = &uri["specforge://graph/".len()..];
            entity::read(state, entity_id)
        }
        _ if uri.starts_with("specforge://entities/") => {
            let kind = &uri["specforge://entities/".len()..];
            entities_by_kind::read(state, kind)
        }
        _ if uri.starts_with("specforge://ext/") => {
            // Extension-contributed resource: dispatch through the Wasm
            // runtime (WASM-only migration, Phase 4).
            let Some(entry) = state.surface_entries.iter().find(|e| {
                e.surface_type == specforge_registry::SurfaceType::McpResource
                    && e.enabled
                    && matches_uri_template(&e.contribution_name, &uri)
            }) else {
                return Err(invalid_params(format!("Unknown resource URI: {}", uri)));
            };
            let Some(root) = state.project_root.clone() else {
                return Err(invalid_params(
                    "Extension resources need a project root; pass {\"path\": ...} to specforge.analyze first",
                ));
            };
            let runtime = specforge_component::project_runtime(&root);
            match specforge_wasm::dispatch_surface_mcp_resource(
                &entry.extension_name,
                &entry.export_name,
                &uri,
                &runtime,
            ) {
                Ok((content, mime)) => Ok(ResourceText {
                    text: String::from_utf8_lossy(&content).into_owned(),
                    uri,
                    mime_type: mime,
                }),
                Err(diag) => Err(invalid_params(format!("{}: {}", diag.code, diag.message))),
            }
        }
        _ => Err(invalid_params(format!("Unknown resource URI: {}", uri))),
    }
}

/// Match a resource URI against an extension's contribution name, which
/// registration stores as the URI template (e.g. `specforge://ext/widgets/{id}`).
fn matches_uri_template(template: &str, uri: &str) -> bool {
    let (tpl_head, _) = template.split_once('{').unwrap_or((template, ""));
    uri.starts_with(tpl_head)
}

use crate::DEFAULT_CLIENT_ID as DEFAULT_SUBSCRIBER;

/// Map a subscribed resource URI to the delta-notification channel whose
/// changes it observes (C9-01).
fn notification_channel(uri: &str) -> &'static str {
    if uri == "specforge://diagnostics" {
        crate::notifications::DIAGNOSTICS_CHANNEL
    } else {
        crate::notifications::GRAPH_CHANNEL
    }
}

/// MCP `resources/subscribe`: track the client's interest in a resource so
/// recompiles deliver delta notifications (C9-01).
pub fn handle_resource_subscribe(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }
    let Some(uri) = params.get("uri").and_then(|v| v.as_str()) else {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            "Missing required parameter: uri",
        );
    };
    let client = params
        .get("client_id")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_SUBSCRIBER);
    crate::subscriptions::subscribe(state, client, notification_channel(uri));
    JsonRpcResponse::success(id, serde_json::json!({}))
}

/// MCP `resources/unsubscribe`: drop the client's interest in a resource.
pub fn handle_resource_unsubscribe(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }
    let Some(uri) = params.get("uri").and_then(|v| v.as_str()) else {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            "Missing required parameter: uri",
        );
    };
    let client = params
        .get("client_id")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_SUBSCRIBER);
    crate::subscriptions::unsubscribe(state, client, notification_channel(uri));
    JsonRpcResponse::success(id, serde_json::json!({}))
}

/// Split a resource URI into its path and query components.
pub(crate) fn split_query(uri: &str) -> (&str, &str) {
    uri.split_once('?').unwrap_or((uri, ""))
}

/// Query parameters shared by the graph/context/brief resources (C9-06).
#[derive(Default)]
pub(crate) struct ResourceQuery<'a> {
    /// Scope emission to the subgraph rooted at this entity id.
    pub root: Option<&'a str>,
    /// Maximum traversal depth from the scoped root.
    pub depth: Option<usize>,
    /// Only include nodes of these kinds.
    pub kinds: Vec<&'a str>,
    /// Token budget for the emitted payload.
    pub max_tokens: Option<usize>,
}

/// Parse `root`/`depth`/`kinds`/`max_tokens` out of a resource query string.
/// Unknown keys and malformed values are ignored so partial queries still
/// serve.
pub(crate) fn parse_query(query: &str) -> ResourceQuery<'_> {
    let mut parsed = ResourceQuery::default();
    for kv in query.split('&') {
        let Some((key, value)) = kv.split_once('=') else {
            continue;
        };
        match key {
            "root" => parsed.root = Some(value),
            "depth" => parsed.depth = value.parse().ok(),
            "kinds" => {
                parsed.kinds = value
                    .split(',')
                    .map(str::trim)
                    .filter(|k| !k.is_empty())
                    .collect();
            }
            "max_tokens" => parsed.max_tokens = value.parse().ok(),
            _ => {}
        }
    }
    parsed
}
