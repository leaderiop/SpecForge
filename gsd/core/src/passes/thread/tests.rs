use serde_json::json;

use super::fixture::add_test;
use super::*;

fn input() -> PassInput {
    serde_json::from_value(json!({ "entities": [] })).expect("a pass input")
}

/// `input` holding `facts` as `@code/core` test facts `(id, name, file, status)` that name nothing.
fn with_facts(facts: &[(&str, &str, &str, TestStatus)]) -> PassInput {
    let mut input = input();
    for (id, name, file, status) in facts {
        add_test(&mut input, id, name, file, *status, &[]);
    }
    input
}

const ADDS: &str = "js:taskboard/test/tasks.test.js#adds";
const LISTS: &str = "js:taskboard/test/tasks.test.js#lists";
const FILE: &str = "test/tasks.test.js";

#[test]
fn no_test_fact_is_not_run() {
    let input = input();
    let thread = Thread::of(&input);
    assert!(!thread.ran());
    assert_eq!(thread.of_entity("T1"), Evidence::NotRun);
    // A report a collector filed is not read: only test facts are a run.
    let reported: PassInput = serde_json::from_value(json!({
        "entities": [],
        "test_results": { "results": { "T1": { "tests": [{ "name": "adds", "status": TestStatus::Passed }] } } },
    }))
    .expect("a pass input");
    assert_eq!(Thread::of(&reported).of_entity("T1"), Evidence::NotRun);
}

#[test]
fn a_run_with_nothing_linked_is_no_test() {
    let mut input = input();
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Passed, &["T2"]);
    let thread = Thread::of(&input);
    assert!(thread.ran());
    assert_eq!(thread.of_entity("T1"), Evidence::NoTest);
}

#[test]
fn a_skipped_test_is_no_evidence() {
    let mut input = input();
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Skipped, &["T1"]);
    assert_eq!(Thread::of(&input).of_entity("T1"), Evidence::NoTest);
}

#[test]
fn a_failed_test_outweighs_a_passed_one() {
    let mut input = input();
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Passed, &["T1"]);
    add_test(
        &mut input,
        LISTS,
        "lists",
        FILE,
        TestStatus::Failed,
        &["T1"],
    );
    let thread = Thread::of(&input);
    assert_eq!(thread.of_entity("T1"), Evidence::Failed);
    assert_eq!(
        thread.naming("T1"),
        [(ADDS, TestStatus::Passed), (LISTS, TestStatus::Failed)]
    );
}

#[test]
fn a_cited_test_counts_wherever_it_ran() {
    let mut input = input();
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Passed, &["T1"]);
    add_test(
        &mut input,
        LISTS,
        "lists",
        FILE,
        TestStatus::Failed,
        &["T2"],
    );
    let thread = Thread::of(&input);
    assert_eq!(
        thread.cited(&["lists".to_string(), "adds".to_string()]),
        [TestStatus::Passed, TestStatus::Failed]
    );
    assert_eq!(
        thread.evidence(thread.cited(&["lists".to_string()])),
        Evidence::Failed
    );
}

#[test]
fn a_checks_ref_names_a_fact() {
    let input = with_facts(&[
        (ADDS, "adds", FILE, TestStatus::Passed),
        (LISTS, "lists", FILE, TestStatus::Failed),
    ]);
    let thread = Thread::of(&input);
    assert_eq!(
        thread.named("test/tasks.test.js#adds"),
        [TestStatus::Passed]
    );
    assert_eq!(thread.named(LISTS), [TestStatus::Failed]);
    assert_eq!(
        thread.evidence(thread.named("test/tasks.test.js#gone")),
        Evidence::NoTest
    );
}

