//! The host port (ADR 0292): what an adapter that knows one host's file formats and hook protocol gives the
//! operations, and the neutral values that cross it. The operations know no format and no GSD word; the adapters
//! (`specforge-hosts`) know no contribution's meaning.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specforge_protocol_types::{Attribution, Decision, HookEvent, ToolClass};

use super::Host;

/// Where a rendered file comes from: the extension, its version, what kind of contribution, and its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub extension: String,
    pub version: String,
    /// `agent`, `skill` or `slash`.
    pub kind: &'static str,
    pub id: String,
}

/// The model an agent asks for, once its `setting:` is read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ModelChoice {
    /// None declared, or `inherit`: the host's own.
    #[default]
    Inherit,
    /// `heavy`, `standard` or `light`: a tier each host maps to its own model, or to none.
    Tier(&'static str),
    /// Any other word: a model id of the host the extension names.
    Id(String),
}

/// One agent to render.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedAgent {
    pub source: Source,
    pub id: String,
    pub description: String,
    pub tools: Vec<ToolClass>,
    pub model: ModelChoice,
    /// `minimal` … `max`; absent for `inherit`.
    pub effort: Option<String>,
    pub color: Option<String>,
    /// What the extension's `render__` answered (and the judged-rules section appended after it).
    pub body: String,
    pub attribution: Option<Attribution>,
}

/// One skill to render.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedSkill {
    pub source: Source,
    pub id: String,
    pub description: String,
    pub tools: Vec<ToolClass>,
    pub argument_hint: Option<String>,
    pub body: String,
    pub attribution: Option<Attribution>,
}

/// What a slash command calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCall {
    /// A prompt: `specforge.prompt` with the MCP name, or `specforge prompt <short>.<id>`.
    Prompt { mcp_name: String, cli: String },
    /// A command: its MCP tool, or `specforge <short> <name>`.
    Command { mcp_tool: String, cli: String },
}

/// One slash command to render: a trampoline to a prompt or a command.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedSlash {
    pub source: Source,
    /// `<short>-<route>`.
    pub name: String,
    pub description: String,
    pub argument_hint: Option<String>,
    pub tools: Vec<ToolClass>,
    pub effort: Option<String>,
    pub call: SlashCall,
    pub attribution: Option<Attribution>,
}

/// One hook to wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedHook {
    /// `<short>.<id>`; `core.spec_guard` for the host's own.
    pub route: String,
    pub description: String,
    pub event: HookEvent,
    pub tools: Vec<ToolClass>,
    /// Project-relative globs, `{spec_root}` already the spec root.
    pub paths: Vec<String>,
    pub timeout_ms: u32,
}

/// The shell comment that ends every hook command install writes: it marks the entry as install's, so a user's own
/// hook, even one that also starts `specforge hook `, is never taken for it.
pub const HOOK_MARK: &str = "# specforge-install";

impl RenderedHook {
    /// The command a host runs: `specforge hook <route> --host <host> --event <event> [--tools …] [--paths …]`.
    pub fn command(&self, host: Host) -> String {
        let mut words = vec![
            "specforge".to_string(),
            "hook".to_string(),
            self.route.clone(),
            "--host".to_string(),
            host.as_str().to_string(),
            "--event".to_string(),
            self.event.as_str().to_string(),
        ];
        if !self.tools.is_empty() {
            words.extend(["--tools".to_string(), self.tools.join(",")]);
        }
        if !self.paths.is_empty() {
            // One flag per glob: a glob may hold a comma (`{a,b}`), which no list separator may.
            for path in &self.paths {
                words.extend(["--paths".to_string(), quoted(path)]);
            }
        }
        words.push(HOOK_MARK.to_string());
        words.join(" ")
    }

    /// The seconds a host that takes seconds is told to wait: the call's budget and five more for the process, so the
    /// host never kills a hook before it answers by its own failure policy.
    pub fn host_timeout_seconds(&self) -> u32 {
        (self.timeout_ms + 5_000).div_ceil(1_000)
    }
}

/// `word` as one shell word: a glob or a space in it must reach the process as written.
// shortcut: POSIX quoting only; the hosts run the command under `sh`, a Windows shell is not quoted for.
fn quoted(word: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "._/,:=@%+-".contains(c);
    if word.chars().all(plain) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

/// Everything one host is given to render, neutral.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub host: Host,
    pub agents: Vec<RenderedAgent>,
    pub skills: Vec<RenderedSkill>,
    pub slashes: Vec<RenderedSlash>,
    pub hooks: Vec<RenderedHook>,
}

