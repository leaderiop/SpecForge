//! Judged verdicts over checked values (ADR 0337 §6): the outcome class of each checked value is digested, and a
//! check this machine did not perform leaves its input unjudged (I207).

use specforge_common::Diagnostic;
use specforge_effects::OutcomeClass;
use specforge_project::judged::Cited;
use specforge_project::testing::{
    GSD_JUDGED, GSD_JUDGED_SPEC, TestProject, gsd_judged_project, gsd_judged_runtime,
};
use specforge_project::{CheckedValue, CheckedValues, ValueOutcome};
use specforge_registry::PlainDisk;
use specforge_test::prelude::*;

use crate::judged::{PLAN, W311, compile, judgement, record_from};

fn found<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Vec<&'a Diagnostic> {
    diagnostics.iter().filter(|d| d.code == code).collect()
}

/// The first decision `W311` reads of the plan: the entity the checked values are given to.
fn decision(project: &TestProject) -> String {
    let compiled = compile(project);
    let rule = compiled
        .environment()
        .registries()
        .judged_rule(W311)
        .expect("W311");
    let now =
        specforge_project::judged::inputs(&compiled, &PlainDisk, PLAN, rule, &Cited::default())
            .expect("W311 judges the plan");
    let decisions = now
        .reports
        .iter()
        .find(|r| r.name == "decisions")
        .expect("an input");
    decisions.members[0].clone()
}

/// `id`'s one URL checked by `links` with `outcome`.
fn checked(id: &str, outcome: ValueOutcome) -> CheckedValues {
    let mut values = CheckedValues::default();
    values.insert(
        id,
        CheckedValue {
            field: "url".into(),
            item: 0,
            checker: "links".into(),
            outcome,
        },
    );
    values
}

const OK: ValueOutcome = ValueOutcome::Class(OutcomeClass::Ok);

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "an entity without checked values digests as it did before checked values"
)]
fn an_entity_without_checked_values_digests_as_before() {
    let project = gsd_judged_project("");
    let compiled = compile(&project);
    let rule = compiled
        .environment()
        .registries()
        .judged_rule(W311)
        .expect("W311");
    let now =
        specforge_project::judged::inputs(&compiled, &PlainDisk, PLAN, rule, &Cited::default())
            .expect("W311 judges the plan");
    let decisions = now
        .reports
        .iter()
        .find(|r| r.name == "decisions")
        .expect("an input");
    assert_eq!(decisions.digest, PINNED_DECISIONS);
    assert!(now.unchecked.is_empty());
}

/// The `decisions` digest of the fixture plan, taken from this code, which builds the same bytes as before checked
/// values existed (an entity with none digests as `[digest, id]`). That those bytes did not move is held by the
/// committed verdicts of `gsd/parity/fixtures/taskboard/spec/plans.spec`, which
/// `gsd/core/tests/cli/parity_verify.rs` (`the_twin_holds_its_verifier_verdicts`) still judges fresh.
const PINNED_DECISIONS: &str =
    "sha256:74a8804ab011abf737f0ef17c15ad3589617fa264c77c96e220836d1a1c4d75c";

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a checked value whose outcome class changes stales the verdict that read it"
)]
fn a_checked_value_turning_404_stales_the_verdict() {
    let project = gsd_judged_project("");
    let id = decision(&project);
    let ok = compile(&project).with_checked_values(checked(&id, OK));
    record_from(&project, &ok, PLAN, W311, "passed", &Cited::default(), "");
    let fresh = compile(&project).with_checked_values(checked(&id, OK));
    assert_eq!(judgement(&fresh, PLAN, W311).0, "fresh");
    let gone = ValueOutcome::Class(OutcomeClass::Status(404));
    let stale = compile(&project).with_checked_values(checked(&id, gone));
    let (state, changed) = judgement(&stale, PLAN, W311);
    assert_eq!(
        (state.as_str(), changed.as_slice()),
        ("stale", ["decisions".to_string()].as_slice())
    );
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a value rechecked later with the same outcome class stales nothing"
)]
fn the_same_outcome_class_rechecked_later_stays_fresh() {
    let project = gsd_judged_project("");
    let id = decision(&project);
    let ok = compile(&project).with_checked_values(checked(&id, OK));
    record_from(&project, &ok, PLAN, W311, "passed", &Cited::default(), "");
    // A later run observes the same class: when it was observed is not digested.
    let later = compile(&project).with_checked_values(checked(&id, OK));
    assert_eq!(judgement(&later, PLAN, W311).0, "fresh");
    assert!(found(&later.diagnostics(), "I207").is_empty());
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "an input with a value this machine did not check is skipped like a missing report, and reported as I207"
)]
fn an_unknown_outcome_is_skipped_like_a_missing_report_and_i207() {
    let project = gsd_judged_project("");
    let id = decision(&project);
    let ok = compile(&project).with_checked_values(checked(&id, OK));
    record_from(&project, &ok, PLAN, W311, "passed", &Cited::default(), "");
    let offline = compile(&project).with_checked_values(checked(&id, ValueOutcome::Unknown));
    assert_eq!(
        judgement(&offline, PLAN, W311).0,
        "fresh",
        "not stale, not red"
    );
    let diagnostics = offline.diagnostics();
    let i207 = found(&diagnostics, "I207");
    assert_eq!(i207.len(), 1, "{diagnostics:?}");
    assert!(i207[0].machine);
    let rule = offline
        .environment()
        .registries()
        .judged_rule(W311)
        .expect("W311");
    let now =
        specforge_project::judged::inputs(&offline, &PlainDisk, PLAN, rule, &Cited::default())
            .expect("W311 judges the plan");
    assert_eq!(now.unchecked, ["decisions"]);
    assert!(now.unjudged.contains(&"decisions".to_string()));
}

