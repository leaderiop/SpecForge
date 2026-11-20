//! The plan flow under gsd-strict on the taskboard twin: a planning prompt renders with its thread step before its
//! check, `execute_phase` waits for the plan checks the twin does not carry, and renders once their verdicts are
//! recorded.

use super::commands_support::{strict, twin};
use super::e516_remedy::e516_suggestions;
use super::harness::Project;
use super::prompts_gates::{PHASE, failing, not_refused_by, refused_by, renders};
use super::prompts_support::{payload, prompt};
use super::thread_plan::{thread, trace};

/// The twin's executed phase: its plans' checks are history and owe no verdict.
const EXECUTED: &str = "v1.0/01";
/// The open plan of [`PHASE`], whose plan checks `execute_phase` gates on.
const PLAN: &str = "v1.0/02/02-01";
/// The rules the plan checker judges on an open plan.
const PLAN_CHECKS: [&str; 4] = ["W311", "W313", "W315", "W316"];
/// The steps of `plan_phase`, in order: the thread runs before the check.
const STEPS: [&str; 8] = [
    "research",
    "record_research",
    "patterns",
    "plan",
    "thread",
    "check",
    "record_verdict",
    "execute",
];

/// The twin as committed, under gsd-strict. It holds only verifier verdicts (gsd/E501 on phase 01), so nothing is
/// stripped: the missing plan checks of [`PLAN`] are its starting state, and recording them is the remedy.
fn strict_twin() -> Project {
    let t = twin();
    strict(&t);
    t
}

#[test]
fn plan_phase_renders_under_strict_with_the_thread_before_the_check() {
    for phase in [EXECUTED, PHASE] {
        let t = strict_twin();
        let rendered = payload(&prompt(&t, "plan_phase", &[phase], "claude-code"));
        let steps: Vec<&str> = rendered["payload"]["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .map(|s| s["id"].as_str().expect("an id"))
            .collect();
        assert_eq!(steps, STEPS, "{phase}");
        assert_eq!(rendered["payload"]["subject"], phase);
        // The thread the step feeds to `specforge trace --plan` passes on the twin as committed.
        let (run, outcome) = trace(&t, &thread(&t, phase));
        assert_eq!(run.code, 0, "{phase}: {}", run.all());
        assert_eq!(outcome["ok"], true, "{phase}: {outcome}");
    }
}

#[test]
fn execute_phase_is_refused_under_strict_until_the_plan_checks_are_recorded() {
    let t = strict_twin();
    refused_by(&t, "execute_phase", &[PHASE], "E516");
    let mut codes = failing(&t, "execute_phase", &[PHASE]);
    codes.sort();
    assert_eq!(
        codes,
        [
            "gsd/E516", "gsd/E516", "gsd/E516", "gsd/E516", "gsd/W311", "gsd/W313", "gsd/W315"
        ]
    );
    let found = e516_suggestions(&t, "execute_phase", &[PHASE]);
    let messages: Vec<&str> = found.iter().map(|(message, _)| message.as_str()).collect();
    for rule in PLAN_CHECKS {
        let missing = format!("the gsd/{rule} verdict on '{PLAN}' is missing");
        let naming: Vec<&&str> = messages.iter().filter(|m| m.ends_with(&missing)).collect();
        assert_eq!(naming.len(), 1, "{rule}: {messages:?}");
    }
    // The same state lets the executed phase run and the open phase be planned again.
    renders(&t, "execute_phase", &[EXECUTED]);
    renders(&t, "plan_phase", &[PHASE]);
    // Under gsd-compat the verdict gate is off: strict is what refuses it.
    not_refused_by(&twin(), "execute_phase", &[PHASE], "E516");
}

#[test]
fn execute_phase_renders_once_the_plan_check_verdicts_are_recorded() {
    let t = strict_twin();
    for rule in PLAN_CHECKS {
        let run = t.record(PLAN, rule, "passed", &[]);
        assert_eq!(run.code, 0, "{rule}: {}", run.all());
    }
    renders(&t, "execute_phase", &[PHASE]);
}
