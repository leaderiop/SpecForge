//! The engine's tools below the surfaces, one row each (ADR 0334): the MCP core tools are asserted to be these rows,
//! the CLI twins they name are declared commands, and a contribution names them by a `specforge:` class.
//!
//! Here because the registry, the operations, the host adapters, the MCP server, the CLI and the extensions all read
//! it, and this crate is the one they all reach.

/// What an engine tool does to its environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineEffect {
    /// It only reads.
    Reads,
    /// It writes output artifacts (a test report, an export), not the project's spec files.
    WritesOutput,
    /// It writes the project's files.
    Mutates,
}

/// The group an engine tool is in: what a `specforge:<group>` class expands to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineGroup {
    /// The graph reads.
    Reads,
    /// One entity or one file: where it is written, what names it, what implements it.
    Navigation,
    /// Typed changes to the spec files.
    Edits,
    /// Recorded outputs: the test report, a rendered export.
    Output,
    /// The project's setup: creation, extensions, settings, migration, health, prompts, inference sessions.
    Setup,
}

impl EngineGroup {
    /// Every group, in catalogue order.
    pub const ALL: [EngineGroup; 5] = [
        EngineGroup::Reads,
        EngineGroup::Navigation,
        EngineGroup::Edits,
        EngineGroup::Output,
        EngineGroup::Setup,
    ];

    /// The group's name in a class: `reads`, `navigation`, `edits`, `output` or `setup`.
    pub const fn as_str(self) -> &'static str {
        match self {
            EngineGroup::Reads => "reads",
            EngineGroup::Navigation => "navigation",
            EngineGroup::Edits => "edits",
            EngineGroup::Output => "output",
            EngineGroup::Setup => "setup",
        }
    }

    /// The group `name` names.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|g| g.as_str() == name)
    }

    /// Whether its tools are graph tools: every group but setup.
    pub const fn graph(self) -> bool {
        !matches!(self, EngineGroup::Setup)
    }
}

/// One engine tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineTool {
    /// Its name: the MCP tool is `specforge.<name>`.
    pub name: &'static str,
    /// What it does to its environment.
    pub effect: EngineEffect,
    /// The group it is in.
    pub group: EngineGroup,
    /// The CLI lines that do the same thing, each the words after `specforge`; empty when only MCP serves it.
    pub cli: &'static [&'static str],
    /// When to use it, in one sentence.
    pub use_when: &'static str,
}

impl EngineTool {
    /// Its MCP tool's name: `specforge.<name>`.
    pub fn mcp_name(&self) -> String {
        format!("{MCP_PREFIX}{}", self.name)
    }
}

/// What every engine tool's MCP name starts with.
const MCP_PREFIX: &str = "specforge.";

/// What a contribution's tool class starts with when it names an engine tool, group or extension command.
pub const CLASS_PREFIX: &str = "specforge:";

const fn row(
    name: &'static str,
    effect: EngineEffect,
    group: EngineGroup,
    cli: &'static [&'static str],
    use_when: &'static str,
) -> EngineTool {
    EngineTool {
        name,
        effect,
        group,
        cli,
        use_when,
    }
}

use EngineEffect::{Mutates, Reads as Read, WritesOutput};
use EngineGroup::{Edits, Navigation, Output, Reads, Setup};

