//! Host contributions (ADR 0292): agents, skills, slash commands and hooks, and the one `render__` export that
//! writes the body of every agent and skill. Second `impl` blocks, so the files of the builders they extend stay
//! under the file-size cap.
//!
//! ```
//! # use specforge_extension_sdk::prelude::*;
//! # let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/flow", "1.0.0"));
//! c.agent("flow-planner", |a| {
//!     a.description("Plans a phase")
//!         .tools(&["read", "write", "mcp:context7/*"])
//!         .model("setting:models.planner")
//!         .variant("compact");
//! });
//! c.hook("write_guard", HookEvent::PreTool, |h| {
//!     h.description("Refuses a write that shrinks a file")
//!         .tools(&["write"])
//!         .on_failure_deny()
//!         .handler(|_call| Ok(HookOutput::allow()));
//! });
//! c.render(|call| Ok(format!("# {}\n", call.id())));
//! assert_eq!(c.declaration().surfaces.agents[0].id, "flow-planner");
//! assert_eq!(c.declaration().surfaces.hooks[0].export, "hook__write_guard");
//! ```

use specforge_protocol_types::{
    AgentDescriptor, Attribution, HookDescriptor, HookEvent, HookInput, HookOutput, OnFailure,
    RenderInput, RenderKind, RenderOutput, SkillDescriptor, SlashCommandDescriptor, ToolClass,
};

use crate::operations::Wire;
use crate::{CommandBuilder, CommandGraph, ContributionsBuilder, PlanBuilder, PromptBuilder};

/// The export that renders the body of every agent and skill.
pub const RENDER_EXPORT: &str = "render__";

/// The export that answers the hook `id`.
pub fn hook_export(id: &str) -> String {
    format!("hook__{id}")
}

fn classes(tools: &[&str]) -> Vec<ToolClass> {
    tools.iter().map(|t| t.to_string()).collect()
}

/// Builder for one agent ([`ContributionsBuilder::agent`]).
pub struct AgentBuilder(pub(crate) AgentDescriptor);

impl AgentBuilder {
    pub fn description(&mut self, description: &str) -> &mut Self {
        self.0.description = description.to_string();
        self
    }
    /// Host-neutral tool classes, or `mcp:<server>/<glob>`.
    pub fn tools(&mut self, tools: &[&str]) -> &mut Self {
        self.0.tools = classes(tools);
        self
    }
    /// `heavy`, `standard`, `light` or `inherit`, or `setting:<name>`.
    pub fn model(&mut self, model: &str) -> &mut Self {
        self.0.model = Some(model.to_string());
        self
    }
    /// `minimal`, `low`, `medium`, `high`, `max` or `inherit`, or `setting:<name>`.
    pub fn effort(&mut self, effort: &str) -> &mut Self {
        self.0.effort = Some(effort.to_string());
        self
    }
    pub fn color(&mut self, color: &str) -> &mut Self {
        self.0.color = Some(color.to_string());
        self
    }
    /// A render variant beside the default.
    pub fn variant(&mut self, variant: &str) -> &mut Self {
        self.0.variants.push(variant.to_string());
        self
    }
    /// The enum setting whose value picks the variant.
    pub fn variant_setting(&mut self, setting: &str) -> &mut Self {
        self.0.variant_setting = Some(setting.to_string());
        self
    }
    /// The agent is rendered while the bool setting `setting` is not false. Each call adds a setting; all must be on.
    pub fn when(&mut self, setting: &str) -> &mut Self {
        self.0.when.push(setting.to_string());
        self
    }
    pub fn attribution(&mut self, attribution: Attribution) -> &mut Self {
        self.0.attribution = Some(attribution);
        self
    }
}

/// Builder for one skill ([`ContributionsBuilder::skill`]).
pub struct SkillBuilder(pub(crate) SkillDescriptor);

impl SkillBuilder {
    pub fn description(&mut self, description: &str) -> &mut Self {
        self.0.description = description.to_string();
        self
    }
    pub fn tools(&mut self, tools: &[&str]) -> &mut Self {
        self.0.tools = classes(tools);
        self
    }
    pub fn argument_hint(&mut self, hint: &str) -> &mut Self {
        self.0.argument_hint = Some(hint.to_string());
        self
    }
    pub fn when(&mut self, setting: &str) -> &mut Self {
        self.0.when.push(setting.to_string());
        self
    }
    pub fn attribution(&mut self, attribution: Attribution) -> &mut Self {
        self.0.attribution = Some(attribution);
        self
    }
}

/// Builder for one slash command ([`ContributionsBuilder::slash_prompt`], [`ContributionsBuilder::slash_command`]).
pub struct SlashBuilder(pub(crate) SlashCommandDescriptor);

