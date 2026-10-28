//! What an extension contributes to a host, and what the host sends the exports that serve it (ADR 0292, protocol
//! 1.10.0): agents, skills, slash commands and hooks in an extension's surfaces, the `render__` export that writes
//! an agent's or skill's body, the `hook__<id>` exports, and the state a hook or command keeps.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Attribution, EditPlan, GraphWire, Revision};

/// A host-neutral tool class, mapped per host by its adapter: `read`, `write`, `edit`, `shell`, `search`,
/// `web_fetch`, `web_search`, `ask_user`, `agent`, `skill`, `todo`, or `mcp:<server>/<glob>`.
pub type ToolClass = String;

/// An agent an extension contributes: a host file named `id`, its body written by the `render__` export.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentDescriptor {
    /// kebab-case: the host file's name.
    pub id: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolClass>,
    /// `heavy`, `standard`, `light` or `inherit`, or `setting:<name>` whose value is one of them (or, for Codex, a
    /// model id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// `minimal`, `low`, `medium`, `high`, `max` or `inherit`, or `setting:<name>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Render variants beside the default (`compact`), chosen by the value of `variant_setting`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_setting: Option<String>,
    /// A bool setting of the extension: while it is false the agent is not rendered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<Attribution>,
}

/// A skill an extension contributes: a host file named `id`, its body written by the `render__` export.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillDescriptor {
    pub id: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<Attribution>,
}

/// The host binding of a prompt or a command, named `<short>-<route>` and rendered as a trampoline to the
/// target's own surface. Exactly one of `prompt` and `command` names the target.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlashCommandDescriptor {
    /// A prompt id of the extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// A command id of the extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Absent: derived from the target's arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<Attribution>,
}

/// A hook an extension contributes: a host event wired to `specforge hook <short>.<id>`, answered by `export`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookDescriptor {
    /// snake_case; routed as `<short>.<id>`.
    pub id: String,
    pub description: String,
    pub event: HookEvent,
    /// `pre_tool`, `post_tool` and `pre_commit` only: the classes it fires on (empty: every tool).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolClass>,
    /// Project-relative globs a path of the tool's input must match (empty: any); `{spec_root}/**` names the spec
    /// root.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// The call's budget, at most the sandbox ceiling.
    #[serde(default = "five_seconds", skip_serializing_if = "is_five_seconds")]
    pub timeout_ms: u32,
    /// What a trap, a timeout or a malformed answer does.
    #[serde(default, skip_serializing_if = "OnFailure::is_allow")]
    pub on_failure: OnFailure,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reads_graph: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reads_state: bool,
    /// The hook may answer an edit plan (the host applies it with `Origin::Hook`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub writes: bool,
    /// A bool setting of the extension: while it is false the hook is a no-op.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// `hook__<id>`.
    pub export: String,
}

fn five_seconds() -> u32 {
    5_000
}

fn is_five_seconds(ms: &u32) -> bool {
    *ms == 5_000
}

/// The host events a hook can ask for.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    #[default]
    PreTool,
    PostTool,
    PreCommit,
    UserPrompt,
    SessionStart,
    Stop,
    SubagentStart,
    SubagentStop,
    PreCompact,
    Statusline,
}

impl HookEvent {
    /// Every event.
    pub const ALL: [HookEvent; 10] = [
        HookEvent::PreTool,
        HookEvent::PostTool,
        HookEvent::PreCommit,
        HookEvent::UserPrompt,
        HookEvent::SessionStart,
        HookEvent::Stop,
        HookEvent::SubagentStart,
        HookEvent::SubagentStop,
        HookEvent::PreCompact,
        HookEvent::Statusline,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            HookEvent::PreTool => "pre_tool",
            HookEvent::PostTool => "post_tool",
            HookEvent::PreCommit => "pre_commit",
            HookEvent::UserPrompt => "user_prompt",
            HookEvent::SessionStart => "session_start",
            HookEvent::Stop => "stop",
            HookEvent::SubagentStart => "subagent_start",
            HookEvent::SubagentStop => "subagent_stop",
            HookEvent::PreCompact => "pre_compact",
            HookEvent::Statusline => "statusline",
        }
    }

    /// The event named `name` on the wire.
    pub fn parse(name: &str) -> Option<HookEvent> {
        HookEvent::ALL.into_iter().find(|e| e.as_str() == name)
    }

    /// Whether a hook of this event filters on tool classes and paths.
    pub const fn takes_tools(self) -> bool {
        matches!(
            self,
            HookEvent::PreTool | HookEvent::PostTool | HookEvent::PreCommit
        )
    }

    /// Whether a hook of this event may answer an edit plan.
    pub const fn can_write(self) -> bool {
        !matches!(self, HookEvent::Statusline)
    }
}