/// The engine tools, in the MCP core tools' order.
pub const ENGINE_TOOLS: &[EngineTool] = &[
    row(
        "query",
        Read,
        Reads,
        &["query"],
        "Read an entity and the entities around it, at the depth and resolution you choose.",
    ),
    row(
        "validate",
        Read,
        Reads,
        &["check"],
        "Recompile the project and list its diagnostics; run it after every change.",
    ),
    row(
        "analyze",
        Read,
        Reads,
        &["analyze"],
        "Run an analysis pass (proof coverage, contract symmetry) over the compiled project.",
    ),
    row(
        "export",
        Read,
        Reads,
        &["export"],
        "Read the graph, or the part of it that fits a token budget, in one format.",
    ),
    row(
        "trace",
        Read,
        Reads,
        &["trace"],
        "Follow an entity's thread along its kind's route, the entities within one, or check an agent plan for gaps.",
    ),
    row(
        "context",
        Read,
        Reads,
        &["context"],
        "Read the context a role reads for one entity, cut to its budget.",
    ),
    row(
        "search",
        Read,
        Reads,
        &["search"],
        "Find entities by id, title or text when you do not know the id.",
    ),
    row(
        "explain",
        Read,
        Reads,
        &["explain"],
        "Read what a diagnostic code means and how to fix it.",
    ),
    row(
        "schema",
        Read,
        Reads,
        &["schema"],
        "Read the kinds, their typed fields and the edge types the loaded extensions declare.",
    ),
    row(
        "model",
        Read,
        Reads,
        &["model"],
        "Read the logical data model: kinds, fields and relationships.",
    ),
    row(
        "outline_extensions",
        Read,
        Reads,
        &["outline"],
        "See how the loaded extensions relate and what each one contributes.",
    ),
    row(
        "coverage",
        Read,
        Reads,
        &[],
        "Read each entity's coverage status.",
    ),
    row(
        "stats",
        Read,
        Reads,
        &["stats"],
        "Count the project's entities, files and diagnostics.",
    ),
    row(
        "list",
        Read,
        Reads,
        &[],
        "List entities by id, filtered by kind and field values, a page at a time.",
    ),
    row(
        "inspect",
        Read,
        Navigation,
        &[],
        "Read every field and edge of one entity.",
    ),
    row(
        "find_definition",
        Read,
        Navigation,
        &[],
        "Find where an entity is written in the spec files.",
    ),
    row(
        "find_references",
        Read,
        Navigation,
        &["refs"],
        "Find each place another entity's field names this one.",
    ),
    row(
        "outline",
        Read,
        Navigation,
        &[],
        "List the entities one spec file defines.",
    ),
    row(
        "suggest_fixes",
        Read,
        Navigation,
        &[],
        "Read the fixes offered for the project's diagnostics, each with its edits.",
    ),
    row(
        "format",
        Mutates,
        Edits,
        &["format"],
        "Format spec files in place.",
    ),
    row(
        "rename",
        Mutates,
        Edits,
        &[],
        "Rename an entity in every file that names it.",
    ),
    row(
        "edit",
        Mutates,
        Edits,
        &["new", "set", "unset", "link", "unlink", "delete"],
        "Create, change, link or delete entities as typed edits, checked and written all or nothing.",
    ),
    row(
        "init",
        Mutates,
        Setup,
        &["init"],
        "Create a SpecForge project.",
    ),
    row(
        "add_extension",
        Mutates,
        Setup,
        &["add"],
        "Install extensions, as one change.",
    ),
    row(
        "remove_extension",
        Mutates,
        Setup,
        &["remove"],
        "Remove an installed extension.",
    ),
    row(
        "migrate",
        Mutates,
        Setup,
        &["migrate"],
        "Run the migration pipeline over the spec files.",
    ),
    row(
        "extensions",
        Read,
        Setup,
        &["extensions"],
        "List the project's extensions: enabled, locked or loaded.",
    ),
    row(
        "providers",
        Read,
        Setup,
        &["providers"],
        "List the configured providers.",
    ),
    row(
        "settings",
        Read,
        Setup,
        &["settings"],
        "Read each setting: its value, where it came from and what it gates.",
    ),
    row(
        "settings_set",
        Mutates,
        Setup,
        &["settings set", "settings unset"],
        "Write or remove one setting of specforge.json.",
    ),
    row(
        "verdict_inputs",
        Read,
        Reads,
        &["query --inputs"],
        "Read what a judged rule judges for one entity and the basis its verdict records.",
    ),
    row("doctor", Read, Setup, &["doctor"], "Run the health checks."),
    row(
        "collect",
        WritesOutput,
        Output,
        &["collect"],
        "Record which entities the tests prove, from the test runner's report.",
    ),
    row(
        "render",
        WritesOutput,
        Output,
        &[],
        "Render output in a named format.",
    ),
    row(
        "infer_progress",
        Read,
        Reads,
        &["infer-status"],
        "See which source files inference has analyzed and which are stale.",
    ),
    row(
        "infer_gaps",
        Read,
        Reads,
        &["infer-status --gaps-detail"],
        "Find public source items that no entity cites and no anchor names.",
    ),
    row(
        "infer_session",
        Mutates,
        Setup,
        &[],
        "Start an inference session, mark files analyzed, or end it.",
    ),
    row(
        "find_implementation",
        Read,
        Navigation,
        &["anchors"],
        "Find the source code that implements an entity.",
    ),
    row(
        "find_spec_for_source",
        Read,
        Navigation,
        &["anchors --file"],
        "Find the entities anchored to a source file.",
    ),
    row(
        "prompt",
        Read,
        Setup,
        &["prompt"],
        "Render a prompt by name, with its arguments.",
    ),
];

