//! What a text names from the engine-tool catalogue (ADR 0334): the one dictionary of the port's tool measures. An MCP
//! name (`specforge.trace`) names its tool anywhere, prose included; a CLI line names one only as a command in code,
//! a shell fence's line or an inline code span, read by the SDK shell reader and matched by the catalogue's `by_cli`.
//! Bare prose ("run specforge check") is not a call and names nothing.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use std::collections::BTreeSet;
use std::sync::OnceLock;

use specforge_extension_sdk::SettingsItem;
use specforge_extension_sdk::engine_tools::{ENGINE_TOOLS, EngineTool, by_cli, by_mcp_name};
use specforge_extension_sdk::shell;

/// The fence languages read as shell.
const SHELL: &[&str] = &["bash", "sh", "shell", "zsh"];

/// The CLI verbs no engine tool twins: the ports' (files, git, http, run, wait) and the host's own (install, extension
/// updates, grants, hooks, the harness).
pub const PORT_AND_HOST_VERBS: &[&str] = &[
    "files",
    "git",
    "http",
    "run",
    "wait",
    "install",
    "uninstall",
    "update",
    "grants",
    "hook",
    "harness",
];

/// Commands of `@gsd/security`, whose short name is also `gsd`, that core's text asks for when it is loaded
/// (`security-hooks` for the security gate). Core's tests do not build its declaration.
pub const SECURITY_COMMANDS: &[&str] = &["security-gate", "security-hooks", "threat-ids"];

/// `@gsd/core`'s setting keys, and the ids of the commands, MCP tools and prompts served as `specforge.gsd.<id>`.
fn declared() -> &'static (BTreeSet<String>, BTreeSet<String>) {
    static DECLARED: OnceLock<(BTreeSet<String>, BTreeSet<String>)> = OnceLock::new();
    DECLARED.get_or_init(|| {
        let declaration = gsd_core::build().declaration();
        let keys = declaration
            .settings
            .iter()
            .filter_map(|item| match item {
                SettingsItem::Setting(s) => Some(s.name.clone()),
                _ => None,
            })
            .collect();
        let surfaces = &declaration.surfaces;
        let ids = surfaces
            .commands
            .iter()
            .map(|c| c.id.clone())
            .chain(surfaces.mcp_tools.iter().map(|t| t.name.clone()))
            .chain(surfaces.prompts.iter().map(|p| p.id.clone()))
            .chain(SECURITY_COMMANDS.iter().map(|c| c.replace('-', "_")))
            .collect();
        (keys, ids)
    })
}

/// Whether `words`, the words after `specforge`, run a command the CLI has: an engine tool's CLI twin of any group, a
/// port or host verb, or `gsd` and a command `@gsd/core` or `@gsd/security` declares.
pub fn is_cli_verb(words: &[&str]) -> bool {
    match words {
        ["gsd", command, ..] => {
            SECURITY_COMMANDS.contains(command) || declared().1.contains(&command.replace('-', "_"))
        }
        [first, ..] if PORT_AND_HOST_VERBS.contains(first) => true,
        _ => by_cli(words).is_some(),
    }
}

/// Whether `name`, a `specforge.<x>` word, is one the engine knows: the manifest `specforge.json`, a setting key
/// `@gsd/core` declares, an extension tool `specforge.gsd.<id>` it serves, or an engine tool of the catalogue.
pub fn mcp_name_is_known(name: &str) -> bool {
    let Some(x) = name.strip_prefix("specforge.") else {
        return false;
    };
    x == "json"
        || declared().0.contains(name)
        || x.strip_prefix("gsd.")
            .is_some_and(|id| declared().1.contains(id))
        || by_mcp_name(name).is_some()
}

/// Whether `name`, an `mcp__specforge__<n>` word, is Claude Code's name of a known MCP tool: each engine tool and
/// extension tool mapped forward by the host's rule (a character outside `[A-Za-z0-9_-]` is `_`), never parsed back.
pub fn claude_name_is_known(name: &str) -> bool {
    let claude = |mcp: &str| -> String {
        let n: String = mcp
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("mcp__specforge__{n}")
    };
    ENGINE_TOOLS
        .iter()
        .map(EngineTool::mcp_name)
        .chain(declared().1.iter().map(|id| format!("specforge.gsd.{id}")))
        .any(|mcp| claude(&mcp) == name)
}

/// The distinct catalogue rows `text` names, in catalogue order.
pub fn named(text: &str) -> Vec<&'static EngineTool> {
    let mut found: Vec<&'static EngineTool> = ENGINE_TOOLS
        .iter()
        .filter(|tool| names_mcp(text, &tool.mcp_name()))
        .collect();
    for words in code_commands(text) {
        if let Some(tool) = words
            .split_first()
            .filter(|(first, _)| *first == "specforge")
            .and_then(|(_, rest)| by_cli(&rest.iter().map(String::as_str).collect::<Vec<_>>()))
            && !found.iter().any(|t| t.name == tool.name)
        {
            found.push(tool);
        }
    }
    let at = |tool: &EngineTool| ENGINE_TOOLS.iter().position(|t| t.name == tool.name);
    found.sort_by_key(|tool| at(tool));
    found
}

/// Whether `mcp` stands in `text` as a whole name: no word character before it, and after it neither a word character
/// nor a `.` that goes on into one (`specforge.json` is not `specforge.j`, `specforge.edit.` ends a sentence).
fn names_mcp(text: &str, mcp: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(mcp).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let mut after = text[at + mcp.len()..].chars();
        let next = after.next();
        !before.is_some_and(word)
            && !next.is_some_and(word)
            && !(next == Some('.') && after.next().is_some_and(word))
    })
}

/// Every command of `text`'s code, each its words: each line of a shell fence (a `\`-continued line joined to the next,
/// a `#` comment left out) and each inline code span, through the SDK shell reader, command substitutions included.
/// shortcut: a line inside a multi-line quote or a heredoc body does not parse alone and is skipped; read the whole
/// fence once the SDK reader skips comments and heredoc bodies.
pub fn code_commands(text: &str) -> Vec<Vec<String>> {
    let fenced = gsd_kit::markdown::fences(text)
        .into_iter()
        .filter(|f| SHELL.contains(&f.language.as_str()))
        .flat_map(|f| shell_lines(&f.text));
    let inline = gsd_kit::markdown::inline_code(text)
        .into_iter()
        .map(|(_, code)| code);
    fenced
        .chain(inline)
        .filter_map(|line| shell::commands(&line).ok())
        .flatten()
        .collect()
}

/// The command lines of a shell fence's code: a line ending in `\` joined to the next, `#` comments and blanks left out.
pub fn shell_lines(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut held = String::new();
    for line in code.lines() {
        match line.strip_suffix('\\') {
            Some(head) => {
                held.push_str(head);
                held.push(' ');
            }
            None => {
                held.push_str(line);
                let whole = std::mem::take(&mut held);
                let t = whole.trim();
                if !t.is_empty() && !t.starts_with('#') {
                    out.push(whole);
                }
            }
        }
    }
    if !held.trim().is_empty() {
        out.push(held);
    }
    out
}
