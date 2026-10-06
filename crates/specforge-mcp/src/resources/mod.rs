mod schema;
mod views;

use serde_json::{Value, json};

use crate::protocol::{JsonRpcError, JsonRpcResponse};
use crate::state::McpState;
use crate::surface_call::{Event, Found, Invocation, Ran, Surface};
use crate::surface_table::ResourceEntry;
use crate::target::{Call, TargetSpec};
use crate::tool::{ErrorCode, McpError};
use crate::types::McpResourceDescriptor;
use specforge_ops::export::Format;

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
        error: McpError,
    ) -> Ran<ReadOutcome> {
        Ran::of(Err(Box::new(error)))
    }

    fn unknown(state: &McpState, uri: &str) -> JsonRpcError {
        unknown_resource(state.resource_not_found_code(), uri)
    }

    fn refusal_mut(outcome: &mut ReadOutcome) -> Option<&mut McpError> {
        outcome.as_mut().err().map(|refusal| &mut **refusal)
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
        state: &McpState,
        _: &Found<&'static ResourceSpec, ResourceEntry>,
        invocation: &Invocation,
        mut outcome: ReadOutcome,
        id: Option<Value>,
    ) -> JsonRpcResponse {
        // A refusal names the URI read.
        if let Err(refusal) = &mut outcome {
            refusal.uri.get_or_insert_with(|| invocation.name.clone());
        }
        // A read answers under the URI the client read: its own cache key.
        resource_envelope(
            outcome,
            &invocation.name,
            state.resource_not_found_code(),
            id,
        )
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

/// What a resource read produced, or why it was refused (ADR 0024 D4).
pub(crate) type ReadOutcome = Result<ResourceText, Box<McpError>>;

/// The `resources/read` reply: the only place that builds `contents` or a
/// resource's error. Its `uri` is the one given. A refusal is a JSON-RPC
/// error whose data is its McpError ([`McpError::into_rpc_error`]); one
/// that says the entity a read names does not exist is *not found*, and
/// carries `not_found`, the code of the revision the request speaks.
pub(crate) fn resource_envelope(
    outcome: ReadOutcome,
    uri: &str,
    not_found: i64,
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
        Err(refusal) => {
            let missing = refusal.code == ErrorCode::EntityNotFound;
            let mut error = refusal.into_rpc_error();
            if missing {
                error.code = not_found;
            }
            JsonRpcResponse::from_error(id, error)
        }
    }
}

/// A read of a URI no core resource and no extension serves: a lookup
/// failure, like "Unknown tool", so it carries no McpError, but it is *not
/// found* in the revision of the request (`not_found`) and names the URI.
pub(crate) fn unknown_resource(not_found: i64, uri: &str) -> JsonRpcError {
    JsonRpcError::new(not_found, format!("Unknown resource URI: {uri}"))
        .with_data(json!({ "uri": uri }))
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
    /// stands for its placeholder (the reader refuses an empty one), or the
    /// URI itself with an optional query string (which the reader reads, or
    /// refuses: [`views::ViewQuery`]).
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

/// The core resources, in listing order.
pub static CORE_RESOURCES: &[ResourceSpec] = &[
    ResourceSpec {
        uri: "specforge://graph",
        name: "graph",
        description: "Full spec graph in JSON format",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| views::export_view(call, uri, Format::Graph, None),
    },
    ResourceSpec {
        uri: "specforge://schema",
        name: "schema",
        description: "Graph schema definition",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| views::schema_view(call, uri),
    },
    ResourceSpec {
        uri: "specforge://context",
        name: "context",
        description: "Context-optimized graph (contract, status, verify fields)",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| views::export_view(call, uri, Format::Context, None),
    },
    ResourceSpec {
        uri: "specforge://context/{entity_id}",
        name: "context_entity",
        description: "Context-optimized subgraph rooted at an entity",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| {
            views::export_view(call, uri, Format::Context, Some("specforge://context/"))
        },
    },
    ResourceSpec {
        uri: "specforge://brief",
        name: "brief",
        description: "Brief graph (id, kind, title, edges only)",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| views::export_view(call, uri, Format::Brief, None),
    },
    ResourceSpec {
        uri: "specforge://diagnostics",
        name: "diagnostics",
        description: "Current compilation diagnostics",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| views::diagnostics_view(call, uri),
    },
    ResourceSpec {
        uri: "specforge://graph/{entity_id}",
        name: "entity",
        description: "Subgraph rooted at a specific entity",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| views::entity_view(call, uri),
    },
    ResourceSpec {
        uri: "specforge://entities/{kind}",
        name: "entities_by_kind",
        description: "All entities of a specific kind (e.g. feature, behavior)",
        mime_type: "application/json",
        target: TargetSpec::SERVED,
        read: |call, uri| views::entities_view(call, uri),
    },
];

/// Whether `uri` names a resource the server serves: a core one, or one an
/// extension contributes. The lookup `resources/read` makes
/// ([`crate::surface_call::find`]), so the freshness decision is its.
pub(crate) fn is_served(state: &mut McpState, uri: &str) -> bool {
    crate::surface_call::find::<Resources>(state, uri).is_some()
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
        Err(refused) => return Ran::of(Err(Box::new(refused))),
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
        Err(error) => Ran::of(Err(Box::new(McpError::from_diagnostic(
            &error.diagnostic(),
        )))),
    }
}

use crate::DEFAULT_CLIENT_ID as DEFAULT_SUBSCRIBER;
use crate::subscriptions::Watched;

/// MCP `resources/subscribe`: track the client's interest in a resource so
/// updates of the served project deliver delta notifications (C9-01). A URI
/// the server does not serve is refused as `resources/read` refuses it:
/// not found, the code of the revision of the request.
pub fn handle_resource_subscribe(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    let invocation = match Invocation::read::<Resources>(&params) {
        Ok(invocation) => invocation,
        Err(error) => return JsonRpcResponse::from_error(id, error),
    };
    let uri = invocation.name.as_str();
    // Served by the rule `resources/read` applies: a core resource, or an
    // extension's, the project brought up to date first (ADR 0014 D12,
    // ADR 0024 D2).
    if !is_served(state, uri) {
        return JsonRpcResponse::from_error(
            id,
            unknown_resource(state.resource_not_found_code(), uri),
        );
    }
    let client = params
        .get("client_id")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_SUBSCRIBER);
    crate::subscriptions::subscribe(state, client, Watched::of(uri));
    JsonRpcResponse::success(id, serde_json::json!({}))
}

/// MCP `resources/unsubscribe`: drop the client's interest in a resource. It
/// never refuses a URI: dropping what was never subscribed (or what an
/// extension stopped serving) is a no-op success.
pub fn handle_resource_unsubscribe(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    let invocation = match Invocation::read::<Resources>(&params) {
        Ok(invocation) => invocation,
        Err(error) => return JsonRpcResponse::from_error(id, error),
    };
    let client = params
        .get("client_id")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_SUBSCRIBER);
    crate::subscriptions::unsubscribe(state, client, Watched::of(&invocation.name));
    JsonRpcResponse::success(id, serde_json::json!({}))
}
