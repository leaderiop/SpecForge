//! `new_project` and `new_milestone` (T2): render for three hosts, install as skills on three hosts, hide with their
//! cluster.

use serde_json::json;

use super::harness::{Project, project};
use super::prompts_support::{HOSTS, payload, prompt, render_three_hosts, text_of};

fn step_ids(rendered: &serde_json::Value) -> Vec<String> {
    rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["id"].as_str().expect("an id").to_string())
        .collect()
}

#[test]
fn new_project_renders() {
    let p = project("empty");
    let rendered = payload(&prompt(&p, "new_project", &["--auto"], "claude-code"));
    assert_eq!(
        step_ids(&rendered),
        [
            "research",
            "synthesize",
            "roadmap",
            "record_research",
            "discuss"
        ]
    );
    assert_eq!(
        rendered["payload"]["steps"][4]["prompt"],
        "specforge.gsd.discuss_phase"
    );
    let record = &rendered["payload"]["steps"][3];
    assert_eq!(record["cli"], "specforge gsd record-research");
    assert_eq!(record["tool"], "specforge.gsd.record_research");
    let text = text_of(&rendered);
    assert!(text.contains("<arguments>--auto</arguments>") || text.contains("--auto"));
    assert!(text.contains("Initialize a new project") || text.contains("new project"));
}

#[test]
fn the_preamble_carries_the_current_settings() {
    let p = project("empty");
    p.settings(json!({ "workflow.research": false }));
    let text = text_of(&payload(&prompt(&p, "new_project", &[], "claude-code")));
    assert!(
        text.contains("\"workflow.research\": false"),
        "settings block"
    );
    assert!(
        text.contains("\"audit.enabled\": false"),
        "a default is listed"
    );
}

#[test]
fn new_project_renders_for_three_hosts() {
    let p = project("empty");
    let out = render_three_hosts(
        &p,
        "new_project",
        &["--auto", "--idea", "a todo app"],
        "--auto --idea \"a todo app\"",
        "<objective>",
    );
    assert_eq!(out.len(), 3);
    for (host, rendered) in out {
        assert!(text_of(&rendered).contains("a todo app"), "{host}");
        assert!(text_of(&rendered).contains("`--idea`"), "{host}: the tail");
    }
}

#[test]
fn new_milestone_renders_for_three_hosts() {
    let p = project("empty");
    let out = render_three_hosts(
        &p,
        "new_milestone",
        &["--name", "v2 search", "--reset-phase-numbers"],
        "--name \"v2 search\" --reset-phase-numbers",
        "<objective>",
    );
    assert_eq!(out.len(), 3);
    for (host, rendered) in out {
        assert!(text_of(&rendered).contains("--name"), "{host}");
        assert_eq!(
            step_ids(&rendered),
            [
                "research",
                "synthesize",
                "roadmap",
                "record_research",
                "discuss"
            ],
            "{host}"
        );
    }
}

#[test]
fn new_milestone_refuses_a_bare_name_word() {
    let p = project("empty");
    let run = prompt(
        &p,
        "new_milestone",
        &["--name", "v2", "search"],
        "claude-code",
    );
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("invalid_input"), "{}", run.all());
    assert!(run.all().contains("search"), "{}", run.all());
}

fn skill_files(p: &Project, host_dir: &str, skill: &str) -> String {
    p.read(&format!("{host_dir}/skills/{skill}/SKILL.md"))
}

#[test]
fn new_project_installs_as_a_skill_on_three_hosts() {
    let p = project("empty");
    for host in HOSTS {
        let run = p.install(host, &[]);
        assert_eq!(run.code, 0, "{host}: {}", run.all());
    }
    for (dir, host) in [
        (".claude", "claude-code"),
        (".codex", "codex"),
        (".cursor", "cursor"),
    ] {
        for (skill, id) in [
            ("gsd-new-project", "new_project"),
            ("gsd-new-milestone", "new_milestone"),
        ] {
            let text = skill_files(&p, dir, skill);
            assert!(
                text.contains(&format!("specforge.gsd.{id}")),
                "{host} {skill}"
            );
            assert!(
                text.contains(&format!("\"host\": \"{host}\"")),
                "{host} {skill}"
            );
        }
    }
}

#[test]
fn a_disabled_cluster_hides_the_new_milestone_skill() {
    let p = project("empty");
    p.settings(json!({ "specforge.surface.milestone": false }));
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!p.exists(".claude/skills/gsd-new-milestone/SKILL.md"));
    assert!(p.exists(".claude/skills/gsd-new-project/SKILL.md"));
    // The prompt is still served.
    let rendered = payload(&prompt(&p, "new_milestone", &[], "claude-code"));
    assert_eq!(
        rendered["payload"]["prompt"],
        "@gsd/core@0.1.0:new_milestone"
    );
}
