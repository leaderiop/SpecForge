//! `ingest_docs` and `onboard` (T9): ingestion writes entities through the roadmapper, and onboarding takes the
//! "Skip mapping" path because `@gsd/codebase` is not installed.

use serde_json::Value;

use super::commands_support::twin;
use super::harness::project;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

fn steps(rendered: &Value) -> Vec<(String, Value)> {
    rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| (s["id"].as_str().expect("id").to_string(), s.clone()))
        .collect()
}

#[test]
fn onboard_renders() {
    let p = project("empty");
    let rendered = payload(&prompt(&p, "onboard", &[], "claude-code"));
    let got = steps(&rendered);
    let ids: Vec<&str> = got.iter().map(|(i, _)| i.as_str()).collect();
    assert_eq!(ids, ["ingest", "new_project", "manager"]);
    for (id, prompt) in [
        ("ingest", "ingest_docs"),
        ("new_project", "new_project"),
        ("manager", "manager"),
    ] {
        let step = &got.iter().find(|(i, _)| i == id).expect("a step").1;
        assert_eq!(step["prompt"], format!("specforge.gsd.{prompt}"), "{step}");
    }
    let text = text_of(&rendered);
    for word in [
        "Skip mapping",
        "@gsd/codebase",
        "not installed",
        "specforge query --kinds project,milestone,phase,requirement",
    ] {
        assert!(text.contains(word), "no '{word}'");
    }
}

#[test]
fn onboard_renders_for_three_hosts() {
    let p = project("empty");
    render_three_hosts(
        &p,
        "onboard",
        &["--fast", "--text"],
        "--fast --text",
        "Onboarding",
    );
}

#[test]
fn ingest_docs_renders_for_three_hosts() {
    let p = twin();
    let args = ["--dir", "docs", "--mode", "merge", "--resolve", "auto"];
    for (host, rendered) in render_three_hosts(
        &p,
        "ingest_docs",
        &args,
        "--dir \"docs\" --mode merge --resolve auto",
        "INGEST DOCS",
    ) {
        let got = steps(&rendered);
        // a rendered step carries no agent (K19): the title names it
        for (id, agent) in [
            ("classify", "gsd-doc-classifier"),
            ("synthesize", "gsd-doc-synthesizer"),
            ("roadmap", "gsd-roadmapper"),
        ] {
            let step = &got
                .iter()
                .find(|(i, _)| i == id)
                .unwrap_or_else(|| panic!("{host}: no {id}"))
                .1;
            assert!(
                step["title"].as_str().unwrap_or_default().contains(agent),
                "{host}: {step}"
            );
        }
        assert!(text_of(&rendered).contains("mktemp -d"), "{host}");
    }
}

/// `--path` is the host's option: the project is read from `docs`, and no prompt argument carries that name.
#[test]
fn ingest_docs_has_no_argument_named_path() {
    let p = twin();
    let run = prompt(&p, "ingest_docs", &["--path", "docs"], "claude-code");
    assert!(!run.all().contains("--dir"), "{}", run.all());
    let rendered = payload(&prompt(
        &p,
        "ingest_docs",
        &["--dir", "docs"],
        "claude-code",
    ));
    let args = rendered["payload"]["args"].to_string();
    assert!(!args.contains("\"path\""), "{args}");
}

#[test]
fn ingest_docs_description_names_no_planning() {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/install/claude-code/skills/gsd-ingest-docs.md"),
    )
    .expect("the skill golden");
    let description = skill
        .lines()
        .find(|l| l.starts_with("description:"))
        .expect("a description");
    assert!(!description.contains(".planning"), "{description}");
    assert!(description.contains("SpecForge spec"), "{description}");
}
