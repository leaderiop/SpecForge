//! The core prompts: the `prompts/get` adapter of the request pipeline
//! ([`crate::surface_call`]) over the Prompt spec table ([`CORE_PROMPTS`]).

pub mod context;
pub mod explore;
pub(crate) mod extension;
pub mod infer;
pub mod review;
pub mod trace;

use serde_json::Value;
use specforge_common::Codebook;

use crate::events::{EventSink, McpEvent};
use crate::lifecycle::Revision;
use crate::prompt::{PromptOutcome, PromptSpec, prompt_envelope};
use crate::protocol::JsonRpcResponse;
use crate::state::McpState;
use crate::surface_call::{Found, Invocation, Surface};
use crate::target::{Call, TargetSpec};
use crate::tool::McpError;
use specforge_ops::extension::prompt::ExtensionPrompt;

/// The core prompts, in listing order.
pub static CORE_PROMPTS: &[&dyn PromptSpec] = &[
    &context::Prompt,
    &review::Prompt,
    &trace::Prompt,
    &explore::Prompt,
    &infer::Prompt,
];

/// The core prompt named `name`.
pub fn core_prompt(name: &str) -> Option<&'static dyn PromptSpec> {
    CORE_PROMPTS.iter().copied().find(|p| p.name() == name)
}

/// `prompts/get`: the core prompt table, then the served project's extension prompts (ADR 0291).
/// The request pipeline's prompts adapter ([`crate::surface_call`]).
pub(crate) struct Prompts;

type PromptFound = Found<&'static dyn PromptSpec, ExtensionPrompt>;

impl Surface for Prompts {
    const NAMED_BY: &'static str = "name";
    const TAKES_ARGUMENTS: bool = true;
    const EXTENDED: bool = true;
    const UNKNOWN: &'static str = "Unknown prompt";

    type Core = &'static dyn PromptSpec;
    type Extension = ExtensionPrompt;
    type Outcome = PromptOutcome;

    fn core(name: &str) -> Option<&'static dyn PromptSpec> {
        core_prompt(name)
    }

    fn extension(state: &McpState, name: &str) -> Option<ExtensionPrompt> {
        crate::target::served_prompts(state)
            .by_mcp_name(name)
            .cloned()
    }

    fn target(found: &PromptFound) -> TargetSpec {
        match found {
            Found::Core(spec) => spec.target(),
            Found::Extension(_) => TargetSpec::SERVED_VIEW,
        }
    }

    fn owns_path(found: &PromptFound) -> bool {
        matches!(found, Found::Extension(p) if p.declaration().args.iter().any(|a| a.name == "path"))
    }

    fn invoked(found: &PromptFound, invocation: &Invocation) -> Option<McpEvent> {
        let argument = |name: &str| {
            invocation
                .arguments
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        Some(match found {
            Found::Core(spec) => McpEvent::McpPromptInvoked {
                prompt_name: spec.name().to_string(),
                entity_id: argument("entity_id"),
                kind: argument("kind"),
            },
            // The entity an extension prompt is about is its subject argument's.
            Found::Extension(prompt) => McpEvent::McpPromptInvoked {
                prompt_name: prompt.mcp_name(),
                entity_id: prompt.declaration().subject.as_deref().and_then(argument),
                kind: None,
            },
        })
    }

    fn run(call: &mut Call<'_>, found: &PromptFound, invocation: &Invocation) -> PromptOutcome {
        match found {
            Found::Core(spec) => {
                if let Some(error) = spec.undeclared(&invocation.arguments) {
                    return Err(Box::new(error));
                }
                spec.get(call.view(), &invocation.arguments)
            }
            Found::Extension(prompt) => {
                extension::get(&call.view(), prompt, &invocation.arguments, call.host())
            }
        }
    }

    fn refused(
        _: &PromptFound,
        _: &Invocation,
        error: McpError,
        _: &mut dyn EventSink,
    ) -> PromptOutcome {
        Err(Box::new(error))
    }

    fn refusal_mut(outcome: &mut PromptOutcome) -> Option<&mut McpError> {
        outcome.as_mut().err().map(|refusal| &mut **refusal)
    }

    fn completed(_: &PromptFound, _: &Invocation, _: &PromptOutcome) -> Option<McpEvent> {
        None
    }

    fn envelope(
        _: Revision,
        found: &PromptFound,
        _: &Invocation,
        outcome: PromptOutcome,
        _: &Codebook,
        id: Option<Value>,
    ) -> JsonRpcResponse {
        match found {
            Found::Core(spec) => prompt_envelope(outcome, spec.name(), spec.description(), id),
            Found::Extension(prompt) => prompt_envelope(
                outcome,
                &prompt.mcp_name(),
                &prompt.declaration().description,
                id,
            ),
        }
    }
}
