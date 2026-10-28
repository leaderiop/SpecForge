//! The prompts extensions contribute (ADR 0291), served beside the core ones: listed after them by
//! `prompts/list` and rendered by `prompts/get` and by `specforge.prompt`, all through the one render
//! (`specforge_ops::extension::prompt::render`). Which prompts are served is the ops list's, read from the served
//! project's view, so a prompt whose `when` setting is off is neither listed nor found.

use serde_json::Value;
use specforge_ops::extension::prompt::{
    ExtensionPrompt, PromptArgs, RenderError, RenderedPrompt, render,
};
use specforge_ops::host::Host;
use specforge_ops::view::ProjectView;
use specforge_protocol_types::{PromptArgDescriptor, PromptMessage, PromptRole};

use crate::prompt::{PromptOutcome, Rendered};
use crate::registry::{PromptArgument, PromptDescriptor};
use crate::tool::McpError;

/// What an argument takes, in words: its field type, a reference's kinds, an enum's values, an integer's bounds.
fn takes(arg: &PromptArgDescriptor) -> String {
    let mut text = arg.field_type.clone();
    if let Some(kinds) = &arg.target_kind {
        text = format!("{text} of {}", kinds.replace('|', " or "));
    }
    if !arg.enum_values.is_empty() {
        text = format!("{text}: {}", arg.enum_values.join(", "));
    }
    match (arg.minimum, arg.maximum) {
        (Some(least), Some(most)) => text = format!("{text} {least}..{most}"),
        (Some(least), None) => text = format!("{text} >= {least}"),
        (None, Some(most)) => text = format!("{text} <= {most}"),
        (None, None) => {}
    }
    text
}

/// `prompt` as `prompts/list` describes it: each argument's type is in its description, the one place MCP lets a
/// prompt say it.
pub(crate) fn descriptor(prompt: &ExtensionPrompt) -> PromptDescriptor {
    let declared = prompt.declaration();
    PromptDescriptor {
        name: prompt.mcp_name(),
        description: declared.description.clone(),
        arguments: declared
            .args
            .iter()
            .map(|arg| PromptArgument {
                name: arg.name.clone(),
                description: match &arg.description {
                    Some(described) => format!("{described} ({})", takes(arg)),
                    None => format!("({})", takes(arg)),
                },
                required: arg.required,
            })
            .collect(),
    }
}

/// `prompt` rendered over `view` for `host`, or the refusal.
pub(crate) fn rendered(
    view: &ProjectView<'_>,
    prompt: &ExtensionPrompt,
    given: PromptArgs<'_>,
    host: Option<Host>,
) -> Result<RenderedPrompt, Box<McpError>> {
    render(view, prompt, given, host).map_err(|error| {
        Box::new(match error {
            RenderError::Refused(e) | RenderError::Gate(e) => McpError::from(*e),
            RenderError::Call(e) => crate::extension_dispatch::failed_call(&e.diagnostic()),
        })
    })
}

/// What an extension prompt rendered, as the messages of one prompt: what its export said, then the host's payload as
/// JSON text.
pub(crate) fn messages(rendered: RenderedPrompt) -> Vec<PromptMessage> {
    let RenderedPrompt {
        mut messages,
        payload,
    } = rendered;
    messages.push(PromptMessage {
        role: PromptRole::User,
        text: serde_json::to_string(&payload).expect("a payload serializes"),
    });
    messages
}

/// A `prompts/get` of an extension prompt.
pub(crate) fn get(
    view: &ProjectView<'_>,
    prompt: &ExtensionPrompt,
    arguments: &Value,
    host: Option<Host>,
) -> PromptOutcome {
    let named = arguments.as_object().cloned().unwrap_or_default();
    let rendered = rendered(view, prompt, PromptArgs::Named(&named), host)?;
    Ok(Rendered {
        messages: messages(rendered),
    })
}
