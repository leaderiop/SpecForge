//! The prompt an E516 refusal names, run on the subject it names, is not refused by that verdict and runs the judge
//! that records it again: for a missing verdict (the plan checks of a draft plan), for a stale one held in another
//! phase, and for two judges' verdicts held in two phases, where each named run is refused by the other's verdicts and
//! the refusal names the entities that owe them and the gate-free `record-verdict`, on the taskboard twin under
//! gsd-strict; and, on the discuss project, a verdict no phase or milestone holds, named by its own entity and
//! recorded without a gate.

use serde_json::Value;

use super::commands_support::{strict, twin};
use super::harness::{Project, project};
use super::prompts_gates::{
    PHASE, gate_failed, judge_discussion, not_refused_by, refused_by, renders, stale_plan_check,
};
use super::prompts_support::{payload, prompt};

/// The phase before `PHASE`, whose verifier verdicts the twin carries fresh.
const VERIFIED: &str = "v1.0/01";
/// The open plan of [`PHASE`].
const PLAN: &str = "v1.0/02/02-01";
/// The rules the plan checker judges on an open plan.
const PLAN_CHECKS: [&str; 4] = ["W311", "W313", "W315", "W316"];
/// The words that open every tail naming the verdicts that refuse every named run.
const TAIL: &str = "is also refused by the verdicts owed";

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
pub(super) fn e516_suggestions(p: &Project, id: &str, args: &[&str]) -> Vec<(String, String)> {
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

/// Prompt `id` on `subject` renders a step that runs `judge`: a run of it records that judge's verdict again.
fn rejudges(p: &Project, id: &str, subject: &str, judge: &str) {
    let rendered = payload(&prompt(p, id, &[subject], "claude-code"));
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    assert!(
        steps.iter().any(|s| s["agent"] == judge),
        "{id} {subject} runs no {judge}: {steps:?}"
    );
}

/// The phase that scopes an entity id (`v1.0/02/02-01` and `v1.0/02/02-01/TR1` are in `v1.0/02`).
fn phase_of(entity: &str) -> String {
    entity.split('/').take(2).collect::<Vec<_>>().join("/")
}

/// The entities whose verifier verdicts a `re-verify --stale-only` `answer` lists, each once, in its order, every item
/// `stale`.
fn stale_listed(answer: &Value) -> Vec<String> {
    let mut entities: Vec<String> = Vec::new();
    for item in answer["items"].as_array().expect("re-verify lists items") {
        assert_eq!(item["state"], "stale", "{item}");
        let entity = item["entity"].as_str().expect("an entity").to_string();
        if !entities.contains(&entity) {
            entities.push(entity);
        }
    }
    entities
}

/// How many entities a tail group names before it counts the rest.
const NAMED: usize = 5;

/// The entities a tail group names, each quoted as the message quotes its own: the first five, then how many more.
fn quoted(entities: &[String]) -> String {
    let names = entities
        .iter()
        .take(NAMED)
        .map(|e| format!("'{e}'"))
        .collect::<Vec<_>>()
        .join(", ");
    match entities.len().saturating_sub(NAMED) {
        0 => names,
        n => format!("{names}, and {n} more"),
    }
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
        assert_eq!(
            named.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            ["plan_phase", "plan_review_convergence"],
            "{suggestion}"
        );
        // A named prompt may be refused by a gate of its own unrelated to this verdict, so each is shown not to be
        // refused by E516 and to run the checker; the first, the planning prompt, renders.
        for (id, subject) in &named {
            assert_eq!(subject, &held, "{suggestion}");
            not_refused_by(&p, id, &[subject], "E516");
        }
    }
    rejudges(&p, "plan_phase", PHASE, "gsd-plan-checker");
    // The check step records the four verdicts: the refusal is gone.
    for rule in PLAN_CHECKS {
        let run = p.record(PLAN, rule, "passed", &[]);
        assert_eq!(run.code, 0, "{rule}: {}", run.all());
    }
    renders(&p, "execute_phase", &[PHASE]);
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
    rejudges(&p, "execute_phase", VERIFIED, "gsd-verifier");
    // The verify step records the verifier's verdicts again: the refusal is gone.
    assert!(p.verify_phase(VERIFIED) > 0);
    renders(&p, "plan_phase", &[PHASE]);
}

