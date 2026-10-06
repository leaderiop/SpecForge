mod brief;
mod context;
mod diagnostics;
mod entities_by_kind;
mod entity;
mod graph;
mod schema;

use serde_json::{Value, json};

use crate::protocol::{JsonRpcError, JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::surface_call::{Event, Found, Invocation, Ran, Surface};
use crate::surface_table::ResourceEntry;
use crate::target::{self, Call, CallTarget, TargetSpec};
use crate::types::McpResourceDescriptor;

/// `resources/read`: the core resources (matched first), then the extension
/// resource whose template names the URI (ADR 0017 D9). The request
/// pipeline's resources adapter ([`crate::surface_call`]).
pub(crate) struct Resources;

impl Surface for Resources {
    const NAMED_BY: &'static str = "uri";
    const TAKES_ARGUMENTS: bool = false;
    const EXTENDED: bool = true;
    const UNKNOWN: &'static str = "Unknown resource URI";

    type Core = &'static ResourceSpec;
    type Extension = ResourceEntry;
    type Outcome = ReadOutcome;

    fn core(uri: &str) -> Option<&'static ResourceSpec> {
        CORE_RESOURCES.iter().find(|r| r.matches(uri))
    }

    fn extension(state: &McpState, uri: &str) -> Option<ResourceEntry> {
        state.surfaces().resource(uri).cloned()
    }

    fn target(found: &Found<&'static ResourceSpec, ResourceEntry>) -> TargetSpec {
        match found {
            Found::Core(spec) => spec.target,
            Found::Extension(_) => TargetSpec::SERVED,
        }
    }

    fn invoked(_: &Found<&'static ResourceSpec, ResourceEntry>, _: &Invocation) -> Option<Event> {
        None
    }

    fn run(
        call: &mut Call<'_>,
        found: &Found<&'static ResourceSpec, ResourceEntry>,
        invocation: &Invocation,
    ) -> Ran<ReadOutcome> {
        match found {
            Found::Core(spec) => Ran::of((spec.read)(call, &invocation.name)),
            Found::Extension(entry) => extension_resource(call, entry, &invocation.name),
        }
    }

    fn refused(
        _: &Found<&'static ResourceSpec, ResourceEntry>,
        error: crate::tool::McpError,
    ) -> Ran<ReadOutcome> {
        Ran::of(Err(invalid_params(error.message)))
    }

    fn without_project(target: &CallTarget, outcome: ReadOutcome) -> ReadOutcome {
        outcome.map_err(|refused| without_project(target, refused))
    }

    fn completed(
        _: &Found<&'static ResourceSpec, ResourceEntry>,
        invocation: &Invocation,
        outcome: &ReadOutcome,
    ) -> Option<Event> {
        // A read that returned content: its format is its MIME type.
        let text = outcome.as_ref().ok()?;
        Some((
            "mcp_resource_read".to_string(),
            json!({"resourceUri": invocation.name, "format": text.mime_type}),
        ))
    }

    fn envelope(
        _: &McpState,
        found: &Found<&'static ResourceSpec, ResourceEntry>,
        invocation: &Invocation,
        outcome: ReadOutcome,
        id: Option<Value>,
    ) -> JsonRpcResponse {
        // A core resource answers under its URI without the query it was
        // read with; an extension's, under the URI read.
        let uri = match found {
            Found::Core(_) => split_query(&invocation.name).0,
            Found::Extension(_) => invocation.name.as_str(),
        };
        resource_envelope(outcome, uri, id)
    }
}

/// What a resource read returned: its MIME type and text. The URI is the one
/// the envelope writes.
pub(crate) struct ResourceText {
    pub mime_type: String,
    pub text: String,
}

impl ResourceText {
    pub fn json(text: impl Into<String>) -> Self {
        Self {
            mime_type: "application/json".into(),
            text: text.into(),
        }
    }
}

/// What a resource read produced, or why it was refused.
pub(crate) type ReadOutcome = Result<ResourceText, JsonRpcError>;

/// The `resources/read` reply: the only place that builds `contents`. Its
/// `uri` is the one given.
pub(crate) fn resource_envelope(
    outcome: ReadOutcome,
    uri: &str,
    id: Option<Value>,
) -> JsonRpcResponse {
    match outcome {
        Ok(text) => JsonRpcResponse::success(
            id,
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": text.mime_type,
                    "text": text.text,
                }]
            }),
        ),
        Err(error) => JsonRpcResponse::from_error(id, error),
    }
}

pub(crate) fn invalid_params(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError::new(error_codes::INVALID_PARAMS, message)
}

/// A read that names an entity the graph does not have: invalid params, as
/// ever, its `McpError` (`entity_not_found`) as the error's `data`, which is
/// what [`without_project`] reads.
pub(crate) fn entity_not_found(message: impl Into<String>, entity_id: &str) -> JsonRpcError {
    let error = crate::tool::entity_not_found(entity_id);
    JsonRpcError::new(error.code.rpc_code(), message).with_data(error.to_json())
}

