//! The slash command of every prompt keeps the metadata of the upstream command file it is ported from (D-r42-11):
//! description verbatim, tools mapped by class, effort, and the argument hint (which `ADAPTED` lists where a typed
//! argument replaced upstream's free text).

use std::path::Path;

/// `(command stem, our hint)`: upstream's hint says `[phase]` or takes free text where the prompt has typed arguments.
const ADAPTED: &[(&str, &str)] = &[
    ("extract-learnings", "<phase>"),
    ("milestone-summary", "[--milestone <version>]"),
    ("pr-branch", "[--target <branch>]"),
    (
        "review",
        "<phase> [--claude] [--codex] [--opencode] [--qwen] [--cursor] [--agy] [--all]",
    ),
    ("ultraplan-phase", "<phase>"),
    ("verify-work", "<phase> [--ws <name>]"),
    ("validate-phase", "<phase>"),
    ("audit-milestone", "<milestone>"),
    ("complete-milestone", "<milestone>"),
    ("ship", "<phase>"),
    (
        "execute-phase",
        "<phase> [--wave N] [--gaps-only] [--interactive] [--tdd]",
    ),
];

fn vendored(stem: &str) -> String {
    let path = Path::new(gsd_kit::snapshot!("commands/gsd")).join(format!("{stem}.md"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The frontmatter lines of a command file.
fn front(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "---");
    let end = lines[1..]
        .iter()
        .position(|l| *l == "---")
        .expect("a closing ---")
        + 1;
    lines[1..end].iter().map(|l| l.to_string()).collect()
}

fn scalar(front: &[String], key: &str) -> Option<String> {
    let line = front
        .iter()
        .find_map(|l| l.strip_prefix(&format!("{key}:")))?;
    let value = line.trim();
    let unquoted = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .map(|v| v.replace("\\\"", "\""))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
                .map(String::from)
        })
        .unwrap_or_else(|| value.to_string());
    Some(unquoted)
}

fn class(tool: &str) -> String {
    match tool {
        "Read" => "read".to_string(),
        "Write" => "write".to_string(),
        "Edit" => "edit".to_string(),
        "Bash" => "shell".to_string(),
        "Glob" | "Grep" => "search".to_string(),
        "WebFetch" => "web_fetch".to_string(),
        "WebSearch" => "web_search".to_string(),
        "Skill" | "SlashCommand" => "skill".to_string(),
        "AskUserQuestion" => "ask_user".to_string(),
        "Agent" => "agent".to_string(),
        "TodoWrite" => "todo".to_string(),
        mcp => {
            let rest = mcp
                .strip_prefix("mcp__")
                .unwrap_or_else(|| panic!("unmapped tool {mcp}"));
            let (server, tool) = rest.split_once("__").expect("mcp__server__tool");
            format!("mcp:{server}/{tool}")
        }
    }
}

/// Upstream's port rule: a line naming one of these is a port line.
fn names_gsd(text: &str) -> bool {
    [".planning", "gsd-tools", "gsd_run", "gsd-sdk"]
        .iter()
        .any(|w| text.contains(w))
}

fn tools(front: &[String]) -> Vec<String> {
    let at = front
        .iter()
        .position(|l| l.starts_with("allowed-tools:"))
        .expect("allowed-tools");
    let mut classes: Vec<String> = Vec::new();
    for line in &front[at + 1..] {
        let Some(tool) = line.trim().strip_prefix("- ") else {
            break;
        };
        let c = class(tool.trim());
        if !classes.contains(&c) {
            classes.push(c);
        }
    }
    classes
}

#[test]
fn every_prompt_slash_keeps_its_upstream_metadata() {
    let declaration = gsd_core::build().declaration();
    let surfaces = &declaration.surfaces;
    for slash in surfaces
        .slash_commands
        .iter()
        .filter(|s| s.prompt.is_some())
    {
        let id = slash.prompt.as_deref().expect("a prompt");
        let attribution = slash
            .attribution
            .as_ref()
            .unwrap_or_else(|| panic!("{id}: no attribution"));
        let stem = attribution
            .path
            .strip_prefix("commands/gsd/")
            .and_then(|p| p.strip_suffix(".md"))
            .unwrap_or_else(|| panic!("{id}: attribution {}", attribution.path));
        assert_eq!(
            stem.replace('-', "_"),
            id,
            "{id}: the command file is the prompt's"
        );
        let fm = front(&vendored(stem));
        let prompt = surfaces
            .prompts
            .iter()
            .find(|p| p.id == id)
            .expect("the prompt");
        // verbatim, except a description that names a tool or a path of GSD's (a port line, D-r43-9)
        let theirs = scalar(&fm, "description");
        let ported = theirs.as_deref().is_some_and(names_gsd) && !names_gsd(&prompt.description);
        if !ported {
            assert_eq!(
                Some(prompt.description.clone()),
                theirs,
                "{id}: description"
            );
        }
        assert_eq!(slash.tools, tools(&fm), "{id}: tools");
        assert_eq!(slash.effort, scalar(&fm, "effort"), "{id}: effort");
        let hint = slash.argument_hint.clone().unwrap_or_default();
        match ADAPTED.iter().find(|(s, _)| *s == stem) {
            Some((_, ours)) => assert_eq!(hint, *ours, "{id}: adapted hint"),
            None => assert_eq!(
                Some(hint).filter(|h| !h.is_empty()),
                scalar(&fm, "argument-hint").filter(|h| !h.is_empty()),
                "{id}: hint"
            ),
        }
    }
}