/// A stale verifier verdict in the executed phase and stale plan checks of the open plan in the next: the run each
/// refusal names is refused by the other's verdicts, so each names them and the gate-free `record-verdict`, which
/// clears both.
#[test]
fn verdicts_of_two_judges_in_two_phases_name_each_other_and_the_gate_free_route() {
    let p = twin();
    strict(&p);
    stale_plan_check(&p, PLAN);
    p.write(
        "src/tasks.js",
        &format!("{}\n// edited\n", p.read("src/tasks.js")),
    );
    // plan_phase on the open phase excuses its plan checks; the verifier verdicts of the executed phase refuse it.
    let found = e516_suggestions(&p, "plan_phase", &[PHASE]);
    assert!(!found.is_empty());
    for (message, suggestion) in &found {
        assert_eq!(phase_of(entity_of(message)), VERIFIED, "{message}");
        assert_eq!(
            named(suggestion),
            [("execute_phase".to_string(), VERIFIED.to_string())],
            "{suggestion}"
        );
        // The open plan owes four plan checks and is named once.
        assert!(
            suggestion.contains(&format!("{TAIL} on {PHASE} (gsd-plan-checker: '{PLAN}')")),
            "{suggestion}"
        );
        assert_eq!(
            suggestion.matches(&format!("'{PLAN}'")).count(),
            1,
            "{suggestion}"
        );
        assert!(
            suggestion.contains("specforge gsd record-verdict"),
            "{suggestion}"
        );
    }
    // execute_phase on the executed phase excuses its verifier verdicts; the open plan's checks refuse it. The
    // stale must-haves the edit left in the executed phase are those re-verify lists.
    let stale = stale_listed(&p.re_verify(VERIFIED, true));
    assert_eq!(
        stale,
        [
            "v1.0/01/01-01/AR1",
            "v1.0/01/01-01/KL1",
            "v1.0/01/01-01/TR1",
            "v1.0/01/01-02/KL1",
            "v1.0/01/01-02/TR1",
            "v1.0/01/01-02/TR2",
        ]
    );
    let owed = format!("{TAIL} on {VERIFIED} (gsd-verifier: {})", quoted(&stale));
    assert_eq!(
        owed,
        format!(
            "{TAIL} on v1.0/01 (gsd-verifier: 'v1.0/01/01-01/AR1', 'v1.0/01/01-01/KL1', 'v1.0/01/01-01/TR1', \
             'v1.0/01/01-02/KL1', 'v1.0/01/01-02/TR1', and 1 more)"
        )
    );
    let found = e516_suggestions(&p, "execute_phase", &[VERIFIED]);
    assert_eq!(found.len(), PLAN_CHECKS.len(), "{found:?}");
    for (message, suggestion) in &found {
        assert_eq!(entity_of(message), PLAN, "{message}");
        assert_eq!(
            named(suggestion)
                .iter()
                .map(|(id, at)| (id.as_str(), at.as_str()))
                .collect::<Vec<_>>(),
            [("plan_phase", PHASE), ("plan_review_convergence", PHASE)],
            "{suggestion}"
        );
        assert!(suggestion.contains(&owed), "{suggestion}");
        assert!(
            suggestion.contains(&format!("'{VERIFIED}/01-01/TR1'")),
            "{suggestion}"
        );
    }
    // The gate-free route: record both judges' verdicts, and both runs render.
    for rule in PLAN_CHECKS {
        let run = p.record(PLAN, rule, "passed", &[]);
        assert_eq!(run.code, 0, "{rule}: {}", run.all());
    }
    assert!(p.verify_phase(VERIFIED) > 0);
    renders(&p, "execute_phase", &[VERIFIED]);
    renders(&p, "plan_phase", &[PHASE]);
}