/// What a read refused with, as the call's target makes it
/// ([`target::without_project`]): with nothing served, an entity that is
/// not found is the no-project refusal, an internal error (-32603) whose
/// `data` is its `McpError`, as the refusal of a resource that needs a
/// project is. Any other refusal is returned as it is.
fn without_project(target: &CallTarget, error: JsonRpcError) -> JsonRpcError {
    let named = error
        .data
        .as_ref()
        .filter(|data| data.get("code").and_then(Value::as_str) == Some("entity_not_found"))
        .and_then(|data| data.get("entity_id")?.as_str())
        .map(str::to_string);
    let Some(entity_id) = named else {
        return error;
    };
    let refused = target::without_project(target, crate::tool::entity_not_found(&entity_id));
    if refused.code != crate::tool::ErrorCode::PreconditionFailed {
        return error;
    }
    JsonRpcError::new(refused.code.rpc_code(), refused.message.clone()).with_data(refused.to_json())
}

/// One core resource: everything the server lists and reads about it.
pub struct ResourceSpec {
    /// Its URI, or an RFC 6570 template (`specforge://graph/{entity_id}`).
    pub uri: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
    /// Which project it reads: the served one, brought up to date first.
    pub target: TargetSpec,
    /// Read the resource at a URI it [matches](Self::matches).
    pub(crate) read: fn(&Call<'_>, &str) -> ReadOutcome,
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
        target: TargetSpec::SERVED,
        read: |call, uri| graph::read(&call.view(), uri),
    },
    ResourceSpec {
        uri: "specforge://schema",
        name: "schema",
        description: "Graph schema definition",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, _| schema::read(&call.view()),
    },
    ResourceSpec {
        uri: "specforge://context",
        name: "context",
        description: "Context-optimized graph (contract, status, verify fields)",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| context::read(call.state, uri),
    },
    ResourceSpec {
        uri: "specforge://context/{entity_id}",
        name: "context_entity",
        description: "Context-optimized subgraph rooted at an entity",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| context::read(call.state, uri),
    },
    ResourceSpec {
        uri: "specforge://brief",
        name: "brief",
        description: "Brief graph (id, kind, title, edges only)",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| brief::read(call.state, uri),
    },
    ResourceSpec {
        uri: "specforge://diagnostics",
        name: "diagnostics",
        description: "Current compilation diagnostics",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, _| diagnostics::read(call.state),
    },
    ResourceSpec {
        uri: "specforge://graph/{entity_id}",
        name: "entity",
        description: "Subgraph rooted at a specific entity",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| entity::read(call.state, after(uri, "specforge://graph/")),
    },
    ResourceSpec {
        uri: "specforge://entities/{kind}",
        name: "entities_by_kind",
        description: "All entities of a specific kind (e.g. feature, behavior)",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| entities_by_kind::read(call.state, after(uri, "specforge://entities/")),
    },
];

/// Whether `uri` names a resource the server serves: a core one, or one an
/// extension contributes.
pub(crate) fn is_served(state: &McpState, uri: &str) -> bool {
    CORE_RESOURCES.iter().any(|r| r.matches(uri)) || state.surfaces().resource(uri).is_some()
}

/// The resource adapter: an extension resource read through its `mcp__`
/// export, over the `WasmRuntime` seam the call's project was compiled in.
/// A failed read (a trap, an answer that is not its content and MIME type,
/// an export the guest does not route) is a server-side fault of the
/// extension: an internal error (-32603) whose `data` is the `McpError`
/// carrying the E028 diagnostic (D6, `mcp_structured_error_responses`).
fn extension_resource(call: &Call<'_>, entry: &ResourceEntry, uri: &str) -> Ran<ReadOutcome> {
    // The project the resource reads, in the runtime it was compiled in.
    let project = match call.project() {
        Ok(project) => project,
        Err(refused) => {
            return Ran::of(Err(
                invalid_params(refused.message.clone()).with_data(refused.to_json())
            ));
        }
    };
    let runtime = project.runtime;
    let started = std::time::Instant::now();
    match specforge_wasm::ExtensionCalls::new(runtime.as_ref()).read_mcp_resource(
        &entry.extension,
        &entry.export,
        uri,
    ) {
        // A read whose export answered its content is a dispatched
        // resource; a failed call (a trap, or an answer that is not the
        // content and its MIME type) is the read's error, and no dispatch is
        // recorded.
        Ok(read) => Ran {
            events: vec![(
                "surface_mcp_resource_dispatched".to_string(),
                json!({
                    "extensionName": entry.extension,
                    "uriTemplate": entry.uri_template,
                    "mimeType": read.mime_type,
                    "durationMs": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                }),
            )],
            outcome: Ok(ResourceText {
                text: read.content,
                mime_type: read.mime_type,
            }),
        },
        Err(error) => {
            let diagnostic = error.diagnostic();
            Ran::of(Err(JsonRpcError::new(
                error_codes::INTERNAL_ERROR,
                diagnostic.message.clone(),
            )
            .with_data(
                crate::tool::McpError::from_diagnostic(&diagnostic).to_json(),
            )))
        }
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
/// updates of the served project deliver delta notifications (C9-01).
pub fn handle_resource_subscribe(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
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
