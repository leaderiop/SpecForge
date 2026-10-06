//! The core prompts: the `prompts/get` dispatcher over the Prompt spec
//! table ([`CORE_PROMPTS`]), the prompt-side twin of
//! [`crate::tools::handle_tool_call`].

mod context;
mod explore;
mod infer;
mod review;
mod table;
mod trace;

use serde_json::{Value, json};

use crate::prompt::{PromptSpec, prompt_envelope};
use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::target::{self, Call};
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

pub fn handle_prompt_get(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    // A request that fails GetPromptRequest's own schema is malformed: a
    // protocol error, as for tools/call.
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            "Missing required parameter: name",
        );
    };
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => Value::Object(Default::default()),
        Some(object @ Value::Object(_)) => object.clone(),
        Some(_) => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Invalid params: arguments must be an object",
            );
        }
    };
    // So is an unknown prompt: it is not an invocation.
    let Some(spec) = core_prompt(name) else {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!("Unknown prompt: {name}"),
        );
    };

    let mut event = json!({ "promptName": spec.name });
    for (argument, field) in [("entity_id", "entityId"), ("kind", "kind")] {
        if let Some(value) = arguments.get(argument).and_then(Value::as_str) {
            event[field] = Value::from(value);
        }
    }
    state.push_event("mcp_prompt_invoked", event);

    // The project the prompt reads, brought up to date with disk first.
    let outcome = match target::resolve(state, spec.target, &arguments) {
        Ok(target) => {
            let call = Call::new(state, target);
            (spec.render)(&call, arguments)
                .map_err(|refused| Box::new(target::without_project(call.target(), *refused)))
        }
        Err(refused) => Err(Box::new(crate::tool::McpError::from(refused))),
    };
    prompt_envelope(outcome, spec, id)
}
