//! The prompt an E516 refusal names, run on the subject it names, is not refused by that verdict: for a missing
//! verdict (the plan checks of a draft plan) and for a stale one held in another phase, each with one holder, on the
//! taskboard twin under gsd-strict. Several holders are pinned by the unit tests of the suggestion.

use super::commands_support::{strict, twin};
use super::harness::Project;
use super::prompts_gates::{PHASE, gate_failed, not_refused_by, refused_by, renders};
use super::prompts_support::prompt;

/// The phase before `PHASE`, whose verifier verdicts the twin carries fresh.
const VERIFIED: &str = "v1.0/01";
/// The words that open the tail naming another holder of the same judge's owed verdicts.
const TAIL: &str = "also owed on";

/// The `(prompt id, subject)` of every `specforge prompt gsd.<id> <subject>` a suggestion names.
fn named(suggestion: &str) -> Vec<(String, String)> {
    suggestion
        .split("`specforge prompt gsd.")
        .skip(1)
        .map(|rest| {
            let form = rest.split('`').next().unwrap_or_default();
            let (id, subject) = form
                .split_once(' ')
                .unwrap_or_else(|| panic!("a prompt and its subject: {suggestion}"));
            (id.to_string(), subject.to_string())
        })
        .collect()
}

/// The `(message, suggestion)` of each gsd/E516 diagnostic of the refusal of prompt `id`.
fn e516_suggestions(p: &Project, id: &str, args: &[&str]) -> Vec<(String, String)> {
    let found = gate_failed(&prompt(p, id, args, "claude-code"));
    found["data"]["failing"]
        .as_array()
        .expect("failing")
        .iter()
        .filter(|d| d["code"] == "gsd/E516")
        .map(|d| {
            let text = |key: &str| {
                d[key]
                    .as_str()
                    .unwrap_or_else(|| panic!("{key}: {d}"))
                    .to_string()
            };
            (text("message"), text("suggestion"))
        })
        .collect()
}

/// The phase that scopes an entity id (`v1.0/02/02-01` and `v1.0/02/02-01/TR1` are in `v1.0/02`).
fn phase_of(entity: &str) -> String {
    entity.split('/').take(2).collect::<Vec<_>>().join("/")
}

/// The entity a message names (`the gsd/W311 verdict on 'v1.0/02/02-01' is missing`).
fn entity_of(message: &str) -> &str {
    message
        .split('\'')
        .nth(1)
        .unwrap_or_else(|| panic!("an entity: {message}"))
}

#[test]
fn a_missing_plan_check_names_the_prompts_that_run_the_checker_on_its_phase() {
    let p = twin();
    // gsd-compat (no settings): the verdict gate is off, so the missing plan checks refuse nothing.
    not_refused_by(&p, "execute_phase", &[PHASE], "E516");
    strict(&p);
    refused_by(&p, "execute_phase", &[PHASE], "E516");
    let found = e516_suggestions(&p, "execute_phase", &[PHASE]);
    assert_eq!(found.len(), 4, "{found:?}");
    for (message, suggestion) in &found {
        assert!(message.ends_with("is missing"), "{message}");
        assert!(!suggestion.contains(TAIL), "one holder: {suggestion}");
        let named = named(suggestion);
        let held = phase_of(entity_of(message));
        assert_eq!(
            named.first(),
            Some(&("plan_phase".to_string(), held.clone())),
            "{suggestion}"
        );
        assert!(
            suggestion.starts_with(&format!("run /gsd-plan-phase {held} ")),
            "{suggestion}"
        );
        // A named prompt may be refused by a gate of its own unrelated to this verdict, so each is shown not to be
        // refused by E516 rather than to render; the first, the planning prompt, renders.
        for (id, subject) in &named {
            assert_eq!(subject, &held, "{suggestion}");
            not_refused_by(&p, id, &[subject], "E516");
        }
    }
    renders(&p, "plan_phase", &[PHASE]);
}

#[test]
fn a_stale_verdict_in_another_phase_names_that_phases_prompt() {
    let p = twin();
    strict(&p);
    // plan_phase runs the plan checker, so the twin's missing plan checks are excused: only a stale verifier verdict
    // can refuse it.
    renders(&p, "plan_phase", &[PHASE]);
    p.write(
        "src/tasks.js",
        &format!("{}\n// edited\n", p.read("src/tasks.js")),
    );
    refused_by(&p, "plan_phase", &[PHASE], "E516");
    let found = e516_suggestions(&p, "plan_phase", &[PHASE]);
    assert!(!found.is_empty());
    for (message, suggestion) in &found {
        let held = phase_of(entity_of(message));
        assert_eq!(held, VERIFIED, "{message}");
        assert!(!suggestion.contains(TAIL), "one holder: {suggestion}");
        assert_eq!(
            named(suggestion),
            [("execute_phase".to_string(), held.clone())],
            "{suggestion}"
        );
        assert!(
            suggestion.contains(&format!("run /gsd-execute-phase {held} ")),
            "{suggestion}"
        );
        assert!(
            suggestion.contains("specforge gsd re-verify"),
            "{suggestion}"
        );
        assert!(!suggestion.contains(PHASE), "the gated phase: {suggestion}");
    }
    renders(&p, "execute_phase", &[VERIFIED]);
    assert!(p.verify_phase(VERIFIED) > 0);
    renders(&p, "plan_phase", &[PHASE]);
}
