//! The content verdicts of gsd-core's `gate-cutover` fixtures that a core rule answers (D-r44-22), against the twin
//! that rule runs on: what upstream blocked, the twin reports; what upstream let through, the twin leaves alone. The
//! plan-time rows are `parity_plans.rs`; these are the schema-drift, prohibition, TDD, context-drift and decision rows.
//! The table that classifies all 171 verdicts is `tests/parity_rules/cutover.rs`.

use serde_json::{Value, json};

use super::execute_rules::{T0, T1, junit, repository, tdd, touch_schema};
use super::harness::{Project, project};
use super::parity_plans::{diagnostics, e310_ids, printed, without_decision};
use super::prompts_gates::{
    PHASE, refused_by, renders, research_against_a_stale_context, research_in_step_with_its_context,
};

/// The messages of the `gsd/<code>` diagnostics of `check`.
fn messages(p: &Project, code: &str) -> Vec<String> {
    diagnostics(p)
        .into_iter()
        .filter(|(c, ..)| c == &format!("gsd/{code}"))
        .map(|(.., m)| m)
        .collect()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("a list")
        .iter()
        .map(|v| v.as_str().expect("a string").to_string())
        .collect()
}

/// A summary that ran a database push: the evidence the schema gate asks for.
fn pushed(p: &Project) {
    p.edit(
        "spec/plans.spec",
        "one_liner              \"An in-memory task store with add and list\"",
        "one_liner              \"An in-memory task store; ran npx prisma db push\"",
    );
}

#[test]
fn gate_cutover_schema_drift_verdicts_reproduce() {
    // Blocked: a schema file changed, no push ran. E405 names the file, the ORM and its push command.
    let blocked = printed("verify-schema-drift-blocks");
    assert_eq!(blocked["blocking"], true);
    let p = project("execute");
    touch_schema(&p);
    let found = messages(&p, "E405");
    assert_eq!(found.len(), 1, "{found:?}");
    for file in strings(&blocked["schema_files"]) {
        assert!(found[0].contains(&file), "{file} in {found:?}");
    }
    for orm in strings(&blocked["orms"]) {
        assert!(found[0].contains(&orm), "{orm} in {found:?}");
    }
    let push = blocked["message"]
        .as_str()
        .expect("a message")
        .lines()
        .find_map(|l| l.trim().strip_prefix("prisma: "))
        .and_then(|l| l.split(" (").next())
        .expect("the push command")
        .to_string();
    assert!(
        p.run(&["check", "--format", "json"]).stdout.contains(&push),
        "{push}"
    );

    // The surface verb says the same thing, and the environment cannot bypass it: nothing in SpecForge reads one.
    for name in [
        "verify-surface-schema-drift-blocks",
        "verify-surface-schema-drift-env-ignored",
        "verify-schema-drift-env-not-true",
    ] {
        assert_eq!(printed(name)["blocking"], true, "{name}");
    }
    for value in ["true", "1"] {
        let run = p.run_with(
            &["check", "--format", "json"],
            &[("GSD_SKIP_SCHEMA_CHECK", value)],
        );
        assert!(run.stdout.contains("gsd/E405"), "{value}: {}", run.all());
    }

    // Upstream's environment bypass is SpecForge's setting: with the gate off nothing is reported.
    assert_eq!(printed("verify-schema-drift-env-skip")["blocking"], false);
    let p = project("execute");
    touch_schema(&p);
    p.settings(json!({ "workflow.schema_drift_gate": false }));
    assert!(messages(&p, "E405").is_empty());

    // Clean: no schema file in the plan.
    let clean = printed("verify-schema-drift-clean");
    assert_eq!(
        (clean["blocking"].clone(), clean["schema_files"].clone()),
        (false.into(), json!([]))
    );
    assert!(messages(&project("execute"), "E405").is_empty());

    // Pushed: the schema file changed and a summary shows the push.
    let done = printed("verify-schema-drift-pushed");
    assert_eq!(
        (done["blocking"].clone(), done["unpushed_orms"].clone()),
        (false.into(), json!([]))
    );
    assert_eq!(strings(&done["schema_files"]), ["prisma/schema.prisma"]);
    let p = project("execute");
    touch_schema(&p);
    pushed(&p);
    assert!(messages(&p, "E405").is_empty());
}

