//! The 15 core agents, rendered by the real host into `.claude/agents/`.

use super::harness::{Project, project};
use gsd_core::agents::roster::AGENTS;

/// The `---` frontmatter of a rendered agent file as `(key, value)` lines, and the rest.
pub fn split(file: &str) -> (Vec<(String, String)>, String) {
    let mut lines = file.split('\n');
    assert_eq!(lines.next(), Some("---"), "no frontmatter in:\n{file}");
    let mut front = Vec::new();
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
        let (k, v) = line.split_once(':').unwrap_or((line, ""));
        front.push((k.to_string(), v.trim().to_string()));
    }
    (front, lines.collect::<Vec<_>>().join("\n"))
}

pub fn value<'a>(front: &'a [(String, String)], key: &str) -> Option<&'a str> {
    front
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Installs into `host` and answers the project.
pub fn installed(p: &Project, host: &str) {
    let run = p.install(host, &[]);
    assert_eq!(run.code, 0, "install {host}: {}", run.all());
}

pub fn claude_agent(p: &Project, id: &str) -> String {
    p.read(&format!(".claude/agents/{id}.md"))
}

#[test]
fn every_core_agent_renders() {
    let p = project("empty");
    installed(&p, "claude-code");
    assert_eq!(AGENTS.len(), 15);
    for agent in AGENTS {
        let file = claude_agent(&p, agent.id);
        let (front, body) = split(&file);
        assert_eq!(value(&front, "name"), Some(agent.id), "{file}");
        assert!(
            value(&front, "description").is_some_and(|d| !d.is_empty()),
            "{file}"
        );
        assert!(value(&front, "tools").is_some(), "{file}");
        assert_eq!(value(&front, "color"), Some(agent.color), "{file}");
        assert!(
            body.contains(&agent.full[agent.full.find('\n').unwrap_or(0)..]),
            "{}",
            agent.id
        );
    }
    let listed = std::fs::read_dir(p.root().join(".claude/agents"))
        .expect("the agents folder")
        .count();
    assert_eq!(listed, 15);
}

#[test]
fn a_rendered_agent_carries_its_attribution() {
    let p = project("empty");
    installed(&p, "claude-code");
    for agent in AGENTS {
        let file = claude_agent(&p, agent.id);
        let notice = format!(
            "<!-- Ported from gsd-core agents/{}.md at v1.18.0. Copyright (c) 2026 Open GSD. Licensed under MIT. -->",
            agent.id
        );
        assert!(
            file.trim_end().ends_with(&notice),
            "{}:\n{}",
            agent.id,
            &file[file.len().saturating_sub(300)..]
        );
    }
}

fn compact_ids() -> Vec<&'static str> {
    AGENTS
        .iter()
        .filter(|a| a.compact.is_some())
        .map(|a| a.id)
        .collect()
}

/// The text a cursor agent file holds after its frontmatter.
fn cursor_agent(p: &Project, id: &str) -> String {
    split(&p.read(&format!(".cursor/agents/{id}.md"))).1
}

#[test]
fn compact_variant_renders() {
    let p = project("empty");
    p.settings(serde_json::json!({ "workflow.compact_content": true }));
    installed(&p, "cursor");
    let mut compact = 0;
    for agent in AGENTS {
        let body = cursor_agent(&p, agent.id);
        let expected = agent.compact.unwrap_or(agent.full);
        assert!(body.contains(expected.trim_end()), "{}", agent.id);
        if agent.compact.is_some() {
            compact += 1;
            assert!(
                !body.contains(agent.full.trim_end()),
                "{} is full",
                agent.id
            );
        }
    }
    assert_eq!(compact, 10);
}

#[test]
fn compact_is_not_rendered_unless_asked() {
    let p = project("empty");
    installed(&p, "cursor");
    for agent in AGENTS {
        assert!(
            cursor_agent(&p, agent.id).contains(agent.full.trim_end()),
            "{}",
            agent.id
        );
    }
}

