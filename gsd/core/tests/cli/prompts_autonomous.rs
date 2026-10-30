//! `autonomous` (T3): an agent-followed workflow. Its unattended settings are declared and inert, and its bounds are
//! optional references that must name a phase.

use serde_json::json;

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

#[test]
fn autonomous_renders() {
    let out = render_three_hosts(
        &twin(),
        "autonomous",
        &["--from", "01", "--to", "02", "--converge"],
        "--from 01 --to 02 --converge",
        "<purpose>",
    );
    let steps = out[0].1["payload"]["steps"].as_array().expect("steps");
    let prompts: Vec<&str> = steps.iter().filter_map(|s| s["prompt"].as_str()).collect();
    assert_eq!(
        prompts,
        [
            "specforge.gsd.discuss_phase",
            "specforge.gsd.plan_phase",
            "specforge.gsd.plan_review_convergence",
            "specforge.gsd.execute_phase",
            "specforge.gsd.verify_work",
            "specforge.gsd.audit_milestone",
            "specforge.gsd.complete_milestone",
        ]
    );
    assert_eq!(steps[7]["id"], "cleanup");
    assert_eq!(steps[7]["cli"], "specforge gsd cleanup");
}

#[test]
fn autonomous_says_its_unattended_settings_are_inert() {
    let p = twin();
    p.settings(json!({ "claude_orchestration.enabled": true }));
    let text = text_of(&payload(&prompt(&p, "autonomous", &[], "claude-code")));
    assert!(text.contains("claude_orchestration.enabled"), "{text}");
    assert!(text.contains("declared settings with no effect"), "{text}");
    assert!(
        text.contains("`specforged`) is not part of `@gsd/core`"),
        "{text}"
    );
}

/// Probe P3: an optional reference is resolved when it is given.
#[test]
fn autonomous_refuses_an_unknown_from_phase() {
    let run = prompt(&twin(), "autonomous", &["--from", "09"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
    assert!(run.all().contains("unresolved entity"), "{}", run.all());
}

#[test]
fn autonomous_names_code_review_and_the_ui_gate_as_not_installed() {
    let text = text_of(&payload(&prompt(&twin(), "autonomous", &[], "claude-code")));
    assert!(text.contains("/gsd-code-review"), "{text}");
    assert!(text.contains("ui-plan-gate"), "{text}");
    assert!(text.contains("not installed"), "{text}");
}

/// REVIEW-r43 Minor 3: the skip-discuss `specforge new context` carries `canonical_refs` in the same line (the entity is
/// refused without them), so the line is accepted as written.
#[test]
fn the_skip_discuss_context_line_is_accepted() {
    let p = twin();
    let text = text_of(&payload(&prompt(&p, "autonomous", &[], "claude-code")));
    let start = text.find("`specforge new context ").expect("the line") + 1;
    let line = &text[start..start + text[start..].find('`').expect("an end")];
    let line = line
        .replace("${padded_phase}", "09")
        .replace("{Phase Name}", "Docs")
        .replace("${PHASE_NUM}", "01")
        .replace("$MILESTONE", "v1.0")
        .replace("{date}", "2026-10-30")
        .replace("{goal}", "docs");
    let words = shell_split(&line);
    let mut args: Vec<&str> = words.iter().skip(1).map(String::as_str).collect();
    args.push("--dry-run");
    let run = p.run(&args);
    assert_eq!(run.code, 0, "{line}\n{}", run.all());
}

/// The words of a shell line (quotes honoured and dropped).
fn shell_split(line: &str) -> Vec<String> {
    let (mut out, mut word, mut any) = (Vec::new(), String::new(), false);
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                any = true;
            }
            (None, c) if c.is_whitespace() => {
                if any || !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            (None, c) => word.push(c),
        }
    }
    if any || !word.is_empty() {
        out.push(word);
    }
    out
}
