//! MCP prompts an extension contributes (ADR 0291): a prompt is declared with its typed arguments, the gates that
//! refuse it, the steps it asks of the agent and the handler that renders its messages.
//!
//! ```
//! # use specforge_extension_sdk::prelude::*;
//! # let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
//! c.prompt("plan_phase", |p| {
//!     p.title("Plan a phase")
//!         .description("Plan the phase, then check the plans")
//!         .arg("phase", |a| {
//!             a.reference("phase").required();
//!         })
//!         .arg("research", |a| {
//!             a.bool();
//!         })
//!         .subject("phase")
//!         .step("plan", |s| {
//!             s.title("Write the plans");
//!         })
//!         .handler(|call: &PromptCall<'_>| {
//!             let phase = call.str("phase").unwrap_or_default();
//!             Ok(PromptOutput::user(format!("Plan phase {phase}")))
//!         });
//! });
//! assert_eq!(c.declaration().surfaces.prompts[0].export, "prompt__plan_phase");
//! ```

use serde_json::Value;
use specforge_protocol_types::{
    Attribution, GraphWire, PromptArgDescriptor, PromptDescriptor, PromptInput, PromptOutput,
    PromptStep,
};

use crate::operations::Wire;

/// The export that renders the prompt `id`.
pub fn prompt_export(id: &str) -> String {
    format!("prompt__{id}")
}

/// Builder for one prompt ([`crate::ContributionsBuilder::prompt`]).
pub struct PromptBuilder {
    pub(crate) descriptor: PromptDescriptor,
    pub(crate) render: Option<Wire>,
}

impl PromptBuilder {
    pub(crate) fn new(id: &str) -> Self {
        PromptBuilder {
            descriptor: PromptDescriptor {
                id: id.to_string(),
                export: prompt_export(id),
                ..Default::default()
            },
            render: None,
        }
    }

    /// The one-line title a prompt listing shows.
    pub fn title(&mut self, title: &str) -> &mut Self {
        self.descriptor.title = title.to_string();
        self
    }

    pub fn description(&mut self, description: &str) -> &mut Self {
        self.descriptor.description = description.to_string();
        self
    }

    /// Declare the argument `name` (a string unless `f` says otherwise).
    pub fn arg(&mut self, name: &str, f: impl FnOnce(&mut PromptArgBuilder)) -> &mut Self {
        let mut b = PromptArgBuilder(PromptArgDescriptor {
            name: name.to_string(),
            field_type: "string".to_string(),
            ..Default::default()
        });
        f(&mut b);
        self.descriptor.args.push(b.0);
        self
    }

    /// The prompt is refused while the local code `code` fails (the entry gate).
    pub fn gate(&mut self, code: &str) -> &mut Self {
        self.descriptor.gates.push(code.to_string());
        self
    }

    /// The `reference` argument whose entity scopes the gates.
    pub fn subject(&mut self, arg: &str) -> &mut Self {
        self.descriptor.subject = Some(arg.to_string());
        self
    }

    /// A step the agent takes, in declaration order.
    pub fn step(&mut self, id: &str, f: impl FnOnce(&mut PromptStepBuilder)) -> &mut Self {
        let mut b = PromptStepBuilder(PromptStep {
            id: id.to_string(),
            ..Default::default()
        });
        f(&mut b);
        self.descriptor.steps.push(b.0);
        self
    }

    /// The handler receives the graph ([`PromptCall::graph`]); a workflow usually reads through tools.
    pub fn reads_graph(&mut self) -> &mut Self {
        self.descriptor.reads_graph = true;
        self
    }

    /// The prompt is served while the bool setting `setting` of the extension is not false.
    pub fn when(&mut self, setting: &str) -> &mut Self {
        self.descriptor.when = Some(setting.to_string());
        self
    }

    /// Where the prompt's text was ported from.
    pub fn attribution(&mut self, attribution: Attribution) -> &mut Self {
        self.descriptor.attribution = Some(attribution);
        self
    }

