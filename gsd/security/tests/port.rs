//! The ported `gsd-security-auditor` against the vendored upstream files (gsd-core v1.19.0), line for line under
//! core's port rule, its descriptor, and the two cases of upstream's `secure-phase-single-writer.test.cjs`.

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "../../core/tests/common/port.rs"]
mod rule;
use rule::check_lines;

const AGENT: &str = "gsd-security-auditor";

fn upstream_dir() -> PathBuf {
    PathBuf::from(gsd_kit::snapshot!())
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
    assert_eq!(
        a["attribution"]["version"],
        concat!("v", gsd_kit::tracked_version!())
    );
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

// The command file, the workflow and the template (plan 16 T22), under the same rule: a line may differ only where
// upstream's names a ported tool, or where it is listed here with its reason.

/// Body lines (0-based) of `gsd-core/workflows/secure-phase.md` that differ although upstream's names no ported tool.
/// Upstream line numbers are 1-based, so each is `n - 1`.
const REWRITTEN: &[(&str, usize)] = &[
    // the purpose and the audit are the phase's `security_audit`
    ("gsd-core/workflows/secure-phase.md", 1),
    // state detection reads the audit, the plans and the summaries from the spec, not `*-SECURITY.md` files
    ("gsd-core/workflows/secure-phase.md", 51),
    ("gsd-core/workflows/secure-phase.md", 52),
    ("gsd-core/workflows/secure-phase.md", 53),
    ("gsd-core/workflows/secure-phase.md", 56),
    ("gsd-core/workflows/secure-phase.md", 57),
    // the plans' `<threat_model>` is `trust_boundary` and `threat` entities, the flags a summary's `threat_flags`
    ("gsd-core/workflows/secure-phase.md", 64),
    ("gsd-core/workflows/secure-phase.md", 68),
    ("gsd-core/workflows/secure-phase.md", 74),
    // option 2 is `accept_risk`, a person's command
    ("gsd-core/workflows/secure-phase.md", 98),
    // the settings are read from the prompt's settings, not `config-get`
    ("gsd-core/workflows/secure-phase.md", 108),
    // step 6 creates and updates the audit with `record_security_audit`
    ("gsd-core/workflows/secure-phase.md", 144),
    ("gsd-core/workflows/secure-phase.md", 146),
    ("gsd-core/workflows/secure-phase.md", 150),
    ("gsd-core/workflows/secure-phase.md", 154),
    ("gsd-core/workflows/secure-phase.md", 155),
    // accepted risks are `accept_risk`, not a line in SECURITY.md
    ("gsd-core/workflows/secure-phase.md", 164),
    // the commit stages the spec
    ("gsd-core/workflows/secure-phase.md", 173),
];

fn vendored_body(path: &str) -> Vec<String> {
    let text = read(&upstream_dir().join(path));
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    if path.starts_with("commands/") {
        let end = lines[1..]
            .iter()
            .position(|l| l == "---")
            .expect("a closing ---")
            + 1;
        lines.drain(..=end);
    }
    lines
}

#[test]
fn the_secure_phase_assets_are_upstreams_but_for_the_ported_lines() {
    let mut changed = 0;
    for (path, asset) in gsd_security::prompts::ASSETS {
        changed += check_lines(path, path, asset, &vendored_body(path), &ported(), &|n| {
            REWRITTEN.contains(&(*path, n))
        });
    }
    // the ported lines: 11 name a tool, 18 are listed above (the command and the template are upstream's)
    assert_eq!(changed, 29, "the number of changed lines");
}

/// Plan 16 T4's line, asked here too: the hooks of every capability extension at `verify:post`.
#[test]
fn the_hooks_line_asks_the_capability_extensions() {
    let workflow =
        gsd_security::prompts::asset("gsd-core/workflows/secure-phase.md").expect("the workflow");
    assert!(workflow.contains(
        "VERIFY_POST_HOOKS_JSON=$(for c in ai codebase ops security ui; do specforge gsd \"$c-hooks\" verify:post --format json 2>/dev/null; done | jq -sc '{activeHooks: (map(.activeHooks) | add // []), skippedHooks: (map(.skippedHooks) | add // [])}')"
    ));
}

/// REVIEW-security-part2 M1: step 4 option 2 must not tell the agent to close threats the person did not accept.
#[test]
fn the_secure_phase_text_does_not_close_accepted_threats() {
    let workflow =
        gsd_security::prompts::asset("gsd-core/workflows/secure-phase.md").expect("the workflow");
    let option = workflow
        .lines()
        .find(|l| l.starts_with("2. \"Accept all open"))
        .expect("option 2");
    assert!(!option.contains("set all CLOSED"), "{option}");
    assert!(option.contains("do not record it closed"), "{option}");
}
