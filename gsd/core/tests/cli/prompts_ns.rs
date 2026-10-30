//! The six `/gsd-ns-*` routers (T11): each renders upstream's body and the route table of this install.

use serde_json::json;

use gsd_core::prompts::roster::{Owner, owner};

use super::commands_support::twin;
use super::harness::project;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const ROUTERS: [&str; 6] = [
    "context", "ideate", "manage", "project", "review", "workflow",
];
const INTENT: &str = "plan the next phase";

fn vendored(router: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/upstream/v1.18.0/commands/gsd")
        .join(format!("ns-{router}.md"));
    std::fs::read_to_string(path).expect("the vendored router")
}

/// The `requires: [a, b]` targets of upstream's frontmatter, in order.
fn requires(router: &str) -> Vec<String> {
    let text = vendored(router);
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("requires: ["))
        .expect("a requires line");
    line.trim_end_matches(']')
        .split(", ")
        .map(String::from)
        .collect()
}

/// The `/gsd-x` targets of the route table the render appended, in order, with their second column.
fn route_rows(text: &str) -> Vec<(String, String)> {
    let table = text
        .split("## Route table on this install")
        .nth(1)
        .expect("a route table");
    table
        .lines()
        .filter_map(|l| {
            l.strip_prefix("| `/gsd-")
                .or_else(|| l.strip_prefix("| `$gsd-"))
        })
        .map(|l| {
            let (name, rest) = l.split_once("` | ").expect("a row");
            (name.to_string(), rest.trim_end_matches(" |").to_string())
        })
        .collect()
}

#[test]
fn ns_skills_route() {
    let p = twin();
    for router in ROUTERS {
        let id = format!("ns_{router}");
        let args = ["--intent", INTENT];
        for (host, rendered) in render_three_hosts(
            &p,
            &id,
            &args,
            "--intent \"plan the next phase\"",
            "Route to the appropriate",
        ) {
            let text = text_of(&rendered);
            // upstream's table rows are verbatim
            for row in vendored(router).lines().filter(|l| l.starts_with("| ")) {
                assert!(text.contains(row), "{router} {host}: no '{row}'");
            }
            let rows = route_rows(&text);
            let names: Vec<&str> = rows.iter().map(|(n, _)| n.as_str()).collect();
            let wanted = requires(router);
            assert_eq!(names, wanted, "{router} {host}");
            for (name, what) in &rows {
                match owner(name).expect("a roster row") {
                    Owner::Extension(extension) => {
                        assert!(what.starts_with("not installed"), "{name}: {what}");
                        assert!(what.contains(extension), "{name}: {what}");
                    }
                    _ => assert!(what.ends_with("(served by @gsd/core)"), "{name}: {what}"),
                }
            }
            // REVIEW-r43 Minor 18: a router has no shell block and does not know what is installed on the host
            assert!(
                !text.contains("In the shell blocks above"),
                "{router} {host}"
            );
            assert!(!text.contains("(installed)"), "{router} {host}");
            assert!(
                rendered["payload"]["steps"]
                    .as_array()
                    .is_none_or(|s| s.is_empty()),
                "{router} {host}: steps"
            );
        }
    }
}

/// The targets of each router are upstream's `requires:`, and every one is a command of the roster.
#[test]
fn ns_targets_match_upstreams_requires() {
    let p = twin();
    for router in ROUTERS {
        let text = text_of(&payload(&prompt(
            &p,
            &format!("ns_{router}"),
            &[],
            "claude-code",
        )));
        let names: Vec<String> = route_rows(&text).into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, requires(router), "{router}");
        for name in names {
            assert!(
                owner(&name).is_some(),
                "{router}: {name} is not in the roster"
            );
        }
    }
}

#[test]
fn ns_ideate_is_hidden_by_research_ideate() {
    let p = project("empty");
    p.settings(json!({ "specforge.surface.research_ideate": false }));
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!p.exists(".claude/skills/gsd-ns-ideate/SKILL.md"));
    assert!(p.exists(".claude/skills/gsd-ns-workflow/SKILL.md"));
    // the prompt itself is always served
    let rendered = payload(&prompt(
        &p,
        "ns_ideate",
        &["--intent", "sketch it"],
        "claude-code",
    ));
    assert!(text_of(&rendered).contains("Route to the appropriate"));
}

#[test]
fn an_ns_skill_has_the_read_and_skill_tools_only() {
    let p = project("empty");
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    for router in ROUTERS {
        let skill = p.read(&format!(".claude/skills/gsd-ns-{router}/SKILL.md"));
        let tools: Vec<&str> = skill
            .lines()
            .skip_while(|l| !l.starts_with("allowed-tools:"))
            .skip(1)
            .take_while(|l| l.starts_with("  - "))
            .map(|l| l.trim_start_matches("  - "))
            .filter(|t| !t.starts_with("mcp__"))
            .collect();
        assert_eq!(tools, ["Read", "Skill"], "{router}");
    }
}

#[test]
fn ns_route_table_converts_for_codex() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "ns_workflow",
        &["--intent", INTENT],
        "codex",
    )));
    let table = text
        .split("## Route table on this install")
        .nth(1)
        .expect("a table");
    assert!(table.contains("`$gsd-plan-phase`"), "{table}");
    assert!(!table.contains("`/gsd-"), "{table}");
}
