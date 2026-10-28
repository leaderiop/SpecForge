//! Extension prompts (ADR 0291): the MCP prompts extensions declare in their surfaces, each a `prompt__<id>` Wasm
//! export. [`ExtensionPrompt`] derives one prompt once for every surface (`specforge.<short>.<id>`, `specforge
//! <short> <cli-name>`); [`ExtensionPrompts`] is the project's, as the registry build routes them and as their
//! `when` settings serve them. [`render`] is the one operation both surfaces run: it reads the typed arguments
//! ([`args`]), evaluates the entry gate in the subject's scope, calls the export and appends the host's message:
//! the prompt's version, a digest of what the export said, its attribution and each step with the state of its gate.

pub mod args;

use serde::Serialize;
use serde_json::Value;
use specforge_package::Sha256;
use specforge_protocol_types::command_args::option_name;
use specforge_protocol_types::{
    Attribution, PromptDescriptor, PromptInput, PromptMessage, PromptStep, RawGraph,
};
use specforge_wasm::{CallError, CallFailure, ExtensionCalls, Operation};

pub use args::{PromptArgs, words};

use super::command::{ExtensionCommand, ExtensionCommands, canonical, graph_export, today};
use super::gate::{self, Gate, GateState};
use crate::host::Host;
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};

/// One prompt an extension declares, derived once for every surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionPrompt {
    extension: String,
    version: String,
    short: String,
    declaration: PromptDescriptor,
}

impl ExtensionPrompt {
    /// The declaring extension's name.
    pub fn extension(&self) -> &str {
        &self.extension
    }

    /// The declaring extension's short name, which routes its prompts.
    pub fn short(&self) -> &str {
        &self.short
    }

    pub fn id(&self) -> &str {
        &self.declaration.id
    }

    pub fn declaration(&self) -> &PromptDescriptor {
        &self.declaration
    }

    /// Its name on the command line: its id, `_` spelled `-`.
    pub fn cli_name(&self) -> String {
        option_name(&self.declaration.id)
    }

    /// Its MCP prompt's name: `specforge.{short}.{id}`.
    pub fn mcp_name(&self) -> String {
        format!("specforge.{}.{}", self.short, self.declaration.id)
    }

    /// Its version as the payload names it: `@ext@version:id`.
    pub fn qualified(&self) -> String {
        format!(
            "{}@{}:{}",
            self.extension, self.version, self.declaration.id
        )
    }
}

/// The project's extension prompts that are served.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtensionPrompts {
    prompts: Vec<ExtensionPrompt>,
}

impl ExtensionPrompts {
    /// The prompts `view`'s registry build routes whose `when` setting is not false, in its order.
    pub fn build(view: &ProjectView<'_>) -> Self {
        let served = |extension: &str, when: &Option<String>| {
            let Some(setting) = when else {
                return true;
            };
            let value = view
                .runtime()
                .and_then(|r| r.settings_of(extension))
                .and_then(|s| s.values.get(setting).cloned());
            value != Some(Value::Bool(false))
        };
        ExtensionPrompts {
            prompts: view
                .registries()
                .prompts()
                .iter()
                .filter(|r| served(r.extension(), &r.declaration().when))
                .map(|route| ExtensionPrompt {
                    extension: route.extension().to_string(),
                    version: route.version().to_string(),
                    short: route.short().to_string(),
                    declaration: route.declaration().clone(),
                })
                .collect(),
        }
    }

    pub fn all(&self) -> &[ExtensionPrompt] {
        &self.prompts
    }

    /// The prompt whose MCP name is `name`.
    pub fn by_mcp_name(&self, name: &str) -> Option<&ExtensionPrompt> {
        self.prompts.iter().find(|p| p.mcp_name() == name)
    }

    /// The prompt of the extension with short name `short` that `specforge <short> <cli_name>` names.
    pub fn by_cli_name(&self, short: &str, cli_name: &str) -> Option<&ExtensionPrompt> {
        self.prompts
            .iter()
            .find(|p| p.short == short && p.cli_name() == cli_name)
    }
}

/// One step of a rendered prompt, with its gate evaluated now.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RenderedStep {
    pub id: String,
    pub title: String,
    /// The MCP tool a command step runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// The same command on the command line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli: Option<String>,
    /// The MCP prompt a sub-prompt step renders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// A command step's gate, each code's state now; none for any other step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<Vec<GateState>>,
}

/// What the host adds after the export's messages (`McpExtensionPromptPayload`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExtensionPromptPayload {
    /// `@ext@version:id`.
    pub prompt: String,
    /// `sha256:` of the export's message texts, in order.
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<Attribution>,
    /// The entity the gates were scoped to, as its full path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<RenderedStep>,
}

/// A rendered prompt: the export's messages, and what the host says about them.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedPrompt {
    pub messages: Vec<PromptMessage>,
    pub payload: ExtensionPromptPayload,
}

impl RenderedPrompt {
    /// The prompt as `specforge prompt --format json` and `specforge.prompt` write it: `{messages: [{role, text}],
    /// payload}`.
    pub fn document(&self) -> Value {
        serde_json::json!({
            "messages": self.messages,
            "payload": self.payload,
        })
    }
}

/// Why a prompt did not render.
#[derive(Debug)]
pub enum RenderError {
    /// An argument is refused (`invalid_input`, `entity_not_found`), or the view has no project on disk
    /// (`no_project`).
    Refused(Box<OpError>),
    /// The entry gate fails (`gate_failed`); the export was not called.
    Gate(Box<OpError>),
    /// The export did not answer a `PromptOutput` (E028).
    Call(CallError),
}

