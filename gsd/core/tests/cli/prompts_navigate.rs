//! `next`, `progress` and `manager` (T1): prompts that read the project's situation. `next` routes from the command
//! `smart_entry`, so the same prompt on three projects names three different recommended actions.

use serde_json::Value;

use super::commands_support::{gsd, json, twin};
use super::harness::project;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

fn step_ids(rendered: &Value) -> Vec<String> {
    rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["id"].as_str().expect("an id").to_string())
        .collect()
}

/// The `recommended` action `smart-entry` answers on `p`.
fn recommended(p: &super::harness::Project) -> String {
    let answer = json(&gsd(p, "smart-entry", &["--format", "json"]));
    answer["recommended"]
        .as_str()
        .expect("recommended")
        .to_string()
}

#[test]
fn next_routes_by_state() {
    let rendered = payload(&prompt(&twin(), "next", &[], "claude-code"));
    assert_eq!(step_ids(&rendered), ["detect"]);
    assert_eq!(
        rendered["payload"]["steps"][0]["cli"],
        "specforge gsd smart-entry --host claude-code"
    );
    let text = text_of(&rendered);
    assert!(text.contains("specforge gsd smart-entry"), "{text}");
    for stem in [
        "/gsd-debug",
        "/gsd-code-review",
        "/gsd-capture",
        "/gsd-map-codebase",
    ] {
        assert!(
            text.contains(&format!("- `{stem}`: not served on this project")),
            "{stem}"
        );
    }
    // The command the step names answers differently on different projects.
    let paused = twin();
    let pause = gsd(&paused, "pause", &["--by", "me", "stopped for lunch"]);
    assert_eq!(pause.code, 0, "{}", pause.all());
    let seen = [
        recommended(&project("empty")),
        recommended(&twin()),
        recommended(&paused),
    ];
    assert_ne!(seen[0], seen[1], "{seen:?}");
    assert_ne!(seen[1], seen[2], "{seen:?}");
}

#[test]
fn next_renders_for_three_hosts() {
    render_three_hosts(&twin(), "next", &["--text"], "--text", "smart entry");
}

#[test]
fn progress_renders_for_three_hosts() {
    let args = ["--next", "--auto", "--do", "fix the title trim"];
    let out = render_three_hosts(
        &twin(),
        "progress",
        &args,
        "--next --auto --do \"fix the title trim\"",
        "<purpose>",
    );
    assert_eq!(step_ids(&out[0].1), ["report"]);
}

#[test]
fn progress_names_the_active_workstream() {
    let p = project("workstreams");
    let said =
        |p: &super::harness::Project| text_of(&payload(&prompt(p, "progress", &[], "claude-code")));
    assert!(
        said(&p).contains("No workstream is active."),
        "{}",
        said(&p)
    );
    gsd(&p, "workstreams", &["switch", "--name", "auth"]);
    assert!(
        said(&p).contains("Active workstream: auth."),
        "{}",
        said(&p)
    );
}

#[test]
fn progress_names_progress_report_and_its_three_modes() {
    let text = text_of(&payload(&prompt(&twin(), "progress", &[], "claude-code")));
    assert!(text.contains("specforge gsd progress-report"), "{text}");
    for mode in ["--forensic", "--next", "--do"] {
        assert!(text.contains(mode), "{mode}");
    }
}

#[test]
fn manager_renders_for_three_hosts() {
    render_three_hosts(
        &twin(),
        "manager",
        &["--analyze-deps"],
        "--analyze-deps",
        "<purpose>",
    );
}

#[test]
fn manager_steps_into_the_lifecycle_prompts() {
    let rendered = payload(&prompt(&twin(), "manager", &[], "claude-code"));
    assert_eq!(
        step_ids(&rendered),
        [
            "dashboard",
            "discuss",
            "plan",
            "execute",
            "verify",
            "complete"
        ]
    );
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    assert_eq!(
        steps[0]["cli"],
        "specforge gsd progress-report --host claude-code"
    );
    for (step, prompt) in steps[1..].iter().zip([
        "discuss_phase",
        "plan_phase",
        "execute_phase",
        "verify_work",
        "complete_milestone",
    ]) {
        assert_eq!(step["prompt"], format!("specforge.gsd.{prompt}"), "{step}");
    }
}

/// REVIEW-r43 Minor 17 and 22: when `specforge` itself fails, `next` does not recover by running `specforge query`; and
/// its resume scan counts the `plan` and `summary` nodes the query answers, not `plans` and `summaries` arrays.
#[test]
fn next_recovers_without_specforge_and_counts_nodes() {
    let p = twin();
    let text = text_of(&payload(&prompt(&p, "next", &[], "claude-code")));
    let recovery = text
        .split("**`specforge` itself fails**")
        .nth(1)
        .and_then(|t| t.split("\n2. ").next())
        .expect("the first recovery");
    assert!(recovery.contains("`spec/*.spec`"), "{recovery}");
    assert!(
        !recovery.contains("Read the computed project state with `specforge query"),
        "{recovery}"
    );
    assert!(!text.contains("(.plans // [])") && !text.contains("(.summaries // [])"));
}
