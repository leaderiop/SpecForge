//! `pause_work` and `resume_work` (T4): the session handoff round-trips through the lines the prompts give.

use super::commands_support::{gsd, json, twin};
use super::harness::{Project, project};
use super::prompts_support::{command, payload, prompt, render_three_hosts, text_of};

fn situation(p: &super::harness::Project) -> String {
    let answer = json(&gsd(p, "smart-entry", &["--format", "json"]));
    answer["situation"].as_str().expect("situation").to_string()
}

#[test]
fn pause_resume_render() {
    let p = twin();
    let paused = payload(&prompt(&p, "pause_work", &[], "claude-code"));
    assert_eq!(paused["payload"]["steps"][0]["id"], "pause");
    assert_eq!(paused["payload"]["steps"][0]["cli"], "specforge gsd pause");
    let text = text_of(&paused);
    assert!(text.contains("--by"), "{text}");
    assert!(text.contains("`handoff` entity"), "{text}");
    assert!(!text.contains("HANDOFF.json"), "{text}");
    let resumed = payload(&prompt(&p, "resume_work", &[], "claude-code"));
    assert_eq!(resumed["payload"]["steps"][0]["id"], "resume");
    assert_eq!(
        resumed["payload"]["steps"][0]["cli"],
        "specforge gsd resume"
    );
    let text = text_of(&resumed);
    assert!(text.contains("`handoff` entity"), "{text}");
    assert!(!text.contains("HANDOFF.json"), "{text}");
}

#[test]
fn resume_work_names_the_active_workstream() {
    let p = project("workstreams");
    let said = |p: &Project| text_of(&payload(&prompt(p, "resume_work", &[], "claude-code")));
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
fn pause_then_resume_through_the_rendered_lines() {
    let p = twin();
    let before = situation(&p);
    assert_ne!(before, "paused");
    let text = text_of(&payload(&prompt(&p, "pause_work", &[], "claude-code")));
    let fill = [
        ("$MILESTONE", "v1.0"),
        ("{context-name}", "Save and load"),
        ("{X}", "1"),
        ("{Y}", "2"),
        ("{phase_number}", "02"),
        ("{where exactly are we}", "mid-edit"),
        ("{specific first action when resuming}", "run the tests"),
        (
            "{mental state, approach, what you were thinking}",
            "trim first",
        ),
        ("[reason]", "lunch"),
        ("[who]", "ada"),
        ("[phase]", "02"),
    ];
    let handoff = command(&text, "specforge new handoff", "", &fill);
    let refs: Vec<&str> = handoff.iter().map(String::as_str).collect();
    let made = p.run(&refs);
    assert_eq!(made.code, 0, "{handoff:?}: {}", made.all());
    // the pause line ends with the commit: the host command is what runs here
    let line = text
        .lines()
        .find(|l| l.starts_with("specforge gsd pause"))
        .and_then(|l| l.split(" && ").next())
        .expect("a pause line");
    let words = super::prompts_support::words(
        &line
            .replace("$MILESTONE", "v1.0")
            .replace("[reason]", "lunch")
            .replace("[who]", "ada")
            .replace("[phase]", "02"),
    );
    let refs: Vec<&str> = words.iter().skip(1).map(String::as_str).collect();
    let paused = p.run(&refs);
    assert_eq!(paused.code, 0, "{words:?}: {}", paused.all());
    assert_eq!(situation(&p), "paused");
    let id = paused
        .stdout
        .split("paused: ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .expect("a pause id")
        .to_string();
    let resumed = gsd(&p, "resume", &[&id]);
    assert_eq!(resumed.code, 0, "{}", resumed.all());
    assert_ne!(situation(&p), "paused");
    assert!(p.nodes_of("pause").is_empty());
    assert!(p.nodes_of("handoff").is_empty());
}

#[test]
fn pause_work_report_names_the_session_report_by_reference() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "pause_work",
        &["--report"],
        "claude-code",
    )));
    assert!(text.contains("--report"), "{text}");
    assert!(
        text.contains("specforge gsd reference gsd-core/workflows/session-report.md"),
        "{text}"
    );
    let answer = gsd(&p, "reference", &["gsd-core/workflows/session-report.md"]);
    assert_eq!(answer.code, 0, "{}", answer.all());
}

#[test]
fn pause_work_renders_for_three_hosts() {
    render_three_hosts(
        &twin(),
        "pause_work",
        &["--report"],
        "--report",
        "<objective>",
    );
}

#[test]
fn resume_work_renders_for_three_hosts() {
    render_three_hosts(&twin(), "resume_work", &[], "", "<objective>");
}
