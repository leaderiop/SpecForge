use serde_json::json;

use super::fixture::add_test;
use super::*;

fn input() -> PassInput {
    serde_json::from_value(json!({ "entities": [] })).expect("a pass input")
}

/// `input` holding `facts` as `@code/core` test facts `(id, name, file, status)` and `proofs` as `(truth, fact)`
/// `proof` edges.
fn with_facts(facts: &[(&str, &str, &str, TestStatus)], proofs: &[(&str, &str)]) -> PassInput {
    let mut input = input();
    for (id, name, file, status) in facts {
        add_test(&mut input, id, name, file, *status, &[]);
    }
    for (truth, fact) in proofs {
        input.edges.push(PassEdge {
            source: truth.to_string(),
            target: fact.to_string(),
            label: "proof".to_string(),
        });
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
        thread.linked("T1"),
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
    let input = with_facts(
        &[
            (ADDS, "adds", FILE, TestStatus::Passed),
            (LISTS, "lists", FILE, TestStatus::Failed),
        ],
        &[],
    );
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

#[test]
fn a_truths_proof_results_are_linked_to_it() {
    let mut input = with_facts(
        &[
            (ADDS, "adds", FILE, TestStatus::Failed),
            (LISTS, "lists", FILE, TestStatus::Passed),
        ],
        &[("TR1", ADDS)],
    );
    add_test(
        &mut input,
        "js:taskboard/test/tasks.test.js#tagged",
        "tagged",
        FILE,
        TestStatus::Passed,
        &["TR1"],
    );
    let thread = Thread::of(&input);
    assert_eq!(
        thread.linked("TR1"),
        [
            ("js:taskboard/test/tasks.test.js#tagged", TestStatus::Passed),
            (ADDS, TestStatus::Failed)
        ]
    );
    assert_eq!(thread.proof("TR1"), [(ADDS, TestStatus::Failed)]);
    assert_eq!(thread.of_entity("TR1"), Evidence::Failed);
}

#[test]
fn a_test_fact_is_a_test_run() {
    let input = with_facts(&[(ADDS, "adds", FILE, TestStatus::Passed)], &[]);
    let thread = Thread::of(&input);
    assert!(thread.ran());
    assert_eq!(thread.of_entity("TR1"), Evidence::NoTest);
}

#[test]
fn a_checks_ref_names_a_fact_by_its_name_and_file() {
    let input = with_facts(
        &[
            (ADDS, "adds", FILE, TestStatus::Passed),
            (
                "js:other/test/other.test.js#adds",
                "adds",
                "test/other.test.js",
                TestStatus::Failed,
            ),
        ],
        &[],
    );
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
    let input = with_facts(&[(FIXES, "fixes #42", FILE, TestStatus::Passed)], &[]);
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
    let input = with_facts(
        &[
            (FIXES, "fixes #42", FILE, TestStatus::Passed),
            (ADDS, "adds", FILE, TestStatus::Failed),
            (
                "js:o/x.js#whole",
                "test/tasks.test.js#adds",
                "x.js",
                TestStatus::Passed,
            ),
        ],
        &[],
    );
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
    assert_eq!(thread.linked("T1"), [(ADDS, TestStatus::Passed)]);
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

#[test]
fn a_named_test_and_a_proof_both_link_a_truth() {
    let mut input = with_facts(
        &[(ADDS, "adds", FILE, TestStatus::Passed)],
        &[("TR1", ADDS)],
    );
    add_test(
        &mut input,
        LISTS,
        "lists",
        FILE,
        TestStatus::Failed,
        &["TR1"],
    );
    let thread = Thread::of(&input);
    assert_eq!(
        thread.linked("TR1"),
        [(LISTS, TestStatus::Failed), (ADDS, TestStatus::Passed)]
    );
    assert_eq!(thread.of_entity("TR1"), Evidence::Failed);
}

/// The passes whose handler reads the thread (`Thread::of`): `grep -rn 'Thread::of' gsd/core/src/passes` keeps this
/// list whole.
const READ_THE_THREAD: [&str; 4] = ["derive", "gate_e515", "scan", "verify_rules"];

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