impl SlashBuilder {
    pub fn argument_hint(&mut self, hint: &str) -> &mut Self {
        self.0.argument_hint = Some(hint.to_string());
        self
    }
    pub fn tools(&mut self, tools: &[&str]) -> &mut Self {
        self.0.tools = classes(tools);
        self
    }
    pub fn effort(&mut self, effort: &str) -> &mut Self {
        self.0.effort = Some(effort.to_string());
        self
    }
    pub fn when(&mut self, setting: &str) -> &mut Self {
        self.0.when.push(setting.to_string());
        self
    }
    pub fn attribution(&mut self, attribution: Attribution) -> &mut Self {
        self.0.attribution = Some(attribution);
        self
    }
}

/// What a `render__` handler receives.
pub struct RenderCall<'a>(&'a RenderInput);

impl<'a> RenderCall<'a> {
    pub fn kind(&self) -> RenderKind {
        self.0.kind
    }
    /// The agent's or skill's id.
    pub fn id(&self) -> &'a str {
        &self.0.id
    }
    /// `claude-code`, `codex` or `cursor`.
    pub fn host(&self) -> &'a str {
        &self.0.host
    }
    pub fn variant(&self) -> Option<&'a str> {
        self.0.variant.as_deref()
    }
    pub fn today(&self) -> &'a str {
        &self.0.today
    }
    /// The whole input.
    pub fn input(&self) -> &'a RenderInput {
        self.0
    }
}

/// What a `hook__<id>` handler receives: the host's event and the facts it computed.
pub struct HookCall<'a>(&'a HookInput<CommandGraph>);

impl<'a> HookCall<'a> {
    /// The event, tool, path, shell, commit and state facts.
    pub fn facts(&self) -> &'a HookInput<CommandGraph> {
        self.0
    }

    /// The host's clock when the hook fired: `YYYY-MM-DDTHH:MM:SSZ`, RFC 3339 UTC with seconds (protocol 1.11.0,
    /// ADR 0294); empty when the host passed none.
    pub fn now(&self) -> &'a str {
        &self.0.now
    }

    /// The value of `extension`'s `setting` a hook that declares [`HookBuilder::reads_setting`] was sent; none when the
    /// extension is not enabled or the setting is unset (ADR 0323).
    pub fn other_setting(&self, extension: &str, setting: &str) -> Option<&'a serde_json::Value> {
        self.0.other_settings.get(&format!("{extension}:{setting}"))
    }

    /// A builder for the plan a hook that declares `writes` answers.
    pub fn plan(&self) -> PlanBuilder {
        PlanBuilder::for_hook(self.0)
    }
}

/// Builder for one hook ([`ContributionsBuilder::hook`]).
pub struct HookBuilder {
    pub(crate) descriptor: HookDescriptor,
    pub(crate) wire: Option<Wire>,
}

impl HookBuilder {
    pub fn description(&mut self, description: &str) -> &mut Self {
        self.descriptor.description = description.to_string();
        self
    }
    /// The tool classes a `pre_tool`, `post_tool` or `pre_commit` hook fires on.
    pub fn tools(&mut self, tools: &[&str]) -> &mut Self {
        self.descriptor.tools = classes(tools);
        self
    }
    /// Project-relative globs a path of the tool's input must match.
    pub fn paths(&mut self, paths: &[&str]) -> &mut Self {
        self.descriptor.paths = paths.iter().map(|p| p.to_string()).collect();
        self
    }
    pub fn timeout_ms(&mut self, timeout_ms: u32) -> &mut Self {
        self.descriptor.timeout_ms = timeout_ms;
        self
    }
    /// A trap, a timeout or a malformed answer denies the event.
    pub fn on_failure_deny(&mut self) -> &mut Self {
        self.descriptor.on_failure = OnFailure::Deny;
        self
    }
    pub fn reads_graph(&mut self) -> &mut Self {
        self.descriptor.reads_graph = true;
        self
    }
    pub fn reads_state(&mut self) -> &mut Self {
        self.descriptor.reads_state = true;
        self
    }
    /// The hook may answer an edit plan.
    pub fn writes(&mut self) -> &mut Self {
        self.descriptor.writes = true;
        self
    }
    pub fn when(&mut self, setting: &str) -> &mut Self {
        self.descriptor.when.push(setting.to_string());
        self
    }
    /// Install the hook on these hosts only (`claude-code`, `codex`, `cursor`); none: every host (ADR 0322).
    pub fn hosts(&mut self, hosts: &[&str]) -> &mut Self {
        self.descriptor.hosts = hosts.iter().map(|h| h.to_string()).collect();
        self
    }
    /// The input carries the repository the `cwd` is in, and an outside path's repository (ADR 0320).
    pub fn reads_git(&mut self) -> &mut Self {
        self.descriptor.reads_git = true;
        self
    }
    /// [`Self::reads_git`], and `git status --porcelain=v2 --branch` too.
    pub fn reads_git_status(&mut self) -> &mut Self {
        self.descriptor.reads_git = true;
        self.descriptor.reads_git_status = true;
        self
    }
    /// The input carries the value of another extension's setting (ADR 0323).
    pub fn reads_setting(&mut self, extension: &str, setting: &str) -> &mut Self {
        self.descriptor
            .reads_settings
            .push(format!("{extension}:{setting}"));
        self
    }
    /// The function that answers the event.
    pub fn handler(
        &mut self,
        handler: impl Fn(&HookCall<'_>) -> Result<HookOutput, String> + 'static,
    ) -> &mut Self {
        self.wire = Some(crate::operations::wire(
            "hook",
            move |input: &HookInput<CommandGraph>| handler(&HookCall(input)),
        ));
        self
    }
}

impl ContributionsBuilder {
    /// Contribute an agent; its body is the `render__` handler's.
    ///
    /// # Panics
    ///
    /// When an agent or skill of that id is already declared.
    pub fn agent(&mut self, id: &str, f: impl FnOnce(&mut AgentBuilder)) -> &mut Self {
        self.assert_new_name(id);
        let mut b = AgentBuilder(AgentDescriptor {
            id: id.to_string(),
            ..Default::default()
        });
        f(&mut b);
        self.agents.push(b.0);
        self
    }

