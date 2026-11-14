//! Tool classes and a host's tool names (ADR 0292).

use specforge_protocol_types::ToolClass;
use specforge_protocol_types::engine_tools::{EngineClass, parse_class};

/// The name install gives SpecForge's own MCP server in a host's config.
pub const MCP_SERVER: &str = "specforge";

/// Claude Code's name for SpecForge's MCP tool `mcp` (`specforge.trace`): `mcp__specforge__` and the name with every
/// character outside `[A-Za-z0-9_-]` written `_` (ADR 0334 D5).
pub fn claude_mcp_tool(mcp: &str) -> String {
    let name: String = mcp
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("mcp__{MCP_SERVER}__{name}")
}

/// Claude Code's tool names for `class`; `mcp:<server>/<glob>` is `mcp__<server>__<glob>`, and a `specforge:` class
/// is the one MCP tool it names (a group names none: the operations expanded it).
pub fn claude_names(class: &str) -> Vec<String> {
    let names: &[&str] = match class {
        "read" => &["Read"],
        "write" => &["Write"],
        "edit" => &["Edit", "MultiEdit"],
        "shell" => &["Bash"],
        "search" => &["Glob", "Grep"],
        "web_fetch" => &["WebFetch"],
        "web_search" => &["WebSearch"],
        "ask_user" => &["AskUserQuestion"],
        "agent" => &["Agent"],
        "skill" => &["Skill"],
        "todo" => &["TodoWrite"],
        _ => &[],
    };
    if !names.is_empty() {
        return names.iter().map(|n| (*n).to_string()).collect();
    }
    match parse_class(class) {
        Some(EngineClass::Tool(tool)) => return vec![claude_mcp_tool(&tool.mcp_name())],
        Some(EngineClass::Command { short, id }) => {
            return vec![claude_mcp_tool(&format!("specforge.{short}.{id}"))];
        }
        Some(EngineClass::Group(_)) => return Vec::new(),
        None => {}
    }
    match class.strip_prefix("mcp:").and_then(|r| r.split_once('/')) {
        Some((server, glob)) => vec![format!("mcp__{server}__{glob}")],
        None => Vec::new(),
    }
}

/// The class of a tool Claude Code names; empty for a tool no class covers.
pub fn claude_class(name: &str) -> ToolClass {
    match name {
        "Read" => "read".into(),
        "Write" => "write".into(),
        "Edit" | "MultiEdit" | "NotebookEdit" => "edit".into(),
        "Bash" => "shell".into(),
        "Glob" | "Grep" => "search".into(),
        "WebFetch" => "web_fetch".into(),
        "WebSearch" => "web_search".into(),
        "AskUserQuestion" => "ask_user".into(),
        "Agent" | "Task" => "agent".into(),
        "Skill" => "skill".into(),
        "TodoWrite" => "todo".into(),
        _ => name
            .strip_prefix("mcp__")
            .and_then(|r| r.split_once("__"))
            .map(|(server, tool)| format!("mcp:{server}/{tool}"))
            .unwrap_or_default(),
    }
}

/// `names`, each once, in order.
pub fn unique(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in names {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// Codex's tool names for `class`: it has a shell, a patch tool and MCP; no other class names a tool.
pub fn codex_names(class: &str) -> Vec<String> {
    match class {
        "write" | "edit" => vec!["apply_patch".into(), "Edit".into(), "Write".into()],
        "shell" => vec!["Bash".into()],
        _ => class
            .strip_prefix("mcp:")
            .and_then(|r| r.split_once('/'))
            .map(|(server, glob)| format!("mcp__{server}__{glob}"))
            .into_iter()
            .collect(),
    }
}

/// The class of a tool Codex names; empty for a tool no class covers.
pub fn codex_class(name: &str) -> ToolClass {
    match name {
        "Bash" | "shell" | "local_shell" => "shell".into(),
        "apply_patch" | "Edit" => "edit".into(),
        "Write" => "write".into(),
        _ => name
            .strip_prefix("mcp__")
            .and_then(|r| r.split_once("__"))
            .map(|(server, tool)| format!("mcp:{server}/{tool}"))
            .unwrap_or_default(),
    }
}

/// The class of a tool Cursor names; empty for a tool no class covers.
pub fn cursor_class(name: &str) -> ToolClass {
    match name {
        "Shell" | "Bash" => "shell".into(),
        "Read" => "read".into(),
        "Write" => "write".into(),
        "Edit" | "StrReplace" | "Delete" | "ApplyPatch" => "edit".into(),
        "Grep" | "Glob" | "SemanticSearch" => "search".into(),
        "Task" => "agent".into(),
        _ => name
            .strip_prefix("MCP:")
            .and_then(|r| r.split_once(':'))
            .map(|(server, tool)| format!("mcp:{server}/{tool}"))
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "Claude Code names an engine tool class as it names an MCP tool of the specforge server"
    )]
    fn an_engine_class_is_named_as_claude_code_names_an_mcp_tool() {
        assert_eq!(
            claude_names("specforge:trace"),
            ["mcp__specforge__specforge_trace"]
        );
        assert_eq!(
            claude_names("specforge:t.stats"),
            ["mcp__specforge__specforge_t_stats"]
        );
        assert!(claude_names("specforge:reads").is_empty());
        assert!(claude_names("specforge:nope").is_empty());
        assert_eq!(
            claude_mcp_tool("specforge.a-b.c d"),
            "mcp__specforge__specforge_a-b_c_d"
        );
    }
}