#[test]
fn gate_cutover_prohibition_verdicts_reproduce() {
    // A prohibition verified by a test that names none is flagged unverified and not located.
    for name in [
        "prohibition-enforcement-json-flag",
        "prohibition-enforcement-request-file-located-false",
    ] {
        let up = printed(name);
        assert_eq!(
            (
                up["status"].clone(),
                up["flagged"].clone(),
                up["tier"].clone(),
                up["located"].clone()
            ),
            (
                "unverified".into(),
                true.into(),
                "test".into(),
                false.into()
            ),
            "{name}"
        );
    }
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "verification judgment",
        "verification test",
    );
    let found = messages(&p, "E319");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("is verified by a test but names none"),
        "{found:?}"
    );
    // Naming the test is the located check.
    p.edit(
        "spec/plans.spec",
        "verification test",
        "verification test\n  test         \"node --test test/store.test.js\"",
    );
    assert!(messages(&p, "E319").is_empty());
}

#[test]
fn gate_cutover_tdd_verdicts_reproduce() {
    // A RED commit, then a GREEN one: upstream passes the plan, W403 stays silent.
    let pass = printed("tdd-review-checkpoint-pass");
    assert_eq!(
        (
            pass["violations"].clone(),
            pass["rows"][0]["status"].clone()
        ),
        (0.into(), "Pass".into())
    );
    let p = repository();
    p.settings(json!({ "workflow.tdd_mode": true }));
    tdd(&p);
    p.git_commit(
        "test(01-01): Store module",
        &[("test/tasks.test.js", "t\n")],
        T0,
    );
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T1);
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    junit(&p, &[("adds a task", true)]);
    assert!(messages(&p, "W403").is_empty());

    // A phase with no TDD plan has nothing to review.
    let none = printed("tdd-review-checkpoint-no-tdd-plans");
    assert_eq!(
        (none["tddPlans"].clone(), none["block"].clone()),
        (0.into(), false.into())
    );
    let p = repository();
    p.settings(json!({ "workflow.tdd_mode": true }));
    p.git_commit("feat(01-01): Store module", &[("src/tasks.js", "a\n")], T1);
    assert!(messages(&p, "W403").is_empty());
}

#[test]
fn gate_cutover_context_drift_verdicts_reproduce() {
    // Decisions newer than the research: stale. Upstream warns by default and blocks when told to.
    let warns = printed("verify-context-drift-stale-warns");
    let blocks = printed("verify-context-drift-stale-blocks");
    assert_eq!(
        (
            warns["block"].clone(),
            warns["action"].clone(),
            blocks["block"].clone(),
            blocks["action"].clone()
        ),
        (false.into(), "warn".into(), true.into(), "block".into())
    );
    for name in [
        "verify-surface-context-drift-stale-warns",
        "verify-surface-context-drift-stale-blocks",
    ] {
        assert_eq!(
            printed(name)["stale_artifacts"],
            warns["stale_artifacts"],
            "{name}"
        );
    }
    let p = project("discuss");
    research_against_a_stale_context(&p);
    let found = messages(&p, "W406");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("02-RESEARCH"),
        "the stale artifact: {found:?}"
    );
    for id in ["plan_phase", "plan_review_convergence", "ultraplan_phase"] {
        renders(&p, id, &[PHASE]);
    }
    p.settings(json!({ "workflow.context_drift_action": "block" }));
    for id in ["plan_phase", "plan_review_convergence", "ultraplan_phase"] {
        refused_by(&p, id, &[PHASE], "W406");
    }

    // In step: research that digests the context as it is, is not stale.
    let sync = printed("verify-context-drift-in-sync");
    assert_eq!(
        (sync["block"].clone(), sync["stale_artifacts"].clone()),
        (false.into(), json!([]))
    );
    let p = project("discuss");
    research_in_step_with_its_context(&p);
    assert!(messages(&p, "W406").is_empty());

    // Nothing recorded to go stale: a phase whose research digests no context.
    let bare = printed("verify-context-drift-no-upstream-artifacts");
    assert_eq!(
        (bare["skipped"].clone(), bare["reason"].clone()),
        (true.into(), "no-upstream-artifacts".into())
    );
    let p = project("discuss");
    assert!(messages(&p, "W406").is_empty());
}

#[test]
fn gate_cutover_decision_verdicts_reproduce() {
    // No CONTEXT: nothing to check, whatever the plans cite.
    let absent = printed("decision-coverage-plan-context-absent");
    assert_eq!(
        (absent["passed"].clone(), absent["total"].clone()),
        (true.into(), 0.into())
    );
    let p = project("gate_cutover");
    for name in ["D-03", "D-02", "D-01"] {
        without_decision(&p, name);
    }
    assert!(e310_ids(&p).is_empty());
}