/// A truth's tests are the facts that name it (ADR 0371 D5): a `proof` edge, which no truth writes now, links nothing.
#[test]
fn a_truths_tests_are_the_facts_that_name_it() {
    let mut input = with_facts(&[(ADDS, "adds", FILE, TestStatus::Failed)]);
    input.edges.push(PassEdge {
        source: "TR1".to_string(),
        target: ADDS.to_string(),
        label: "proof".to_string(),
    });
    add_test(
        &mut input,
        LISTS,
        "lists",
        FILE,
        TestStatus::Passed,
        &["TR1"],
    );
    let thread = Thread::of(&input);
    assert_eq!(thread.naming("TR1"), [(LISTS, TestStatus::Passed)]);
    assert_eq!(thread.of_entity("TR1"), Evidence::Passed);
}

#[test]
fn a_test_fact_is_a_test_run() {
    let input = with_facts(&[(ADDS, "adds", FILE, TestStatus::Passed)]);
    let thread = Thread::of(&input);
    assert!(thread.ran());
    assert_eq!(thread.of_entity("TR1"), Evidence::NoTest);
}

#[test]
fn a_checks_ref_names_a_fact_by_its_name_and_file() {
    let input = with_facts(&[
        (ADDS, "adds", FILE, TestStatus::Passed),
        (
            "js:other/test/other.test.js#adds",
            "adds",
            "test/other.test.js",
            TestStatus::Failed,
        ),
    ]);
    let thread = Thread::of(&input);
    assert_eq!(
        thread.named("test/tasks.test.js#adds"),
        [TestStatus::Passed]
    );
    assert_eq!(thread.named("test/missing.test.js#adds"), []);
    // A ref with no file names the test in every file.
    assert_eq!(
        thread.named("adds"),
        [TestStatus::Passed, TestStatus::Failed]
    );
}

#[test]
fn a_checks_ref_splits_at_its_first_hash() {
    const FIXES: &str = "js:taskboard/test/tasks.test.js#fixes #42";
    let input = with_facts(&[(FIXES, "fixes #42", FILE, TestStatus::Passed)]);
    let thread = Thread::of(&input);
    // The first '#' ends the file: the name is `fixes #42`, not `42` in the file `test/tasks.test.js#fixes `.
    assert_eq!(
        thread.named("test/tasks.test.js#fixes #42"),
        [TestStatus::Passed]
    );
}

#[test]
fn a_cited_name_with_a_hash_and_no_file_is_read_whole() {
    const FIXES: &str = "js:taskboard/test/tasks.test.js#fixes #42";
    let input = with_facts(&[
        (FIXES, "fixes #42", FILE, TestStatus::Passed),
        (ADDS, "adds", FILE, TestStatus::Failed),
        (
            "js:o/x.js#whole",
            "test/tasks.test.js#adds",
            "x.js",
            TestStatus::Passed,
        ),
    ]);
    let thread = Thread::of(&input);
    // `fixes ` names no test file: the whole text is the name.
    assert_eq!(thread.cited(&["fixes #42"]), [TestStatus::Passed]);
    // A `file#name` reading that finds a fact wins: `adds` of the file, not the test named the whole text.
    assert_eq!(
        thread.cited(&["test/tasks.test.js#adds"]),
        [TestStatus::Failed]
    );
    assert!(thread.cited(&["nothing #42"]).is_empty());
}

#[test]
fn a_test_names_its_entity_through_its_names_edge() {
    let mut input = input();
    add_test(
        &mut input,
        ADDS,
        "adds",
        FILE,
        TestStatus::Passed,
        &["TR1", "T1"],
    );
    add_test(
        &mut input,
        LISTS,
        "lists",
        FILE,
        TestStatus::Failed,
        &["T2"],
    );
    let thread = Thread::of(&input);
    assert_eq!(thread.naming("TR1"), [(ADDS, TestStatus::Passed)]);
    assert_eq!(thread.naming("T1"), [(ADDS, TestStatus::Passed)]);
    assert_eq!(thread.of_entity("T2"), Evidence::Failed);
    // A fact that names another entity is no evidence for this one.
    assert_eq!(thread.of_entity("T3"), Evidence::NoTest);
}

