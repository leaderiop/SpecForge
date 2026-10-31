//! The ported `gsd-security-auditor` against the vendored upstream files (gsd-core v1.18.0), line for line under
//! core's port rule, its descriptor, and the two cases of upstream's `secure-phase-single-writer.test.cjs`.

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "../../core/tests/common/port.rs"]
mod rule;
use rule::check_lines;

const AGENT: &str = "gsd-security-auditor";

fn upstream_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/upstream/v1.18.0")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// An upstream agent file as `(frontmatter lines, body lines)`.
fn upstream(file: &str) -> (Vec<String>, Vec<String>) {
    let text = read(&upstream_dir().join("agents").join(file));
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    assert_eq!(lines[0], "---", "{file} has no frontmatter");
    let end = lines[1..]
        .iter()
        .position(|l| l == "---")
        .expect("a closing ---")
        + 1;
    (lines[1..end].to_vec(), lines[end + 1..].to_vec())
}

fn ported() -> Regex {
    Regex::new(r"\.planning/|gsd-tools|gsd_run|gsd-sdk").expect("a regex")
}

fn check_body(file: &str, asset: &str) -> usize {
    let (_, body) = upstream(file);
    check_lines(
        file,
        &format!("agents/{file}"),
        asset,
        &body,
        &ported(),
        &|_| false,
    )
}

#[test]
fn both_bodies_are_upstreams_line_for_line() {
    check_body("gsd-security-auditor.md", gsd_security::agents::FULL);
    check_body(
        "gsd-security-auditor.compact.md",
        gsd_security::agents::COMPACT,
    );
}

fn field(front: &[String], key: &str) -> String {
    front
        .iter()
        .find_map(|l| l.strip_prefix(key))
        .unwrap_or_else(|| panic!("no {key}"))
        .trim()
        .to_string()
}

#[test]
fn the_descriptor_is_upstreams_with_the_compact_variant() {
    let declaration = gsd_security::build().declaration();
    let agents = &declaration.surfaces.agents;
    assert_eq!(agents.len(), 1);
    let a = serde_json::to_value(&agents[0]).expect("json");
    let (front, _) = upstream("gsd-security-auditor.md");
    assert_eq!(a["id"], AGENT);
    assert_eq!(a["description"], field(&front, "description:"));
    // Upstream's Read, Bash, Glob, Grep, Skill as core maps them (`gsd/core/tests/port.rs` `class`).
    assert_eq!(
        a["tools"],
        serde_json::json!(["read", "shell", "search", "skill"])
    );
    assert_eq!(a["color"], field(&front, "color:"));
    assert_eq!(a["color"], "red");
    assert_eq!(a["model"], "setting:model_overrides.gsd-security-auditor");
    assert_eq!(a["variants"], serde_json::json!(["compact"]));
    assert_eq!(a["variant_setting"], "specforge.agent_variant");
    assert_eq!(a["attribution"]["path"], "agents/gsd-security-auditor.md");
    assert_eq!(a["attribution"]["version"], "v1.18.0");
}

// Upstream `tests/secure-phase-single-writer.test.cjs` (#2119), 2 cases, both ported here.

#[test]
fn single_writer_the_auditor_tools_hold_no_write_or_edit() {
    let declaration = gsd_security::build().declaration();
    let a = serde_json::to_value(&declaration.surfaces.agents[0]).expect("json");
    let tools: Vec<&str> = a["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t.as_str().expect("str"))
        .collect();
    assert!(!tools.is_empty());
    assert!(
        !tools.contains(&"write") && !tools.contains(&"edit"),
        "{tools:?}"
    );
    // And upstream's own list says the same, so a drift in the map is caught.
    let (front, _) = upstream("gsd-security-auditor.md");
    assert!(
        !front
            .iter()
            .any(|l| l.trim() == "- Write" || l.trim() == "- Edit")
    );
}

#[test]
fn single_writer_the_description_says_it_returns_a_verdict() {
    let declaration = gsd_security::build().declaration();
    let description = &declaration.surfaces.agents[0].description;
    assert!(
        !description.contains("Produces SECURITY.md"),
        "{description}"
    );
    assert!(description.contains("Returns structured"), "{description}");
    // The body says return-only (upstream's role: the orchestrator is the sole writer).
    let body = gsd_security::agents::FULL;
    assert!(
        body.contains("The auditor does NOT write any files"),
        "no return-only statement"
    );
}
