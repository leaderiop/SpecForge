//! `milestone_summary` and `extract_learnings` (T8): the summary is shown, and the learnings are `lesson` entities
//! that the host accepts.

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of, words};

#[test]
fn summary_renders() {
    let p = twin();
    let rendered = payload(&prompt(
        &p,
        "milestone_summary",
        &["--milestone", "v1.0"],
        "claude-code",
    ));
    assert_eq!(rendered["payload"]["steps"][0]["id"], "report");
    assert_eq!(
        rendered["payload"]["steps"][0]["cli"],
        "specforge gsd progress-report --host claude-code"
    );
    let text = text_of(&rendered);
    for word in [
        "--milestone v1.0",
        "specforge query \"v${VERSION}\" --depth 1 --kinds phase --format context",
        "never written",
    ] {
        assert!(text.contains(word), "no '{word}'");
    }
}

#[test]
fn summary_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(
        &p,
        "milestone_summary",
        &["--milestone", "v1.0"],
        "--milestone v1.0",
        "milestone summary",
    );
}

#[test]
fn summary_refuses_a_missing_milestone() {
    let p = twin();
    let run = prompt(
        &p,
        "milestone_summary",
        &["--milestone", "v9.9"],
        "claude-code",
    );
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
}

#[test]
fn extract_learnings_renders_for_three_hosts() {
    let p = twin();
    for (host, rendered) in
        render_three_hosts(&p, "extract_learnings", &["v1.0/02"], "02", "Learnings")
    {
        let text = text_of(&rendered);
        for word in [
            "specforge new lesson",
            "--kind",
            "L-${PHASE_NUMBER}-1",
            "spec/lessons.spec",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
    }
}

#[test]
fn extract_learnings_refuses_a_missing_phase() {
    let p = twin();
    let run = prompt(&p, "extract_learnings", &["v1.0/99"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
}

/// The command the text shows (its continuation lines joined), with the placeholders filled, is accepted by the host
/// and the lesson it writes checks clean.
#[test]
fn a_lesson_written_as_the_text_says_checks_clean() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "extract_learnings",
        &["v1.0/02"],
        "claude-code",
    )));
    let mut lines = text
        .lines()
        .skip_while(|l| !l.starts_with("specforge new lesson"));
    let mut joined = String::new();
    for l in lines.by_ref() {
        let more = l.ends_with('\\');
        joined.push_str(l.trim_end_matches('\\'));
        joined.push(' ');
        if !more {
            break;
        }
    }
    let line = joined
        .replace("${PHASE_NUMBER}", "02")
        .replace("$MILESTONE", "v1.0")
        .replace("<short title>", "Keep it small")
        .replace(
            "<what was decided, why it was decided, Source: artifact>",
            "Keep it small. Source: 02-01-SUMMARY",
        );
    let args: Vec<String> = words(&line).into_iter().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let made = p.run(&args);
    assert_eq!(made.code, 0, "{}", made.all());
    assert!(p.read("spec/lessons.spec").contains("L-02-1"));
    // the twin has verdict errors of its own: the lesson is the one thing the check must not name
    let check = p.run(&["check"]);
    assert!(!check.all().contains("L-02-1"), "{}", check.all());
}