/// A file the adapter owns outright: its text carries the marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedFile {
    /// Project-relative, `/`-separated.
    pub path: String,
    pub text: String,
}

/// One entry the adapter merges into a config file it does not own whole: `value` set as `name` in the object at
/// `keys`, or, with no `name`, pushed on the array at `keys`.
#[derive(Debug, Clone, PartialEq)]
pub struct MergedEntry {
    pub keys: Vec<String>,
    pub name: Option<String>,
    pub value: Value,
}

/// The entries of one config file.
#[derive(Debug, Clone, PartialEq)]
pub struct MergedFile {
    pub path: String,
    pub entries: Vec<MergedEntry>,
}

/// A contribution the host cannot take, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotRendered {
    pub what: String,
    pub why: String,
}

impl std::fmt::Display for NotRendered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.what, self.why)
    }
}

/// What one host gets.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostTree {
    pub owned: Vec<OwnedFile>,
    pub merged: Vec<MergedFile>,
    pub not_rendered: Vec<NotRendered>,
}

/// A config file after merging: its text, or `None` when nothing is left of it; and what it could not take.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Merged {
    pub text: Option<String>,
    pub not_rendered: Vec<NotRendered>,
}

/// The tool a hook fires on, as the host named it and as a class.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolUse {
    pub name: String,
    #[serde(default)]
    pub class: ToolClass,
    #[serde(default)]
    pub input: Value,
}

/// A host's hook event, neutral: what every adapter reads its stdin into.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HostHookEvent {
    pub event: HookEvent,
    #[serde(default)]
    pub host: String,
    /// The project directory the host ran in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolUse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Value>,
}

/// A hook's answer, neutral: what every adapter writes as its host expects.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookAnswer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statusline: Option<String>,
}

/// What a hook process writes for its host.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostReply {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Why a host's stdin is no event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookReadError(pub String);

impl std::fmt::Display for HookReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One host's layout and hook protocol (ADR 0292). Implemented in `specforge-hosts`.
pub trait HostRenderer: Send + Sync {
    fn host(&self) -> Host;

    /// The files and entries this host gets for `rendered`, or why one is not rendered.
    fn render(&self, rendered: &Rendered) -> HostTree;

    /// The directories under which this host's owned files live, project-relative: install looks there for what it
    /// marked and no extension contributes any more.
    fn owned_roots(&self) -> Vec<String>;

    /// Every config file this host merges entries into, project-relative: uninstall strips them all.
    fn merged_paths(&self) -> Vec<String>;

    /// `existing` (the file, if there is one) with every entry it owns replaced by `entries` (none: removed), and
    /// every other key untouched. `text: None` when nothing is left of the file. Err: `existing` is no document of
    /// the format.
    fn merge(
        &self,
        path: &str,
        existing: Option<&str>,
        entries: &[MergedEntry],
    ) -> Result<Merged, String>;

    /// This host's name for `event`; `None`: the host has no such event.
    fn event_name(&self, event: HookEvent) -> Option<&'static str>;

    /// Read the host's hook stdin into the neutral event.
    fn read_event(&self, stdin: &[u8]) -> Result<HostHookEvent, HookReadError>;

    /// The neutral `answer` to `event`, as the host expects it.
    fn write_answer(&self, event: &HostHookEvent, answer: &HookAnswer) -> HostReply;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "a hook command repeats --paths per glob, so a glob holding a comma reaches the hook whole"
    )]
    fn a_hook_command_passes_each_glob_whole() {
        let hook = RenderedHook {
            route: "t.g".into(),
            description: String::new(),
            event: HookEvent::PreTool,
            tools: Vec::new(),
            paths: vec!["specs/**/*.{spec,md}".into(), "a b/*.spec".into()],
            timeout_ms: 1000,
        };
        assert_eq!(
            hook.command(Host::ClaudeCode),
            concat!(
                "specforge hook t.g --host claude-code --event pre_tool ",
                "--paths 'specs/**/*.{spec,md}' --paths 'a b/*.spec' # specforge-install"
            )
        );
    }
}