impl From<OpError> for RenderError {
    fn from(error: OpError) -> Self {
        RenderError::Refused(Box::new(error))
    }
}

/// Render `prompt` over the project `view` describes, for `host`, with the arguments `given`. In order: the
/// arguments are read and admitted; the entry gate is evaluated in the subject's scope (a failing gate is
/// [`RenderError::Gate`], and the export is not called); the export is called with the typed arguments, the project
/// root, today, the host and, if the prompt reads it, the graph; the payload is built from the answer.
pub fn render(
    view: &ProjectView<'_>,
    prompt: &ExtensionPrompt,
    given: PromptArgs<'_>,
    host: Option<Host>,
) -> Result<RenderedPrompt, RenderError> {
    let declared = &prompt.declaration;
    let root = view.project_root().map_err(RenderError::from)?;
    let args = args::read(view, &declared.args, given)?;
    let subject = declared
        .subject
        .as_ref()
        .and_then(|name| args.get(name))
        .and_then(Value::as_str)
        .map(str::to_string);
    if !declared.gates.is_empty() {
        let states = Gate {
            extension: &prompt.extension,
            codes: &declared.gates,
            subject: subject.as_deref(),
            on: &declared.id,
        }
        .evaluate(view);
        if !gate::passes(&states) {
            return Err(RenderError::Gate(Box::new(gate::refusal(
                &prompt.mcp_name(),
                &states,
            ))));
        }
    }
    let input = PromptInput::<RawGraph> {
        args,
        cwd: canonical(root).display().to_string(),
        today: today(),
        graph: declared.reads_graph.then(|| {
            RawGraph::new(graph_export(view)).expect("the graph export is one JSON value")
        }),
        host: host.map(|h| h.as_str().to_string()),
        state: declared
            .reads_state
            .then(|| crate::host::state::read(view, &prompt.extension, None)),
    };
    let Some(runtime) = view.runtime() else {
        return Err(RenderError::Call(CallError::new(
            Operation::Prompt,
            &prompt.extension,
            &declared.export,
            CallFailure::NotLoaded,
        )));
    };
    let output = ExtensionCalls::new(runtime.as_ref())
        .render_prompt(&prompt.extension, &declared.export, &input)
        .map_err(RenderError::Call)?;
    let texts: String = output.messages.iter().map(|m| m.text.as_str()).collect();
    let payload = ExtensionPromptPayload {
        prompt: prompt.qualified(),
        digest: format!("sha256:{}", Sha256::of(texts.as_bytes())),
        attribution: declared.attribution.clone(),
        steps: steps(view, prompt, subject.as_deref()),
        subject,
    };
    Ok(RenderedPrompt {
        messages: output.messages,
        payload,
    })
}

/// `prompt`'s steps with each command step's gate evaluated now.
fn steps(
    view: &ProjectView<'_>,
    prompt: &ExtensionPrompt,
    subject: Option<&str>,
) -> Vec<RenderedStep> {
    let commands = ExtensionCommands::build(view.registries());
    let prompts = ExtensionPrompts::build(view);
    prompt
        .declaration
        .steps
        .iter()
        .map(|step| rendered(view, (&commands, &prompts), prompt, step, subject))
        .collect()
}

fn rendered(
    view: &ProjectView<'_>,
    (commands, prompts): (&ExtensionCommands, &ExtensionPrompts),
    prompt: &ExtensionPrompt,
    step: &PromptStep,
    subject: Option<&str>,
) -> RenderedStep {
    let mut out = RenderedStep {
        id: step.id.clone(),
        title: step.title.clone(),
        tool: None,
        cli: None,
        prompt: None,
        gate: None,
    };
    // `<peer>:<id>` names a declared peer by name or short name; a bare id is this extension's.
    let Some(named) = step.command.as_deref().or(step.prompt.as_deref()) else {
        return out;
    };
    let (peer, id) = named
        .split_once(':')
        .map_or((None, named), |(p, i)| (Some(p), i));
    let owned = |extension: &str, short: &str| match peer {
        Some(p) => p == extension || p == short,
        None => extension == prompt.extension,
    };
    if step.command.is_some() {
        let found = commands
            .all()
            .iter()
            .find(|c| c.id() == id && owned(c.extension(), c.short()));
        if let Some(command) = found {
            out.tool = Some(command.tool_name());
            out.cli = Some(format!(
                "specforge {} {}",
                command.short(),
                command.cli_name()
            ));
            out.gate = Some(command_gate(view, command, subject));
        }
    } else if let Some(sub) = prompts
        .all()
        .iter()
        .find(|p| p.id() == id && owned(p.extension(), p.short()))
    {
        out.prompt = Some(sub.mcp_name());
    }
    out
}

fn command_gate(
    view: &ProjectView<'_>,
    command: &ExtensionCommand,
    subject: Option<&str>,
) -> Vec<GateState> {
    let declared = command.declaration();
    Gate {
        extension: command.extension(),
        codes: &declared.gates,
        subject: subject.filter(|_| declared.subject.is_some()),
        on: command.id(),
    }
    .evaluate(view)
}

/// The refusal of a prompt the project does not serve.
pub fn unknown(name: &str) -> OpError {
    OpError::new(
        OpErrorKind::EntityNotFound,
        "unknown_prompt",
        format!("no prompt '{name}'"),
    )
}