/// The row named `name`.
pub fn engine_tool(name: &str) -> Option<&'static EngineTool> {
    ENGINE_TOOLS.iter().find(|t| t.name == name)
}

const fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The row whose MCP name is `mcp` (`specforge.trace`), in a `const` so a core tool is checked at compile time.
pub const fn by_mcp_name(mcp: &str) -> Option<&'static EngineTool> {
    let (mcp, prefix) = (mcp.as_bytes(), MCP_PREFIX.as_bytes());
    if mcp.len() <= prefix.len() {
        return None;
    }
    let (head, name) = mcp.split_at(prefix.len());
    if !same(head, prefix) {
        return None;
    }
    let mut i = 0;
    while i < ENGINE_TOOLS.len() {
        if same(ENGINE_TOOLS[i].name.as_bytes(), name) {
            return Some(&ENGINE_TOOLS[i]);
        }
        i += 1;
    }
    None
}

/// Whether the CLI line `line` runs `words` (the words after `specforge`): its plain words begin `words`, and each of
/// its `--flag` words is given after them, alone or as `--flag=value`.
fn runs(line: &str, words: &[&str]) -> bool {
    let (flags, plain): (Vec<&str>, Vec<&str>) = line.split(' ').partition(|w| w.starts_with("--"));
    words.len() >= plain.len()
        && plain.iter().zip(words).all(|(p, w)| p == w)
        && flags.iter().all(|flag| {
            words[plain.len()..]
                .iter()
                .any(|w| w == flag || w.strip_prefix(flag).is_some_and(|r| r.starts_with('=')))
        })
}

/// The row a CLI command line runs, `words` being the words after `specforge`: of the twins that run it, the one of
/// the most words, then the first in catalogue order (`query X --inputs gsd/E501` is `verdict_inputs`).
pub fn by_cli(words: &[&str]) -> Option<&'static EngineTool> {
    let mut best: Option<(&'static EngineTool, usize)> = None;
    for tool in ENGINE_TOOLS {
        for line in tool.cli {
            let length = line.split(' ').count();
            if runs(line, words) && best.is_none_or(|(_, longest)| length > longest) {
                best = Some((tool, length));
            }
        }
    }
    best.map(|(tool, _)| tool)
}

/// What a `specforge:` class names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineClass<'a> {
    /// One engine tool: `specforge:<tool>`.
    Tool(&'static EngineTool),
    /// Every tool of a group: `specforge:<group>`.
    Group(EngineGroup),
    /// An extension's command: `specforge:<short>.<command>`.
    Command { short: &'a str, id: &'a str },
}

/// What the tool class `class` names: a row, a group, or (text holding a `.`, split at the last one) an extension
/// command; `None` for a class without the prefix or naming none of them.
pub fn parse_class(class: &str) -> Option<EngineClass<'_>> {
    let rest = class.strip_prefix(CLASS_PREFIX)?;
    if let Some((short, id)) = rest.rsplit_once('.') {
        return (!short.is_empty() && !id.is_empty()).then_some(EngineClass::Command { short, id });
    }
    engine_tool(rest)
        .map(EngineClass::Tool)
        .or_else(|| EngineGroup::parse(rest).map(EngineClass::Group))
}

#[cfg(test)]
mod tests;