/// The seven must-haves of a stale phase are named five and counted two: with `src/tasks.js` and `src/cli.js` edited,
/// each refusal of `/gsd-execute-phase` on the executed phase names the first five that `re-verify --stale-only` lists
/// beside their holder and judge, then `and 2 more`.
#[test]
fn the_seven_must_haves_of_a_stale_phase_are_named_five_and_two_more() {
    let p = twin();
    strict(&p);
    stale_plan_check(&p, PLAN);
    for path in ["src/tasks.js", "src/cli.js"] {
        p.write(path, &format!("{}\n// edited\n", p.read(path)));
    }
    let stale = stale_listed(&p.re_verify(VERIFIED, true));
    assert_eq!(
        stale,
        [
            "v1.0/01/01-01/AR1",
            "v1.0/01/01-01/KL1",
            "v1.0/01/01-01/TR1",
            "v1.0/01/01-02/AR1",
            "v1.0/01/01-02/KL1",
            "v1.0/01/01-02/TR1",
            "v1.0/01/01-02/TR2",
        ]
    );
    let owed = format!("{TAIL} on {VERIFIED} (gsd-verifier: {})", quoted(&stale));
    assert_eq!(
        owed,
        format!(
            "{TAIL} on v1.0/01 (gsd-verifier: 'v1.0/01/01-01/AR1', 'v1.0/01/01-01/KL1', 'v1.0/01/01-01/TR1', \
             'v1.0/01/01-02/AR1', 'v1.0/01/01-02/KL1', and 2 more)"
        )
    );
    let found = e516_suggestions(&p, "execute_phase", &[VERIFIED]);
    assert_eq!(found.len(), PLAN_CHECKS.len(), "{found:?}");
    for (message, suggestion) in &found {
        assert_eq!(entity_of(message), PLAN, "{message}");
        assert!(suggestion.contains(&owed), "{suggestion}");
    }
}

/// A package audit of the project's research, which no phase or milestone holds, and one of phase 02's research, both
/// stale after their packages are edited: the refusal of `/gsd-plan-review-convergence` on phase 02 (which runs no
/// researcher) names, in the phase audit's help, `/gsd-plan-phase` on the phase and the project audit by its own id,
/// owed by it itself, never `RESEARCH` as a holder; the project audit's own help names `record-verdict` with its id,
/// and recording it that way clears its refusal and the phase audit's tail.
#[test]
fn a_verdict_no_phase_holds_is_named_by_its_own_entity_and_recorded_without_a_gate() {
    let p = project("discuss");
    p.write(
        "spec/project_audit.spec",
        "package_audit PA2 \"node:path\" {\n  research RESEARCH\n  package  \"node:path\"\n  registry \"builtin\"\n}\n",
    );
    let audits: Vec<String> = p
        .nodes_of("package_audit")
        .iter()
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect();
    let (phase_audit, project_audit) = ("v1.0/02/02-RESEARCH/PA1", "RESEARCH/PA2");
    assert!(audits.iter().any(|id| id == phase_audit), "{audits:?}");
    assert!(audits.iter().any(|id| id == project_audit), "{audits:?}");
    strict(&p);
    judge_discussion(&p);
    renders(&p, "plan_review_convergence", &[PHASE]);
    p.edit(
        "spec/research.spec",
        "package  \"node:fs\"\n",
        "package  \"node:fs/promises\"\n",
    );
    p.edit(
        "spec/project_audit.spec",
        "package  \"node:path\"\n",
        "package  \"node:path/posix\"\n",
    );
    for audit in [phase_audit, project_audit] {
        assert_eq!(
            p.inputs(audit, "W205")["verdict"]["state"],
            "stale",
            "{audit}"
        );
    }
    let help = |found: &[(String, String)], entity: &str| -> String {
        found
            .iter()
            .find(|(message, _)| entity_of(message) == entity)
            .map(|(_, suggestion)| suggestion.clone())
            .unwrap_or_else(|| panic!("no E516 on {entity}: {found:?}"))
    };
    let found = e516_suggestions(&p, "plan_review_convergence", &[PHASE]);
    let phase_help = help(&found, phase_audit);
    assert!(
        phase_help.contains(&format!("/gsd-plan-phase {PHASE}")),
        "{phase_help}"
    );
    assert!(
        phase_help.contains(&format!(
            "{TAIL} by '{project_audit}' itself (gsd-phase-researcher), outside its scope"
        )),
        "{phase_help}"
    );
    assert_eq!(
        help(&found, project_audit),
        format!(
            "no prompt judges it again where no phase or milestone scopes it: record its verdict with `specforge gsd \
             record-verdict {project_audit} W205 <outcome> <model> '<basis>'` over its inputs (`specforge query \
             {project_audit} --inputs gsd/W205`), which no gate refuses (its judge: gsd-phase-researcher)"
        )
    );
    for (_, suggestion) in &found {
        assert!(!suggestion.contains("on RESEARCH"), "{suggestion}");
    }
    let run = p.record(project_audit, "W205", "legitimate", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    let found = e516_suggestions(&p, "plan_review_convergence", &[PHASE]);
    assert!(
        found
            .iter()
            .all(|(message, _)| !message.contains(&format!("'{project_audit}'"))),
        "{found:?}"
    );
    assert!(!help(&found, phase_audit).contains(TAIL), "{found:?}");
}
