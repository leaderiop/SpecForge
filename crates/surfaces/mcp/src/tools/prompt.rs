//! `specforge.prompt`: render any prompt by name, the way an agent whose host lists no MCP prompts does (ADR 0291).

use serde::Serialize;
use serde_json::{Map, Value};
use specforge_common::shape::Shape;
use specforge_ops::extension::prompt::{ExtensionPrompts, PromptArgs, words};
use specforge_ops::host::Host;

use crate::args::Arguments;
use crate::prompts::{core_prompt, extension};
use crate::reply::Answered;
use crate::target::{Hosted, HostedView};
use crate::tool::McpError;
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup};

/// `specforge.prompt`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// The prompt: `specforge.<short>.<id>` for an extension's, a core prompt's name (`review`)
    name: String,
    /// The prompt's arguments by name; a reference as it is spelled, a list as a JSON array or a comma-separated string
    arguments: Option<Map<String, Value>>,
    /// The prompt's arguments as one `$ARGUMENTS` string, read as the command line reads them; instead of arguments
    argv: Option<String>,
    /// The host to render for: claude-code, codex or cursor; absent, the one the client named
    host: Option<String>,
}

/// `specforge.prompt`'s reply: the messages the prompt says, and the host's payload.
#[derive(Debug, Serialize, Shape)]
pub struct Reply {
    /// What the prompt says, in order.
    messages: Vec<Message>,
    /// An extension prompt's: its version, digest, attribution, subject and steps with their gates; a core prompt's
    /// is its own payload. An open value.
    payload: Value,
}

/// One message of a rendered prompt.
#[derive(Debug, Serialize, Shape)]
pub struct Message {
    role: String,
    text: String,
}

/// `specforge.prompt`: an extension prompt rendered through the ops render both surfaces share, a core prompt through
/// its own, over the served project.
pub(crate) struct Tool;

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.prompt";
    const DESCRIPTION: &'static str = "Render a prompt by name: an extension's (specforge.<short>.<id>, with its typed arguments or an argv string) or a core prompt's; the reply is the prompt's messages and the host's payload with each step's gate";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Core,
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpPromptResult");
    const OPEN_VALUES: &'static [&'static str] = &["$.payload"];
    type Args = Args;
    type Input = HostedView;
    type Returns = Answered<Reply>;

    fn call(hosted: Hosted<'_>, args: Args) -> Answered<Reply> {
        let Hosted { view, host } = hosted;
        if args.arguments.is_some() && args.argv.is_some() {
            return Err(Box::new(McpError::invalid_input(
                "argv",
                "give the arguments or argv, not both",
            )));
        }
        let host = match args.host.as_deref() {
            Some(flag) => Some(Host::from_flag(flag).ok_or_else(|| {
                Box::new(McpError::invalid_input(
                    "host",
                    format!("unknown host '{flag}': claude-code, codex or cursor"),
                ))
            })?),
            None => host,
        };
        let core = core_prompt(&args.name)
            .or_else(|| core_prompt(&format!("specforge://prompts/{}", args.name)));
        if let Some(spec) = core {
            if args.argv.is_some() {
                return Err(Box::new(McpError::invalid_input(
                    "argv",
                    "a core prompt takes its arguments by name",
                )));
            }
            let given = Value::Object(args.arguments.unwrap_or_default());
            if let Some(error) = spec.undeclared(&given) {
                return Err(Box::new(error));
            }
            let rendered = spec.get(view, &given)?;
            let mut texts = rendered.messages.into_iter().map(|m| m.text);
            let instruction = texts.next().unwrap_or_default();
            let payload = texts
                .next()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(Value::Null);
            return Ok(Reply {
                messages: vec![Message {
                    role: "user".into(),
                    text: instruction,
                }],
                payload,
            }
            .into());
        }
        let served = ExtensionPrompts::build(&view);
        let Some(prompt) = served.by_mcp_name(&args.name) else {
            return Err(Box::new(
                McpError::invalid_input("name", format!("no prompt '{}'", args.name))
                    .with_data(serde_json::json!({ "prompts": served.all().iter().map(|p| p.mcp_name()).collect::<Vec<_>>() })),
            ));
        };
        let spelled;
        let given = match (&args.arguments, &args.argv) {
            (_, Some(argv)) => {
                spelled = words(argv);
                PromptArgs::Words(&spelled)
            }
            (Some(named), None) => PromptArgs::Named(named),
            (None, None) => {
                spelled = Vec::new();
                PromptArgs::Words(&spelled)
            }
        };
        let rendered = extension::rendered(&view, prompt, given, host)?;
        let payload = serde_json::to_value(&rendered.payload).expect("a payload serializes");
        Ok(Reply {
            messages: rendered
                .messages
                .into_iter()
                .map(|m| Message {
                    role: serde_json::to_value(m.role)
                        .ok()
                        .and_then(|r| r.as_str().map(str::to_string))
                        .unwrap_or_default(),
                    text: m.text,
                })
                .collect(),
            payload,
        }
        .into())
    }
}