#[test]
fn claude_code_never_renders_compact() {
    let p = project("empty");
    p.settings(serde_json::json!({ "workflow.compact_content": true }));
    installed(&p, "claude-code");
    for agent in AGENTS {
        let body = split(&claude_agent(&p, agent.id)).1;
        assert!(body.contains(agent.full.trim_end()), "{}", agent.id);
    }
}

#[test]
fn an_explicit_variant_beats_the_preset() {
    let p = project("empty");
    p.settings(serde_json::json!({
        "workflow.compact_content": true,
        "specforge.agent_variant": "full"
    }));
    installed(&p, "cursor");
    for agent in AGENTS {
        assert!(
            cursor_agent(&p, agent.id).contains(agent.full.trim_end()),
            "{}",
            agent.id
        );
    }
}

#[test]
fn exactly_ten_agents_have_a_compact_variant() {
    assert_eq!(
        compact_ids(),
        [
            "gsd-advisor-researcher",
            "gsd-assumptions-analyzer",
            "gsd-doc-classifier",
            "gsd-doc-synthesizer",
            "gsd-integration-checker",
            "gsd-nyquist-auditor",
            "gsd-pattern-mapper",
            "gsd-project-researcher",
            "gsd-research-synthesizer",
            "gsd-roadmapper",
        ]
    );
}

fn model_of(p: &Project, id: &str) -> Option<String> {
    let (front, _) = split(&claude_agent(p, id));
    value(&front, "model").map(String::from)
}

#[test]
fn budget_profile_picks_haiku_for_light_agents() {
    let p = project("empty");
    p.settings(serde_json::json!({ "model_profile": "budget" }));
    installed(&p, "claude-code");
    assert_eq!(model_of(&p, "gsd-plan-checker").as_deref(), Some("haiku"));
    assert_eq!(model_of(&p, "gsd-planner").as_deref(), Some("sonnet"));
}

#[test]
fn the_default_profile_is_balanced() {
    let p = project("empty");
    installed(&p, "claude-code");
    // balanced: planner opus, executor sonnet, doc-classifier haiku
    assert_eq!(model_of(&p, "gsd-planner").as_deref(), Some("opus"));
    assert_eq!(model_of(&p, "gsd-executor").as_deref(), Some("sonnet"));
    assert_eq!(model_of(&p, "gsd-doc-classifier").as_deref(), Some("haiku"));
}

#[test]
fn an_override_beats_the_profile() {
    let p = project("empty");
    p.settings(serde_json::json!({
        "model_profile": "budget",
        "model_overrides.gsd-planner": "opus"
    }));
    installed(&p, "claude-code");
    assert_eq!(model_of(&p, "gsd-planner").as_deref(), Some("opus"));
    assert_eq!(model_of(&p, "gsd-plan-checker").as_deref(), Some("haiku"));
}

#[test]
fn inherit_writes_no_model_line() {
    let p = project("empty");
    p.settings(serde_json::json!({ "model_profile": "inherit" }));
    installed(&p, "claude-code");
    for agent in AGENTS {
        assert_eq!(model_of(&p, agent.id), None, "{}", agent.id);
        // the file exists, so a missing line is the profile's doing
        assert!(claude_agent(&p, agent.id).contains("name:"));
    }
}

#[test]
fn adaptive_follows_the_routing_tier() {
    use gsd_core::settings::models::{MODELS, tier};
    let p = project("empty");
    p.settings(serde_json::json!({ "model_profile": "adaptive" }));
    installed(&p, "claude-code");
    for (agent, _, routing) in MODELS {
        let want = match *routing {
            "heavy" => "opus",
            "light" => "haiku",
            _ => "sonnet",
        };
        assert_eq!(model_of(&p, agent).as_deref(), Some(want), "{agent}");
    }
    assert_eq!(tier("opus"), "heavy");
}
