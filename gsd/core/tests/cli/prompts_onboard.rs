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
        "specforge export --kinds project,milestone,phase,requirement",
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
    let args = ["docs", "--mode", "merge", "--resolve", "auto"];
    for (host, rendered) in render_three_hosts(
        &p,
        "ingest_docs",
        &args,
        "docs --mode merge --resolve auto",
        "INGEST DOCS",
    ) {
        let got = steps(&rendered);
        // a step names its agent in its `agent` field (ADR 0313)
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
            assert_eq!(step["agent"], agent, "{host}: {step}");
        }
        assert!(text_of(&rendered).contains("mktemp -d"), "{host}");
    }
}

/// `[path]` is the free-text tail, and `--path` by name is the same argument (the prompt owns that option).
#[test]
fn ingest_docs_takes_a_path() {
    let p = twin();
    for words in [vec!["docs/adr"], vec!["--path", "docs/adr"]] {
        let text = text_of(&payload(&prompt(&p, "ingest_docs", &words, "claude-code")));
        assert!(
            text.contains("<arguments>docs/adr</arguments>"),
            "{words:?}"
        );
    }
    // the command line of the extension reaches it too
    let run = p.run(&["gsd", "ingest-docs", "docs/adr"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        run.stdout.contains("<arguments>docs/adr</arguments>"),
        "{}",
        run.stdout
    );
}

/// REVIEW-k20b m1: a `--path` names the project whatever the current directory is. Here the current directory is a
/// project whose `ingest_docs` declares `path`, which once took the option for its own argument.
#[test]
fn a_path_names_the_project_when_the_cwd_is_another() {
    let (here, other) = (twin(), twin());
    let root = other.root().display().to_string();
    let run = here.run(&[
        "prompt",
        "--path",
        &root,
        "gsd.ingest_docs",
        "--host",
        "claude-code",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        !run.stdout.contains(&root),
        "the path became an argument: {}",
        run.stdout
    );
    let run = here.run(&[
        "gsd",
        "ingest-docs",
        "--path",
        &root,
        "--host",
        "claude-code",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        !run.stdout.contains(&root),
        "the path became an argument: {}",
        run.stdout
    );
    // The prompt's own path is the text after the host's `--`.
    let run = here.run(&[
        "prompt",
        "gsd.ingest_docs",
        "--host",
        "claude-code",
        "--",
        "docs/adr",
    ]);
    assert!(
        run.stdout.contains("<arguments>docs/adr</arguments>"),
        "{}",
        run.all()
    );
}

/// REVIEW-k20b m1: a command with `--path` loads the project it names only; a current directory whose
/// `specforge.json` is broken does not refuse it.
#[test]
fn a_command_with_path_loads_one_project() {
    let (here, other) = (twin(), twin());
    here.write("specforge.json", "{ not json");
    let root = other.root().display().to_string();
    let run = here.run(&["gsd", "smart-entry", "--path", &root]);
    assert_eq!(run.code, 0, "{}", run.all());
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

/// REVIEW-r43 M8: the blocker gate reads `{conflicts_path}`, so the synthesizer must be told to write it (and the intel
/// files) to the paths the orchestrator gives, and a conflicts file that is missing or unreadable is a blocker.
#[test]
fn the_synthesizer_writes_the_conflicts_file_the_gate_reads() {
    for agent in ["gsd-doc-synthesizer.md", "gsd-doc-synthesizer.compact.md"] {
        let text = std::fs::read_to_string(format!(
            "{}/assets/agents/{agent}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("the agent");
        for stale in [
            "return the synthesis in your reply",
            "report the conflicts to the user",
            "not part of @gsd/core",
        ] {
            assert!(!text.contains(stale), "{agent}: still says '{stale}'");
        }
    }
    let p = twin();
    let text = text_of(&payload(&prompt(&p, "ingest_docs", &[], "claude-code")));
    assert!(
        text.contains("A missing, empty or unparsable conflicts file is a BLOCKER"),
        "the gate treats a missing conflicts file as a pass"
    );
}