/// What a hook does when it traps, times out or answers what is not a [`HookOutput`].
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnFailure {
    #[default]
    Allow,
    Deny,
}

impl OnFailure {
    fn is_allow(&self) -> bool {
        *self == OnFailure::Allow
    }
}

/// What a `render__` export writes the body of.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RenderKind {
    #[default]
    Agent,
    Skill,
}

/// What the host sends the `render__` export: install calls it once per agent and skill and host.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderInput {
    pub kind: RenderKind,
    pub id: String,
    /// `claude-code`, `codex` or `cursor`.
    pub host: String,
    /// One of the agent's `variants`, chosen by its `variant_setting`; absent, the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub today: String,
}

/// The body the `render__` export answers (without the host's frontmatter and marker).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderOutput {
    pub body: String,
}

/// What the host sends a `hook__<id>` export: the host's event, and the facts a sandboxed guest cannot read.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HookInput<G = GraphWire> {
    pub event: HookEvent,
    pub host: String,
    pub cwd: String,
    pub today: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<HookTool>,
    /// Every path the tool's input names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<PathFact>,
    /// A shell tool's command, split into words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<Vec<String>>,
    /// `pre_commit`: the commit being made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<CommitFact>,
    /// `user_prompt`: what the user typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// `post_tool`: the host's tool response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    /// `statusline`: the host's status JSON (model, context window).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Value>,
    /// A hook that declares `reads_graph`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<G>,
    /// A hook that declares `writes`: the revision of every source of the compile, as a writing command's.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub revisions: BTreeMap<String, Revision>,
    /// A hook that declares `reads_state`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<StateView>,
}

/// The tool a hook fires on, as the host named it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HookTool {
    pub class: ToolClass,
    /// The host's name for the tool (`Write`, `Bash`).
    pub name: String,
    /// The host's tool input.
    #[serde(default)]
    pub input: Value,
}

/// What the host computed about a path in a tool's input.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathFact {
    /// Project-relative when `in_project`; else as the tool named it.
    pub path: String,
    pub in_project: bool,
    pub in_spec_root: bool,
    pub exists: bool,
    pub bytes: Option<u64>,
    /// A write's content size.
    pub new_bytes: Option<u64>,
}

/// The commit a `pre_commit` hook fires on.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitFact {
    pub message: String,
    pub amend: bool,
}

/// What a hook or command reads of its extension's state (ADR 0292).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateView {
    #[serde(default)]
    pub project: Map<String, Value>,
    /// The host's session; none for a CLI command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Map<String, Value>>,
}

/// What a hook or command asks to change in its state: JSON merge patches (RFC 7396).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Value>,
}

/// A hook's verdict on the event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
    Ask,
}

/// What a `hook__<id>` export answers. Every member is optional: `{}` is a no-op.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HookOutput {
    /// `pre_tool`, `pre_commit`, `user_prompt` and `stop`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Text for the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Text for the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statusline: Option<String>,
    /// A hook that declares `writes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<EditPlan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<StatePatch>,
}

impl HookOutput {
    /// Let the event through.
    pub fn allow() -> Self {
        HookOutput {
            decision: Some(Decision::Allow),
            ..Default::default()
        }
    }

    /// Refuse the event, saying why.
    pub fn deny(reason: impl Into<String>) -> Self {
        HookOutput {
            decision: Some(Decision::Deny),
            reason: Some(reason.into()),
            ..Default::default()
        }
    }

    /// Ask the user, saying why.
    pub fn ask(reason: impl Into<String>) -> Self {
        HookOutput {
            decision: Some(Decision::Ask),
            reason: Some(reason.into()),
            ..Default::default()
        }
    }

    /// This output also giving the model `context`.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// This output also telling the user `notice`.
    pub fn with_notice(mut self, notice: impl Into<String>) -> Self {
        self.notice = Some(notice.into());
        self
    }

    /// This output carrying the edit `plan`.
    pub fn with_plan(mut self, plan: EditPlan) -> Self {
        self.plan = Some(plan);
        self
    }

    /// This output patching the state.
    pub fn with_state(mut self, state: StatePatch) -> Self {
        self.state = Some(state);
        self
    }
}
