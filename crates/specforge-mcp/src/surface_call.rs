//! One MCP request that invokes something by name — `tools/call`,
//! `resources/read`, `prompts/get` — through one pipeline (ADR 0024): read
//! the request, find what it names (the core table, then, with the served
//! project brought up to date, the extension surface table), record the
//! invocation, resolve the call target, run the handler, record what it
//! reported, and answer with the kind's envelope. Each request kind is one
//! [`Surface`] adapter; no other code builds the reply of these three
//! methods. The router guards initialization.

use serde_json::Value;

use crate::protocol::{JsonRpcError, JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::target::{self, Call, CallTarget, TargetSpec};
use crate::tool::McpError;

/// An event to record: its name and payload.
pub(crate) type Event = (String, Value);

/// What one request invokes: a tool's or prompt's name with its `arguments`
/// object, or the URI a resource read names (with `{}`).
pub(crate) struct Invocation {
    pub name: String,
    pub arguments: Value,
}

impl Invocation {
    /// The invocation `params` carry for request kind `S`. A request that
    /// fails its method's own schema is malformed, -32602 (ADR 0004 D4-a):
    /// no `S::NAMED_BY` string ("Missing required parameter: name"), or
    /// `arguments` that is neither absent, null nor an object ("Invalid
    /// params: arguments must be an object").
    pub fn read<S: Surface>(params: &Value) -> Result<Self, JsonRpcError> {
        let Some(name) = params.get(S::NAMED_BY).and_then(Value::as_str) else {
            return Err(JsonRpcError::new(
                error_codes::INVALID_PARAMS,
                format!("Missing required parameter: {}", S::NAMED_BY),
            ));
        };
        let arguments = if S::TAKES_ARGUMENTS {
            match params.get("arguments") {
                None | Some(Value::Null) => Value::Object(Default::default()),
                Some(object @ Value::Object(_)) => object.clone(),
                Some(_) => {
                    return Err(JsonRpcError::new(
                        error_codes::INVALID_PARAMS,
                        "Invalid params: arguments must be an object",
                    ));
                }
            }
        } else {
            Value::Object(Default::default())
        };
        Ok(Invocation {
            name: name.to_string(),
            arguments,
        })
    }
}

/// The entry a request names: a core table's, or the extension surface
/// table's (ADR 0017 D7).
pub(crate) enum Found<C, E> {
    Core(C),
    Extension(E),
}

impl<C: Copy> Found<C, std::convert::Infallible> {
    /// The core entry, for a kind no extension contributes to.
    pub fn core_entry(&self) -> C {
        match self {
            Found::Core(core) => *core,
            Found::Extension(never) => match *never {},
        }
    }
}

/// What a handler produced, with the events to record whatever it was (an
/// extension's dispatch event is recorded for a failed dispatch too).
pub(crate) struct Ran<O> {
    pub outcome: O,
    pub events: Vec<Event>,
}

impl<O> Ran<O> {
    /// A result that records nothing beside the pipeline's own events.
    pub fn of(outcome: O) -> Self {
        Ran {
            outcome,
            events: Vec::new(),
        }
    }
}

/// One request kind. Three adapters: [`crate::tools::Tools`],
/// [`crate::resources::Resources`], [`crate::prompts::Prompts`].
pub(crate) trait Surface {
    /// The parameter naming what is invoked: `"name"` or `"uri"`.
    const NAMED_BY: &'static str;
    /// Whether the request carries an `arguments` object.
    const TAKES_ARGUMENTS: bool;
    /// Whether extensions contribute entries of this kind (tools,
    /// resources; no extension declares a prompt).
    const EXTENDED: bool;
    /// How an unknown name is refused: "Unknown tool", "Unknown resource
    /// URI", "Unknown prompt" (-32602, before anything is recorded).
    const UNKNOWN: &'static str;

    type Core: Copy;
    type Extension;
    type Outcome;

    /// The core entry `name` names.
    fn core(name: &str) -> Option<Self::Core>;
    /// The served project's extension entry `name` names, asked after the
    /// project was brought up to date.
    fn extension(state: &McpState, name: &str) -> Option<Self::Extension>;
    /// How the entry reaches its project.
    fn target(found: &Found<Self::Core, Self::Extension>) -> TargetSpec;
    /// Recorded once the entry is found, before its target resolves
    /// (`mcp_tool_invoked`, `mcp_prompt_invoked`); none for a read.
    fn invoked(
        found: &Found<Self::Core, Self::Extension>,
        invocation: &Invocation,
    ) -> Option<Event>;
    /// Run the entry over its resolved call.
    fn run(
        call: &mut Call<'_>,
        found: &Found<Self::Core, Self::Extension>,
        invocation: &Invocation,
    ) -> Ran<Self::Outcome>;
    /// What a refusal of the call's target is: the outcome, and what a
    /// refused mutation still records.
    fn refused(found: &Found<Self::Core, Self::Extension>, error: McpError) -> Ran<Self::Outcome>;
    /// The outcome of a call whose handler ran, as its target makes it: with
    /// nothing served ([`CallTarget::NoProject`]), a refusal that names a
    /// file or an entity is the no-project refusal ([`target::without_project`],
    /// ADR 0025).
    fn without_project(target: &CallTarget, outcome: Self::Outcome) -> Self::Outcome;
    /// Recorded after the handler's events (`mcp_resource_read`).
    fn completed(
        found: &Found<Self::Core, Self::Extension>,
        invocation: &Invocation,
        outcome: &Self::Outcome,
    ) -> Option<Event>;
    /// The reply: the only place this kind builds its result or its error,
    /// a failure naming the entry (`tool`, `prompt`, `uri`).
    fn envelope(
        state: &McpState,
        found: &Found<Self::Core, Self::Extension>,
        invocation: &Invocation,
        outcome: Self::Outcome,
        id: Option<Value>,
    ) -> JsonRpcResponse;
}

/// Serve one `tools/call`, `resources/read` or `prompts/get`.
pub(crate) fn serve<S: Surface>(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    let invocation = match Invocation::read::<S>(&params) {
        Ok(invocation) => invocation,
        Err(error) => return JsonRpcResponse::from_error(id, error),
    };
    let Some(found) = find::<S>(state, &invocation.name) else {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!("{}: {}", S::UNKNOWN, invocation.name),
        );
    };
    if let Some((name, event)) = S::invoked(&found, &invocation) {
        state.push_event(name, event);
    }
    // The project the call acts on, resolved (and brought up to date)
    // before the handler runs: handlers never pick a root or reload.
    let Ran { outcome, events } =
        match target::resolve(state, S::target(&found), &invocation.arguments) {
            Ok(target) => {
                let mut call = Call::new(state, target);
                let ran = S::run(&mut call, &found, &invocation);
                Ran {
                    outcome: S::without_project(call.target(), ran.outcome),
                    events: ran.events,
                }
            }
            Err(refused) => S::refused(&found, McpError::from(refused)),
        };
    for (name, event) in events {
        state.push_event(name, event);
    }
    if let Some((name, event)) = S::completed(&found, &invocation, &outcome) {
        state.push_event(name, event);
    }
    S::envelope(state, &found, &invocation, outcome, id)
}

/// The entry `name` names: a core one; else, for a kind extensions
/// contribute, the extension surface table's, the served project brought up
/// to date first, so an extension enabled on disk since the last request is
/// found (ADR 0014 D12, ADR 0024 D2).
fn find<S: Surface>(state: &mut McpState, name: &str) -> Option<Found<S::Core, S::Extension>> {
    if let Some(core) = S::core(name) {
        return Some(Found::Core(core));
    }
    if !S::EXTENDED {
        return None;
    }
    if state.project_root().is_some() {
        state.ensure_fresh();
    }
    S::extension(state, name).map(Found::Extension)
}