    /// Contribute a skill; its body is the `render__` handler's.
    ///
    /// # Panics
    ///
    /// When an agent or skill of that id is already declared.
    pub fn skill(&mut self, id: &str, f: impl FnOnce(&mut SkillBuilder)) -> &mut Self {
        self.assert_new_name(id);
        let mut b = SkillBuilder(SkillDescriptor {
            id: id.to_string(),
            ..Default::default()
        });
        f(&mut b);
        self.skills.push(b.0);
        self
    }

    fn assert_new_name(&self, id: &str) {
        assert!(
            !self.agents.iter().any(|a| a.id == id) && !self.skills.iter().any(|s| s.id == id),
            "agent or skill '{id}' is declared twice"
        );
    }

    /// Bind the prompt `prompt` of this extension to a slash command of each host.
    pub fn slash_prompt(&mut self, prompt: &str, f: impl FnOnce(&mut SlashBuilder)) -> &mut Self {
        let mut b = SlashBuilder(SlashCommandDescriptor {
            prompt: Some(prompt.to_string()),
            ..Default::default()
        });
        f(&mut b);
        self.slash_commands.push(b.0);
        self
    }

    /// Bind the command `command` of this extension to a slash command of each host.
    pub fn slash_command(&mut self, command: &str, f: impl FnOnce(&mut SlashBuilder)) -> &mut Self {
        let mut b = SlashBuilder(SlashCommandDescriptor {
            command: Some(command.to_string()),
            ..Default::default()
        });
        f(&mut b);
        self.slash_commands.push(b.0);
        self
    }

    /// Contribute a hook on a host `event`, answered by its handler at the export `hook__<id>`.
    ///
    /// # Panics
    ///
    /// When the hook declares no handler, or a hook of that id is already declared.
    pub fn hook(
        &mut self,
        id: &str,
        event: HookEvent,
        f: impl FnOnce(&mut HookBuilder),
    ) -> &mut Self {
        assert!(
            !self.hooks.iter().any(|h| h.id == id),
            "hook '{id}' is declared twice"
        );
        let mut b = HookBuilder {
            descriptor: HookDescriptor {
                id: id.to_string(),
                event,
                export: hook_export(id),
                timeout_ms: 5_000,
                ..Default::default()
            },
            wire: None,
        };
        f(&mut b);
        let Some(wire) = b.wire else {
            panic!("hook '{id}' declares no handler: answer it with `h.handler(...)`");
        };
        self.add_operation(hook_export(id), format!("hook '{id}'"), wire);
        self.hooks.push(b.descriptor);
        self
    }

    /// The function that writes the body of every agent and skill declared, at the export `render__`.
    ///
    /// # Panics
    ///
    /// When called twice.
    pub fn render(
        &mut self,
        handler: impl Fn(&RenderCall<'_>) -> Result<String, String> + 'static,
    ) -> &mut Self {
        let wire = crate::operations::wire("render", move |input: &RenderInput| {
            handler(&RenderCall(input)).map(|body| RenderOutput { body })
        });
        self.add_operation(RENDER_EXPORT.to_string(), "the renderer".to_string(), wire);
        self
    }
}

impl CommandBuilder {
    /// The command reads its extension's state: the host sends it as `state`.
    pub fn reads_state(&mut self) -> &mut Self {
        self.descriptor.reads_state = true;
        self
    }
}

impl PromptBuilder {
    /// The prompt reads its extension's state: the host sends it as `state`.
    pub fn reads_state(&mut self) -> &mut Self {
        self.descriptor.reads_state = true;
        self
    }
}
