//! Tool classes and a host's tool names (ADR 0292).

use specforge_protocol_types::ToolClass;

/// Claude Code's tool names for `class`; `mcp:<server>/<glob>` is `mcp__<server>__<glob>`.
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
