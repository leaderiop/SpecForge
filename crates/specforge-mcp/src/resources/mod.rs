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
use crate::types::McpResourceDescriptor;

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

/// One core resource: everything the server lists and reads about it.
pub struct ResourceSpec {
    /// Its URI, or an RFC 6570 template (`specforge://graph/{entity_id}`).
    pub uri: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
    /// Read the resource at a URI it [matches](Self::matches).
    pub(crate) read: fn(&McpState, &str) -> ReadOutcome,
}

impl ResourceSpec {
    /// The resource as `resources/list` (or `resources/templates/list`)
    /// describes it.
    pub fn descriptor(&self) -> McpResourceDescriptor {
        McpResourceDescriptor {
            uri: self.uri.into(),
            name: self.name.into(),
            description: Some(self.description.into()),
            mime_type: Some(self.mime_type.into()),
        }
    }

    /// Whether `uri` names this resource: a template's prefix and what
    /// stands for its placeholder (the reader refuses an empty one), or the URI itself with an optional query string
    /// (C9-06: `?root=&depth=&kinds=&max_tokens=`).
    pub fn matches(&self, uri: &str) -> bool {
        match self.uri.split_once('{') {
            Some((prefix, _)) => uri.starts_with(prefix),
            None => {
                uri == self.uri
                    || uri
                        .strip_prefix(self.uri)
                        .is_some_and(|rest| rest.starts_with('?'))
            }
        }
    }
}

/// The value of the placeholder ending a templated resource URI: what
/// follows `prefix`.
fn after<'a>(uri: &'a str, prefix: &str) -> &'a str {
    uri.strip_prefix(prefix).unwrap_or_default()
}

/// The core resources, in listing order.
pub static CORE_RESOURCES: &[ResourceSpec] = &[
    ResourceSpec {
        uri: "specforge://graph",
        name: "graph",
        description: "Full spec graph in JSON format",
        mime_type: "application/json",
        read: graph::read,
    },
    ResourceSpec {
        uri: "specforge://schema",
        name: "schema",
        description: "Graph schema definition",
        mime_type: "application/json",
        read: |state, _| schema::read(state),
    },
    ResourceSpec {
        uri: "specforge://context",
        name: "context",
        description: "Context-optimized graph (contract, status, verify fields)",
        mime_type: "application/json",
        read: context::read,
    },
    ResourceSpec {
        uri: "specforge://context/{entity_id}",
        name: "context_entity",
        description: "Context-optimized subgraph rooted at an entity",
        mime_type: "application/json",
        read: context::read,
    },
    ResourceSpec {
        uri: "specforge://brief",
        name: "brief",
        description: "Brief graph (id, kind, title, edges only)",
        mime_type: "application/json",
        read: brief::read,
    },
    ResourceSpec {
        uri: "specforge://diagnostics",
        name: "diagnostics",
        description: "Current compilation diagnostics",
        mime_type: "application/json",
        read: |state, _| diagnostics::read(state),
    },
    ResourceSpec {
        uri: "specforge://graph/{entity_id}",
        name: "entity",
        description: "Subgraph rooted at a specific entity",
        mime_type: "application/json",
        read: |state, uri| entity::read(state, after(uri, "specforge://graph/")),
    },
    ResourceSpec {
        uri: "specforge://entities/{kind}",
        name: "entities_by_kind",
        description: "All entities of a specific kind (e.g. feature, behavior)",
        mime_type: "application/json",
        read: |state, uri| entities_by_kind::read(state, after(uri, "specforge://entities/")),
    },
];

fn read(state: &McpState, uri: &str) -> ReadOutcome {
    if let Some(resource) = CORE_RESOURCES.iter().find(|r| r.matches(uri)) {
        return (resource.read)(state, uri);
    }
    if uri.starts_with("specforge://ext/") {
        return extension_resource(state, uri);
    }
    Err(invalid_params(format!("Unknown resource URI: {uri}")))
}

/// Whether `uri` names a resource the server serves: a core one, or one an
/// extension contributes.
pub(crate) fn is_served(state: &McpState, uri: &str) -> bool {
    CORE_RESOURCES.iter().any(|r| r.matches(uri))
        || (uri.starts_with("specforge://ext/") && extension_resource_entry(state, uri).is_some())
}

/// The extension resource whose URI template `uri` matches.
fn extension_resource_entry<'a>(
    state: &'a McpState,
    uri: &str,
) -> Option<&'a specforge_registry::SurfaceRegistryEntry> {
    state.surface_entries().find(|e| {
        e.surface_type == specforge_registry::SurfaceType::McpResource
            && uri_template(state, e).is_some_and(|template| matches_uri_template(template, uri))
    })
}

/// The URI template the extension resource `entry` registers (entries name
/// a resource by its `name`).
fn uri_template<'a>(
    state: &'a McpState,
    entry: &specforge_registry::SurfaceRegistryEntry,
) -> Option<&'a str> {
    state
        .registries()
        .manifest_surfaces
        .iter()
        .filter(|(extension, _)| *extension == entry.extension_name)
        .flat_map(|(_, surfaces)| &surfaces.mcp_resources)
        .find(|resource| resource.name == entry.contribution_name)
        .map(|resource| resource.uri_template.as_str())
}

/// An extension-contributed resource, read through the Wasm runtime
/// (WASM-only migration, Phase 4).
fn extension_resource(state: &McpState, uri: &str) -> ReadOutcome {
    let Some(entry) = extension_resource_entry(state, uri) else {
        return Err(invalid_params(format!("Unknown resource URI: {uri}")));
    };
    let Some(root) = state.project_root.clone() else {
        return Err(invalid_params(
            "Extension resources need a project root; pass {\"path\": ...} to specforge.analyze first",
        ));
    };
    let runtime = state.wasm_runtime(&root);
    match specforge_wasm::dispatch_surface_mcp_resource(
        &entry.extension_name,
        &entry.export_name,
        uri,
        runtime.as_ref(),
    ) {
        Ok((content, mime)) => Ok(ResourceText {
            text: String::from_utf8_lossy(&content).into_owned(),
            uri: uri.to_string(),
            mime_type: mime,
        }),
        Err(diag) => Err(invalid_params(format!("{}: {}", diag.code, diag.message))),
    }
}

/// Whether `uri` is one `template` names: the template itself when it has
/// no `{placeholder}`, else its text before the first placeholder followed
/// by more (`specforge://ext/widgets/{id}` names `specforge://ext/widgets/w1`).
fn matches_uri_template(template: &str, uri: &str) -> bool {
    match template.split_once('{') {
        Some((head, _)) => uri.len() > head.len() && uri.starts_with(head),
        None => uri == template,
    }
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
