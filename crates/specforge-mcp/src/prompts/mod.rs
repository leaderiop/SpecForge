//! The core prompts: the `prompts/get` adapter of the request pipeline
//! ([`crate::surface_call`]) over the Prompt spec table ([`CORE_PROMPTS`]).

mod context;
mod explore;
mod infer;
mod review;
mod table;
mod trace;

use serde_json::{Value, json};

use crate::prompt::{PromptOutcome, PromptSpec, prompt_envelope};
use crate::protocol::JsonRpcResponse;
use crate::state::McpState;
use crate::surface_call::{Event, Found, Invocation, Ran, Surface};
use crate::target::{Call, TargetSpec};
use crate::tool::McpError;
use crate::types::McpPromptDescriptor;
pub use table::CORE_PROMPTS;

/// The core prompt named `name`.
pub fn core_prompt(name: &str) -> Option<&'static PromptSpec> {
    CORE_PROMPTS.iter().find(|p| p.name == name)
}

/// What `prompts/list` lists and `initialize` reports: every core prompt,
/// in table order.
pub fn descriptors() -> Vec<McpPromptDescriptor> {
    CORE_PROMPTS.iter().map(PromptSpec::descriptor).collect()
}

/// `prompts/get`: the core prompt table (no extension declares a prompt).
/// The request pipeline's prompts adapter ([`crate::surface_call`]).
pub(crate) struct Prompts;

impl Surface for Prompts {
    const NAMED_BY: &'static str = "name";
    const TAKES_ARGUMENTS: bool = true;
    const EXTENDED: bool = false;
    const UNKNOWN: &'static str = "Unknown prompt";

    type Core = &'static PromptSpec;
    type Extension = std::convert::Infallible;
    type Outcome = PromptOutcome;

    fn core(name: &str) -> Option<&'static PromptSpec> {
        core_prompt(name)
    }

    fn extension(_: &McpState, _: &str) -> Option<std::convert::Infallible> {
        None
    }

    fn target(found: &Found<&'static PromptSpec, std::convert::Infallible>) -> TargetSpec {
        found.core_entry().target
    }

    fn invoked(
        found: &Found<&'static PromptSpec, std::convert::Infallible>,
        invocation: &Invocation,
    ) -> Option<Event> {
        let spec = found.core_entry();
        let mut event = json!({ "promptName": spec.name });
        for (argument, field) in [("entity_id", "entityId"), ("kind", "kind")] {
            if let Some(value) = invocation.arguments.get(argument).and_then(Value::as_str) {
                event[field] = Value::from(value);
            }
        }
        Some(("mcp_prompt_invoked".to_string(), event))
    }

    fn run(
        call: &mut Call<'_>,
        found: &Found<&'static PromptSpec, std::convert::Infallible>,
        invocation: &Invocation,
    ) -> Ran<PromptOutcome> {
        let spec = found.core_entry();
        Ran::of((spec.render)(call, invocation.arguments.clone()))
    }

    fn refused(
        _: &Found<&'static PromptSpec, std::convert::Infallible>,
        error: McpError,
    ) -> Ran<PromptOutcome> {
        Ran::of(Err(Box::new(error)))
    }

    fn refusal_mut(outcome: &mut PromptOutcome) -> Option<&mut McpError> {
        outcome.as_mut().err().map(|refusal| &mut **refusal)
    }

    fn completed(
        _: &Found<&'static PromptSpec, std::convert::Infallible>,
        _: &Invocation,
        _: &PromptOutcome,
    ) -> Option<Event> {
        None
    }

    fn envelope(
        _: &McpState,
        found: &Found<&'static PromptSpec, std::convert::Infallible>,
        _: &Invocation,
        outcome: PromptOutcome,
        id: Option<Value>,
    ) -> JsonRpcResponse {
        let spec = found.core_entry();
        prompt_envelope(outcome, spec, id)
    }
}