#[test]
fn a_cited_test_is_a_fact_by_id_or_by_name() {
    let mut input = input();
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Passed, &[]);
    add_test(&mut input, LISTS, "lists", FILE, TestStatus::Failed, &[]);
    let thread = Thread::of(&input);
    assert_eq!(thread.cited(&[ADDS.to_string()]), [TestStatus::Passed]);
    assert_eq!(
        thread.cited(&["test/tasks.test.js#lists".to_string()]),
        [TestStatus::Failed]
    );
    assert_eq!(
        thread.cited(&["adds".to_string(), "gone".to_string()]),
        [TestStatus::Passed]
    );
    assert_eq!(thread.cited(&["test/other.test.js#adds".to_string()]), []);
}

/// The passes whose handler reads the thread: `grep -rn 'Thread::of' gsd/core/src/passes` keeps this list whole, with
/// the callers of `requirement_proof` (the derive pass), which reads it inside.
const READ_THE_THREAD: [&str; 3] = ["derive", "scan", "verify_rules"];

#[test]
fn every_pass_that_reads_the_thread_declares_test_facts() {
    let declaration = crate::build().declaration();
    let facts = |inputs: &[Input]| -> Vec<String> {
        inputs
            .iter()
            .flat_map(|input| match input {
                Input::Facts { kinds } => kinds.clone(),
                _ => Vec::new(),
            })
            .collect()
    };
    let mut reading: Vec<&str> = Vec::new();
    for pass in &declaration.passes {
        match facts(&pass.inputs).as_slice() {
            [] => {}
            [only] if only == TEST => reading.push(&pass.name),
            other => panic!("pass '{}' declares the facts {other:?}", pass.name),
        }
    }
    reading.sort();
    assert_eq!(reading, READ_THE_THREAD);
}

const TASK: &str = "v1.0/01/01-01/T1";

/// A requirement R1 its plan lists, the plan's task of `task_type`, a commit naming the task when `commit`, and `tests`
/// as test facts `(entity it names, status)`. Answers the input and its commits.
fn traced(task_type: &str, commit: bool, tests: &[(&str, TestStatus)]) -> (PassInput, Vec<Commit>) {
    let mut entities = vec![
        json!({ "id": "R1", "kind": "requirement", "fields": { "text": "a task is added" } }),
        json!({ "id": "v1.0/01/01-01", "kind": "plan", "fields": { "phase": "v1.0/01" },
                "values": { "requirements": ["R1"] } }),
        json!({ "id": TASK, "kind": "task", "fields": { "plan": "v1.0/01/01-01", "type": task_type } }),
    ];
    if commit {
        entities.push(json!({ "id": "abcdef123456", "kind": "commit",
                              "fields": { "task": TASK, "at": "2026-10-29T10:00:00Z" } }));
    }
    let value = json!({ "entities": entities, "edges": [] });
    let mut input: PassInput =
        serde_json::from_value(crate::passes::plans::with_field_edges(value))
            .expect("a pass input");
    for (n, (entity, status)) in tests.iter().enumerate() {
        let id = format!("js:app/test/t.test.js#t{n}");
        add_test(
            &mut input,
            &id,
            &format!("t{n}"),
            "test/t.test.js",
            *status,
            &[entity],
        );
    }
    let commits = crate::passes::commits::commits(&input);
    (input, commits)
}

fn proof_of((input, commits): &(PassInput, Vec<Commit>)) -> Proof {
    requirement_proof(input, commits, "R1")
}

/// Pushes onto `input` an entity `id` of `kind` whose `requirement` is `requirement` and whose `checks` hold one row
/// with the `ref` `reference`: a summary's coverage item when `kind` is `deliverable`.
fn cover(input: &mut PassInput, kind: &str, id: &str, requirement: &str, reference: &str) {
    input.entities.push(
        serde_json::from_value(json!({
            "id": id, "kind": kind, "fields": { "requirement": requirement },
            "values": { "checks": [{ "kind": "unit", "ref": reference }] },
        }))
        .expect("an entity"),
    );
    input.edges.push(PassEdge {
        source: id.to_string(),
        target: requirement.to_string(),
        label: "requirement".to_string(),
    });
}

