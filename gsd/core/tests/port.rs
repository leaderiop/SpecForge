//! The ported agent bodies against the vendored upstream files (gsd-core v1.18.0): line for line, a line may differ
//! only where upstream's names `.planning/`, `gsd-tools`, `gsd_run` or `gsd-sdk`.

use std::path::{Path, PathBuf};

use gsd_core::agents::roster::AGENTS;
use regex::Regex;

pub fn upstream_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/upstream/v1.18.0/agents")
}

pub fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/agents")
}

fn ported() -> Regex {
    Regex::new(r"\.planning/|gsd-tools|gsd_run|gsd-sdk").expect("a regex")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// An upstream file as `(frontmatter lines, body lines)`.
fn upstream(file: &str) -> (Vec<String>, Vec<String>) {
    let text = read(&upstream_dir().join(file));
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    assert_eq!(lines[0], "---", "{file} has no frontmatter");
    let end = lines[1..]
        .iter()
        .position(|l| l == "---")
        .expect("a closing ---")
        + 1;
    (lines[1..end].to_vec(), lines[end + 1..].to_vec())
}

/// Checks one asset against its upstream file: the header, then every line.
fn check_body(file: &str) -> usize {
    let (_, body) = upstream(file);
    let asset = read(&assets_dir().join(file));
    let lines: Vec<&str> = asset.split('\n').collect();
    assert!(
        lines[0].starts_with(&format!(
            "<!-- Ported from gsd-core v1.18.0 agents/{file}, MIT"
        )),
        "{file}: header {}",
        lines[0]
    );
    assert_eq!(lines.len() - 1, body.len(), "{file}: line count");
    let rx = ported();
    let mut changed = 0;
    for (n, (mine, theirs)) in lines[1..].iter().zip(&body).enumerate() {
        if rx.is_match(theirs) {
            assert_ne!(
                mine, theirs,
                "{file}:{n}: a ported line is left as upstream's"
            );
            assert!(
                !rx.is_match(mine),
                "{file}:{n}: still names upstream's tools: {mine}"
            );
            changed += 1;
        } else {
            assert_eq!(mine, theirs, "{file}:{n}: differs outside a ported line");
        }
    }
    changed
}

#[test]
fn bodies_diff_only_in_ported_passages() {
    assert_eq!(AGENTS.len(), 15);
    let mut total = 0;
    for agent in AGENTS {
        total += check_body(&format!("{}.md", agent.id));
    }
    assert!(total > 100, "only {total} lines were ported");
}

#[test]
fn no_asset_names_planning_or_gsd_tools() {
    let rx = ported();
    let mut seen = 0;
    for entry in std::fs::read_dir(assets_dir()).expect("assets") {
        let path = entry.expect("an entry").path();
        let text = read(&path);
        seen += 1;
        for (n, line) in text.lines().enumerate() {
            assert!(!rx.is_match(line), "{}:{}: {line}", path.display(), n + 1);
        }
    }
    assert!(seen >= 15, "{seen} assets");
}

fn class(tool: &str) -> String {
    match tool {
        "Read" => "read",
        "Write" => "write",
        "Edit" => "edit",
        "Bash" => "shell",
        "Glob" | "Grep" => "search",
        "WebFetch" => "web_fetch",
        "WebSearch" => "web_search",
        "Skill" => "skill",
        "AskUserQuestion" => "ask_user",
        "Agent" => "agent",
        mcp => {
            let server = mcp
                .strip_prefix("mcp__")
                .and_then(|m| m.strip_suffix("__*"))
                .unwrap_or_else(|| panic!("unmapped tool {mcp}"));
            return format!("mcp:{server}/*");
        }
    }
    .to_string()
}

/// The tools of an upstream frontmatter: a comma list or a YAML list.
fn upstream_tools(front: &[String]) -> Vec<String> {
    let at = front
        .iter()
        .position(|l| l.starts_with("tools:"))
        .expect("tools");
    let rest = front[at]["tools:".len()..].trim();
    let names: Vec<String> = if rest.is_empty() {
        front[at + 1..]
            .iter()
            .map_while(|l| l.trim().strip_prefix("- ").map(|t| t.trim().to_string()))
            .collect()
    } else {
        rest.split(',').map(|t| t.trim().to_string()).collect()
    };
    let mut classes = Vec::new();
    for name in names {
        let c = class(&name);
        if !classes.contains(&c) {
            classes.push(c);
        }
    }
    classes
}

#[test]
fn tools_are_upstreams() {
    assert_eq!(AGENTS.len(), 15);
    let rx = ported();
    for agent in AGENTS {
        let (front, _) = upstream(&format!("{}.md", agent.id));
        let expected = upstream_tools(&front);
        assert!(!expected.is_empty(), "{}", agent.id);
        assert_eq!(agent.tools.to_vec(), expected, "{}", agent.id);
        let field = |key: &str| {
            front
                .iter()
                .find_map(|l| l.strip_prefix(key))
                .expect("a frontmatter key")
                .trim()
                .to_string()
        };
        assert_eq!(agent.color, field("color:"), "{}", agent.id);
        let theirs = field("description:");
        if rx.is_match(&theirs) {
            assert!(!rx.is_match(agent.description), "{}", agent.id);
        } else {
            assert_eq!(agent.description, theirs, "{}", agent.id);
        }
    }
}
