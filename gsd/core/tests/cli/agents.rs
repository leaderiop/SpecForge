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