const D1: &str = "v1.0/01/01-01/01-01-SUMMARY/D1";
const ADDS_REF: &str = "test/tasks.test.js#adds";

/// [`traced`] with a committed task, the test fact `adds` of `status` that names nothing, and a coverage item of
/// R1 whose check names it.
fn covered(status: TestStatus) -> (PassInput, Vec<Commit>) {
    let (mut input, commits) = traced("auto", true, &[]);
    add_test(&mut input, ADDS, "adds", FILE, status, &[]);
    cover(&mut input, "deliverable", D1, "R1", ADDS_REF);
    (input, commits)
}

#[test]
fn a_passing_test_a_coverage_check_names_proves_a_listed_requirement() {
    assert_eq!(proof_of(&covered(TestStatus::Passed)), Proof::Proven);
}

#[test]
fn a_failed_coverage_check_fails_the_requirement() {
    assert_eq!(proof_of(&covered(TestStatus::Failed)), Proof::Failed);
}

#[test]
fn a_coverage_check_that_names_no_fact_proves_nothing() {
    let (mut input, commits) = traced("auto", true, &[]);
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Passed, &[]);
    cover(
        &mut input,
        "deliverable",
        D1,
        "R1",
        "test/tasks.test.js#gone",
    );
    assert_eq!(requirement_proof(&input, &commits, "R1"), Proof::NoTest);
}

#[test]
fn only_a_deliverable_brings_its_checks() {
    for kind in ["validation_row", "manual_check", "spec_prohibition"] {
        let (mut input, commits) = traced("auto", true, &[]);
        add_test(&mut input, ADDS, "adds", FILE, TestStatus::Passed, &[]);
        cover(&mut input, kind, "v1.0/01/x/X1", "R1", ADDS_REF);
        assert_eq!(
            requirement_proof(&input, &commits, "R1"),
            Proof::NoTest,
            "{kind}"
        );
        assert!(Thread::of(&input).coverage("R1").is_empty(), "{kind}");
    }
}

/// R1, listed by the phase `v1.0/01` and by no plan, with `tests` as test facts `(entity it names, status)`.
fn unlisted(tests: &[(&str, TestStatus)]) -> PassInput {
    let value = json!({ "entities": [
        { "id": "R1", "kind": "requirement", "fields": { "text": "a task is added" } },
        { "id": "v1.0/01", "kind": "phase", "values": { "requirements": ["R1"] } },
    ], "edges": [] });
    let mut input: PassInput =
        serde_json::from_value(crate::passes::plans::with_field_edges(value))
            .expect("a pass input");
    for (n, (entity, status)) in tests.iter().enumerate() {
        let id = format!("js:app/test/t.test.js#t{n}");
        add_test(
            &mut input,
            &id,
            &format!("t{n}"),
            "test/t.test.js",
            *status,
            &[entity],
        );
    }
    input
}

#[test]
fn a_requirement_no_plan_lists_reads_its_tests_alone() {
    let proof = |input: &PassInput| requirement_proof(input, &[], "R1");
    // A phase's `requirements` lists it: no plan does, so no task is looked for.
    assert!(requirement_tasks(&unlisted(&[]), "R1").is_empty());
    assert_eq!(proof(&unlisted(&[])), Proof::NotRun);
    assert_eq!(
        proof(&unlisted(&[("T9", TestStatus::Passed)])),
        Proof::NoTest
    );
    assert_eq!(
        proof(&unlisted(&[("R1", TestStatus::Passed)])),
        Proof::Proven
    );
    assert_eq!(
        proof(&unlisted(&[("R1", TestStatus::Failed)])),
        Proof::Failed
    );
    let mut input = unlisted(&[("T9", TestStatus::Passed)]);
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Failed, &[]);
    cover(&mut input, "deliverable", D1, "R1", ADDS_REF);
    assert_eq!(proof(&input), Proof::Failed);
}