    /// The function that renders the prompt: its messages, or the error that fails the call.
    pub fn handler(
        &mut self,
        handler: impl Fn(&PromptCall<'_>) -> Result<PromptOutput, String> + 'static,
    ) -> &mut Self {
        self.render = Some(crate::operations::wire(
            "prompt",
            move |input: &PromptInput| handler(&PromptCall(input)),
        ));
        self
    }
}

/// Builder for one prompt argument ([`PromptBuilder::arg`]).
pub struct PromptArgBuilder(PromptArgDescriptor);

impl PromptArgBuilder {
    pub fn string(&mut self) -> &mut Self {
        self.typed("string")
    }
    pub fn integer(&mut self) -> &mut Self {
        self.typed("integer")
    }
    pub fn bool(&mut self) -> &mut Self {
        self.typed("bool")
    }
    pub fn date(&mut self) -> &mut Self {
        self.typed("date")
    }
    pub fn path(&mut self) -> &mut Self {
        self.typed("path")
    }
    pub fn string_list(&mut self) -> &mut Self {
        self.typed("string_list")
    }
    /// One of `values`.
    pub fn one_of(&mut self, values: &[&str]) -> &mut Self {
        self.0.enum_values = values.iter().map(|v| v.to_string()).collect();
        self.typed("enum")
    }
    /// Any number of `values`.
    pub fn many_of(&mut self, values: &[&str]) -> &mut Self {
        self.0.enum_values = values.iter().map(|v| v.to_string()).collect();
        self.typed("enum_list")
    }
    /// An entity of `kind` (several as `a|b`), read as its full path.
    pub fn reference(&mut self, kind: &str) -> &mut Self {
        self.0.target_kind = Some(kind.to_string());
        self.typed("reference")
    }
    pub fn reference_list(&mut self, kind: &str) -> &mut Self {
        self.0.target_kind = Some(kind.to_string());
        self.typed("reference_list")
    }
    /// The least value of an integer.
    pub fn minimum(&mut self, n: i64) -> &mut Self {
        self.0.minimum = Some(n);
        self
    }
    /// The greatest value of an integer.
    pub fn maximum(&mut self, n: i64) -> &mut Self {
        self.0.maximum = Some(n);
        self
    }
    pub fn required(&mut self) -> &mut Self {
        self.0.required = true;
        self
    }
    /// The value the argument has when the caller leaves it out.
    pub fn default(&mut self, value: impl Into<Value>) -> &mut Self {
        self.0.default = Some(value.into());
        self
    }
    pub fn description(&mut self, text: &str) -> &mut Self {
        self.0.description = Some(text.to_string());
        self
    }

    fn typed(&mut self, field_type: &str) -> &mut Self {
        self.0.field_type = field_type.to_string();
        self
    }
}

/// Builder for one step of a prompt ([`PromptBuilder::step`]). At most one of `command`, `agent`, `prompt`.
pub struct PromptStepBuilder(PromptStep);

impl PromptStepBuilder {
    pub fn title(&mut self, title: &str) -> &mut Self {
        self.0.title = title.to_string();
        self
    }
    /// The step runs the command `id` of this extension (`<peer>:<id>` of a declared peer's).
    pub fn command(&mut self, id: &str) -> &mut Self {
        self.0.command = Some(id.to_string());
        self
    }
    /// The step is done by the agent `id`.
    pub fn agent(&mut self, id: &str) -> &mut Self {
        self.0.agent = Some(id.to_string());
        self
    }
    /// The step is the prompt `id`.
    pub fn prompt(&mut self, id: &str) -> &mut Self {
        self.0.prompt = Some(id.to_string());
        self
    }
}

/// What a prompt handler reads: the arguments the host typed, the host's name, the date and, when the prompt reads
/// it, the graph. The extension's settings are [`crate::Settings::current`].
pub struct PromptCall<'a>(&'a PromptInput<GraphWire>);

impl PromptCall<'_> {
    /// The argument `name`, as the host typed it; `None` when the caller left it out.
    pub fn arg(&self, name: &str) -> Option<&Value> {
        self.0.args.get(name)
    }
    /// A string, enum, date or path argument, or a reference's full path.
    pub fn str(&self, name: &str) -> Option<&str> {
        self.arg(name).and_then(Value::as_str)
    }
    pub fn bool(&self, name: &str) -> Option<bool> {
        self.arg(name).and_then(Value::as_bool)
    }
    pub fn integer(&self, name: &str) -> Option<i64> {
        self.arg(name).and_then(Value::as_i64)
    }
    /// A list argument's words; empty when the caller left it out.
    pub fn list(&self, name: &str) -> Vec<&str> {
        self.arg(name)
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }
    /// The host the prompt is rendered for (`claude-code`, `codex`, `cursor`), when the host knows it.
    pub fn host(&self) -> Option<&str> {
        self.0.host.as_deref()
    }
    /// The project root.
    pub fn cwd(&self) -> &str {
        &self.0.cwd
    }
    /// The host's date, UTC, `YYYY-MM-DD`.
    pub fn today(&self) -> &str {
        &self.0.today
    }
    /// The project's graph, for a prompt that declares [`PromptBuilder::reads_graph`].
    pub fn graph(&self) -> Option<&GraphWire> {
        self.0.graph.as_ref()
    }
}