#[specforge_test(
    behavior = "digest_judged_inputs",
    verify = "a checked value's outcome is digested by its fixed tag, never by how it is shown"
)]
fn a_value_outcome_is_digested_by_its_fixed_tag() {
    use specforge_effects::{Redirected, Unreachable};
    let tags = [
        (ValueOutcome::Class(OutcomeClass::Ok), "ok"),
        (ValueOutcome::Class(OutcomeClass::Status(404)), "status:404"),
        (ValueOutcome::Class(OutcomeClass::Exit(-1)), "exit:-1"),
        (ValueOutcome::Class(OutcomeClass::NotFound), "not_found"),
        (
            ValueOutcome::Class(OutcomeClass::CannotStart),
            "cannot_start",
        ),
        (
            ValueOutcome::Class(OutcomeClass::Unreachable(Unreachable::RefusedAddress)),
            "unreachable:refused_address",
        ),
        (
            ValueOutcome::Class(OutcomeClass::Redirect(Redirected::Downgrade)),
            "redirect:downgrade",
        ),
        (ValueOutcome::Unknown, "unknown"),
    ];
    for (outcome, tag) in tags {
        assert_eq!(outcome.tag(), tag, "{outcome:?}");
    }
}

/// [`gsd_judged_project`] with `@specforge/checks` loaded, so the plan's `files_modified` is checked as a file under
/// the project root, and the runtime that serves both.
fn file_checked() -> (TestProject, specforge_project::SharedRuntime) {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_JUDGED, "@specforge/checks"])
        .installing_enabled()
        .file("a.spec", GSD_JUDGED_SPEC)
        .file("CLAUDE.md", "# Instructions\n")
        .file("src/auth/session.ts", "export const session = 1;\n");
    let runtime = gsd_judged_runtime().beside(specforge_project::testing::component_runtime());
    (project, std::sync::Arc::new(runtime))
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "the baseline digests an ungated checked value at the commit, over the commit's files"
)]
fn the_baseline_digests_a_checked_value_at_the_commit() {
    use specforge_effects::git::Sha;
    use specforge_project::baseline::{Baseline, Checking, CommitReads};
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    let (project, runtime) = file_checked();
    let compiled = project.compile_in(runtime.clone());
    let digested = compiled.checked_values().of(PLAN);
    assert_eq!(digested.len(), 1, "the plan's one file is checked");
    assert_eq!(digested[0].outcome, OK);
    record_from(
        &project,
        &compiled,
        PLAN,
        W311,
        "passed",
        &Cited::default(),
        "",
    );
    // The commit: every file as it is now.
    let committed: BTreeMap<String, Vec<u8>> = [
        "a.spec",
        "specforge-harness",
        "CLAUDE.md",
        "src/auth/session.ts",
    ]
    .into_iter()
    .map(|key| (key.to_string(), project.read(key).into_bytes()))
    .collect();
    let at_commit = |compiled: &specforge_project::CompiledProject| {
        let fetch = |keys: &[String]| -> Vec<(String, Option<Vec<u8>>)> {
            keys.iter()
                .map(|key| (key.clone(), committed.get(key).cloned()))
                .collect()
        };
        let sizes = |keys: &[String]| -> Vec<(String, Option<u64>)> {
            keys.iter()
                .map(|key| (key.clone(), committed.get(key).map(|b| b.len() as u64)))
                .collect()
        };
        let files = ["a.spec", "specforge-harness"]
            .map(|key| (PathBuf::from(key), committed[key].clone()))
            .to_vec();
        let tree = committed
            .keys()
            .map(|key| (PathBuf::from(key), true))
            .collect();
        let baseline = Baseline::compile(
            compiled.environment(),
            Sha::new("c".repeat(40)),
            Path::new(""),
            files,
            tree,
            CommitReads {
                sizes: &sizes,
                fetch: &fetch,
            },
            &Checking::of(compiled.environment()),
        );
        let judged = &baseline.entities[PLAN].judgements;
        judged
            .iter()
            .find(|j| j.rule == W311)
            .unwrap_or_else(|| panic!("W311 at the commit: {judged:?}"))
            .state
            .clone()
    };
    // Unchanged: the commit digests the file's class as now does, so the verdict is fresh at both ends.
    let now = project.compile_in(runtime.clone());
    assert_eq!(judgement(&now, PLAN, W311).0, "fresh");
    assert_eq!(at_commit(&now), "fresh");
    // Deleted since: now it is missing and the verdict is stale; the commit still holds it, so there it is fresh.
    project.remove("src/auth/session.ts");
    let now = project.compile_in(runtime);
    assert_eq!(judgement(&now, PLAN, W311).0, "stale");
    assert_eq!(at_commit(&now), "fresh");
}