#[test]
fn a_requirement_a_plan_lists_with_no_task_is_no_task() {
    let (mut input, commits) = traced("auto", true, &[("R1", TestStatus::Passed)]);
    input.entities.retain(|e| e.kind != "task");
    input
        .edges
        .retain(|e| !(e.label == "plan" && e.source == TASK));
    assert!(requirement_tasks(&input, "R1").is_empty());
    // A passing test that names the requirement does not stand in for the plan's missing task.
    assert_eq!(requirement_proof(&input, &commits, "R1"), Proof::NoTask);
}

#[test]
fn a_task_without_a_commit_is_no_commit() {
    assert_eq!(
        proof_of(&traced("auto", false, &[(TASK, TestStatus::Passed)])),
        Proof::NoCommit
    );
}

#[test]
fn a_committed_task_without_test_facts_is_not_run() {
    assert_eq!(proof_of(&traced("auto", true, &[])), Proof::NotRun);
}

#[test]
fn a_committed_task_with_nothing_linked_is_no_test() {
    // A test fact is collected, and it names another task.
    let other = traced("auto", true, &[("v1.0/01/01-01/T2", TestStatus::Passed)]);
    assert_eq!(proof_of(&other), Proof::NoTest);
}

#[test]
fn a_passing_test_naming_the_task_proves_it() {
    assert_eq!(
        proof_of(&traced("auto", true, &[(TASK, TestStatus::Passed)])),
        Proof::Proven
    );
}

#[test]
fn a_passing_test_naming_the_requirement_proves_it() {
    assert_eq!(
        proof_of(&traced("tdd", true, &[("R1", TestStatus::Passed)])),
        Proof::Proven
    );
}

#[test]
fn a_failing_linked_test_is_failed() {
    let tests = [(TASK, TestStatus::Passed), (TASK, TestStatus::Failed)];
    assert_eq!(proof_of(&traced("auto", true, &tests)), Proof::Failed);
    assert_eq!(
        proof_of(&traced("auto", true, &[("R1", TestStatus::Failed)])),
        Proof::Failed
    );
}

/// A committed task's tests are the facts that name it: a `proof` edge from the task links nothing.
#[test]
fn a_committed_tasks_tests_are_the_facts_that_name_it() {
    let (mut input, commits) = traced("auto", true, &[(TASK, TestStatus::Passed)]);
    add_test(&mut input, ADDS, "adds", FILE, TestStatus::Failed, &[]);
    input.edges.push(PassEdge {
        source: TASK.to_string(),
        target: ADDS.to_string(),
        label: "proof".to_string(),
    });
    assert_eq!(requirement_proof(&input, &commits, "R1"), Proof::Proven);
}

#[test]
fn the_proof_states_are_the_kinds_values() {
    assert_eq!(Proof::ALL.map(Proof::as_str), crate::kinds::PROOF_STATES);
    for proof in Proof::ALL {
        assert_eq!(Proof::parse(proof.as_str()), Some(proof));
    }
    assert_eq!(Proof::parse("passed"), None);
}

#[test]
fn requirement_proven_needs_complete_and_proven() {
    assert!(requirement_proven(Some("complete"), Proof::Proven));
    for proof in Proof::ALL.into_iter().filter(|p| *p != Proof::Proven) {
        assert!(!requirement_proven(Some("complete"), proof), "{proof:?}");
        assert!(proof.missing().is_some(), "{proof:?}");
    }
    for status in [
        None,
        Some("pending"),
        Some("in_progress"),
        Some("gaps_found"),
    ] {
        assert!(!requirement_proven(status, Proof::Proven), "{status:?}");
    }
    assert_eq!(Proof::Proven.missing(), None);
}
