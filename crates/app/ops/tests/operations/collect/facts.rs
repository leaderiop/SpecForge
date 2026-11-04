//! A collector's facts (ADR 0327): stored in `.specforge/facts/<key>/<collector>.json` under the key of the commit
//! they were collected at, recorded by digest in `specforge-harness`, in one change under the write lock; and a
//! facts-only answer that writes no test report. The fixture is [`Collected`] (`@t/code`'s `test`, `@t/node`'s `node`
//! and `unit`), with a scripted history.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_effects::SharedGit;
use specforge_effects::testing::FakeGit;
use specforge_ops::analysis::collect::{self, Mode, Outcome, Request};
use specforge_ops::authoring::apply::apply_collected;
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::facts::Store;
use specforge_project::harness::digest;
use specforge_project::testing::{
    COLLECTED_EXTENSIONS, Collected, GSD_WRITES_EXTENSIONS, GsdWrites, NODE, NODE_REPORT,
    TestProject, UNIT_REPORT, collected_answer,
};
use specforge_protocol_types::Fact;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// A project enabling the fixture, a truth to report on, and both collectors' reports.
fn project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file("truths.spec", "truth T1 \"One\" {}\n")
        .file("package.json", "{}")
        .file(NODE_REPORT, "<testsuites/>")
        .file(UNIT_REPORT, "{}")
}

/// `node`'s facts: two tests of `a.test.js`, as the wire carries them.
fn two_tests() -> Value {
    json!([
        {"kind": "test", "id": "js:a.test.js#subtracts", "fields": {"file": "a.test.js", "status": "failed"}},
        {"kind": "test", "id": "js:a.test.js#adds", "title": "adds", "fields": {"file": "a.test.js", "status": "passed"}},
    ])
}

/// A `collect__unit` answer: `T1` proved by one passing test.
fn proving() -> WasmCallResult {
    WasmCallResult::Ok(
        json!({"entity_results": [{"entity_id": "T1", "test_results": [{"name": "t1_holds", "status": "passed"}]}]})
            .to_string()
            .into_bytes(),
    )
}

/// The empty answer of a collector that found nothing.
fn nothing() -> WasmCallResult {
    WasmCallResult::Ok(json!({"entity_results": []}).to_string().into_bytes())
}

/// Collect `runner` (every detected collector when `None`) from the reports on disk, served by `runtime`, and store
/// its facts as the surfaces do, the project asking `git` of version control; a plan is left in the outcome.
fn collect(
    project: &TestProject,
    git: &SharedGit,
    runtime: InProcessRuntime,
    runner: Option<&str>,
) -> Outcome {
    let compiled = project.compile_with(Arc::new(runtime), Some(Arc::clone(git)));
    let view = ProjectView::of(&compiled);
    let mut outcome = collect::collect(
        &view,
        Request {
            runner,
            mode: Mode::NoRun,
            announce: &mut |_, _| {},
            caller: caller(),
        },
        &ScriptedMachine::new(),
        &specforge_effects::testing::MemoryConsent::default(),
    )
    .expect("the collect runs");
    let facts = std::mem::take(&mut outcome.facts);
    apply_collected(&view, Vec::new(), facts, false, caller()).expect("the facts are stored");
    outcome
}

/// `node` answering `facts` and `unit` answering `unit`.
fn serving(facts: Value, unit: WasmCallResult) -> InProcessRuntime {
    Collected::new()
        .answering(NODE, "collect__node", collected_answer(facts))
        .answering(NODE, "collect__unit", unit)
        .runtime()
}

fn ledger(project: &TestProject) -> String {
    std::fs::read_to_string(project.root().join("specforge-harness")).unwrap_or_default()
}

fn codes(outcome: &Outcome) -> Vec<&str> {
    outcome
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect()
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a collector's facts are stored under the key of the commit they were collected at and their digest is recorded in specforge-harness"
)]
fn collect_stores_a_collectors_facts_under_the_head_commit_and_records_their_digest() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));

    let outcome = collect(&p, &git, serving(two_tests(), nothing()), Some("node"));

    let bytes = std::fs::read(p.root().join(Store::path("node", SHA))).unwrap();
    let facts: Vec<Fact> = serde_json::from_value(two_tests()).unwrap();
    let canonical = Store::new("node", NODE, SHA, facts).bytes();
    assert_eq!(
        String::from_utf8_lossy(&bytes),
        String::from_utf8_lossy(&canonical)
    );
    let line = format!(
        "node facts:{SHA} {}\n",
        digest("node", &format!("facts:{SHA}"), &bytes)
    );
    assert_eq!(ledger(&p), format!("# specforge-harness 1\n{line}"));
    assert_eq!(outcome.runners[0].facts, 2);
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a dirty tree or a project in no repository files its facts under a key that says so, with I209"
)]
fn a_dirty_tree_files_facts_under_the_commit_and_dirty_with_i209() {
    let p = project();
    let mut fake = FakeGit::repo(p.root(), SHA, Vec::new());
    fake.status.entries.push(specforge_effects::git::GitEntry {
        kind: specforge_effects::git::GitEntryKind::Untracked,
        path: "notes.txt".into(),
        orig_path: None,
        index: None,
        worktree: None,
    });
    let git: SharedGit = Arc::new(fake);

    let outcome = collect(&p, &git, serving(two_tests(), nothing()), Some("node"));

    assert!(
        p.root()
            .join(Store::path("node", &format!("{SHA}-dirty")))
            .is_file()
    );
    assert_eq!(codes(&outcome), ["I209"]);
    assert!(
        outcome.diagnostics[0]
            .message
            .contains(&format!("'{SHA}-dirty'")),
        "{:?}",
        outcome.diagnostics
    );
    assert!(
        ledger(&p).contains(&format!("node facts:{SHA}-dirty ")),
        "{}",
        ledger(&p)
    );
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "a dirty tree or a project in no repository files its facts under a key that says so, with I209"
)]
fn a_project_in_no_repository_files_facts_under_worktree_with_i209() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::none());

    let outcome = collect(&p, &git, serving(two_tests(), nothing()), Some("node"));

    assert!(p.root().join(Store::path("node", "worktree")).is_file());
    assert_eq!(codes(&outcome), ["I209"]);
    assert!(
        outcome.diagnostics[0].message.contains("no git repository"),
        "{:?}",
        outcome.diagnostics
    );
}

#[specforge_test(
    behavior = "collect_harness_ledger",
    verify = "a collect records one facts line per collector, replacing its last"
)]
fn a_second_collect_replaces_the_collectors_line_and_its_store() {
    let p = project();
    {
        let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
        collect(&p, &git, serving(two_tests(), nothing()), Some("node"));
    }
    let next = "fedcba9876543210fedcba9876543210fedcba98";
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), next, Vec::new()));

    collect(
        &p,
        &git,
        serving(json!([two_tests()[0]]), nothing()),
        Some("node"),
    );

    assert!(
        !p.root().join(Store::path("node", SHA)).exists(),
        "the old key's store is gone"
    );
    assert!(p.root().join(Store::path("node", next)).is_file());
    let lines: Vec<String> = ledger(&p).lines().skip(1).map(String::from).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(&format!("node facts:{next} ")),
        "{lines:?}"
    );
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "facts that name a kind no collector writes, or an id outside its kind's schemes, are E116 and none of them is stored"
)]
fn a_fact_of_a_kind_no_collector_writes_is_e116_and_nothing_is_stored() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
    let facts = json!([two_tests()[0], {"kind": "truth", "id": "js:x#y"}]);

    let outcome = collect(&p, &git, serving(facts, nothing()), Some("node"));

    assert_eq!(codes(&outcome), ["E116"]);
    assert!(
        outcome.diagnostics[0]
            .message
            .contains("collector 'node' (@t/node)"),
        "{:?}",
        outcome.diagnostics
    );
    assert!(
        outcome.diagnostics[0]
            .message
            .contains("of the kind 'truth'"),
        "{:?}",
        outcome.diagnostics
    );
    assert!(!p.root().join(".specforge/facts").exists());
    assert_eq!(ledger(&p), "");
    assert_eq!(outcome.runners[0].facts, 0);
}

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "facts that name a kind no collector writes, or an id outside its kind's schemes, are E116 and none of them is stored"
)]
fn a_fact_whose_id_names_none_of_its_kinds_schemes_is_e116() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
    let facts = json!([{"kind": "test", "id": "rust:a#b"}]);

    let outcome = collect(&p, &git, serving(facts, nothing()), Some("node"));

    assert_eq!(codes(&outcome), ["E116"]);
    assert!(
        outcome.diagnostics[0]
            .message
            .contains("'rust:a#b' names none of the schemes"),
        "{:?}",
        outcome.diagnostics
    );
    assert!(!p.root().join(".specforge/facts").exists());
}

#[test]
fn a_collect_with_no_facts_writes_no_store_and_no_line() {
    let p = project();
    let fake = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
    let git: SharedGit = fake.clone();

    let outcome = collect(&p, &git, serving(json!([]), nothing()), Some("node"));

    assert!(!p.root().join(".specforge/facts").exists());
    assert!(!p.root().join("specforge-harness").exists());
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "two writers both land their lines"
)]
fn facts_and_their_line_land_beside_another_writers_line() {
    let uat = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\nuat_test U1 \"U\" {\n  phase 03\n  result pass\n}\n";
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(
            &[
                COLLECTED_EXTENSIONS.as_slice(),
                GSD_WRITES_EXTENSIONS.as_slice(),
            ]
            .concat(),
        )
        .installing_enabled()
        .file("m.spec", uat)
        .file("package.json", "{}")
        .file(NODE_REPORT, "<testsuites/>");
    // A value another writer recorded: the harness field `result` of the UAT test.
    let value = digest("v1.0/03/U1", "result", b"\"pass\"");
    p.write(
        "specforge-harness",
        format!("# specforge-harness 1\nv1.0/03/U1 result {value}\n"),
    );
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
    let runtime = Collected::new()
        .answering(NODE, "collect__node", collected_answer(two_tests()))
        .over(GsdWrites::new().runtime());

    collect(&p, &git, runtime, Some("node"));

    let ledger = ledger(&p);
    assert!(
        ledger.contains(&format!("v1.0/03/U1 result {value}\n")),
        "{ledger}"
    );
    assert!(ledger.contains(&format!("node facts:{SHA} ")), "{ledger}");
}

#[specforge_test(
    behavior = "ingest_collector_report",
    verify = "a collect whose answers carry facts and no test results creates and rewrites no report, in a run of one collector or several"
)]
fn a_facts_only_collect_creates_no_report_and_rewrites_none() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));

    let outcome = collect(&p, &git, serving(two_tests(), nothing()), Some("node"));
    assert!(outcome.report.is_none());
    assert!(!p.root().join("specforge-report.json").exists());

    // A report holding another runner's entries is left as it is, bytes and time.
    let held = "{\n  \"runner\": \"unit\",\n  \"results\": {\"T1\": {\"tests\": [{\"name\": \"t\", \"status\": \"passed\", \"runner\": \"unit\"}]}}\n}";
    p.write("specforge-report.json", held);
    let before = std::fs::metadata(p.root().join("specforge-report.json"))
        .unwrap()
        .modified()
        .unwrap();
    let outcome = collect(&p, &git, serving(two_tests(), nothing()), Some("node"));
    assert!(outcome.report.is_none());
    assert_eq!(
        std::fs::read_to_string(p.root().join("specforge-report.json")).unwrap(),
        held
    );
    let after = std::fs::metadata(p.root().join("specforge-report.json"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(before, after);
}

#[specforge_test(
    behavior = "ingest_collector_report",
    verify = "a collect whose answers carry facts and no test results creates and rewrites no report, in a run of one collector or several"
)]
fn a_mixed_run_of_a_plan_only_and_a_facts_only_collector_writes_no_report() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
    let plan_only = WasmCallResult::Ok(
        json!({"entity_results": [], "plan": {"ops": []}})
            .to_string()
            .into_bytes(),
    );

    let outcome = collect(&p, &git, serving(two_tests(), plan_only), None);

    assert_eq!(outcome.runners.len(), 2, "both collectors ran");
    assert_eq!(outcome.plans.len(), 1, "the plan is kept for the caller");
    assert!(outcome.report.is_none());
    assert!(!p.root().join("specforge-report.json").exists());
    assert!(p.root().join(Store::path("node", SHA)).is_file());
}

#[test]
fn a_mixed_run_with_one_reporting_collector_writes_only_its_runners_entries() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));

    let outcome = collect(&p, &git, serving(two_tests(), proving()), None);

    assert!(outcome.report.is_some());
    let report: Value =
        serde_json::from_slice(&std::fs::read(p.root().join("specforge-report.json")).unwrap())
            .unwrap();
    assert_eq!(report["runner"], "unit", "{report}");
    let tests = report["results"]["T1"]["tests"].as_array().unwrap();
    assert_eq!(tests.len(), 1, "{report}");
    assert_eq!(tests[0]["runner"], "unit");
}

#[test]
fn a_collector_whose_answer_empties_its_earlier_results_still_clears_them() {
    let p = project();
    let git: SharedGit = Arc::new(FakeGit::repo(p.root(), SHA, Vec::new()));
    collect(&p, &git, serving(json!([]), proving()), Some("unit"));

    let outcome = collect(&p, &git, serving(json!([]), nothing()), Some("unit"));

    assert!(
        outcome.report.is_some(),
        "the emptied runner's entries are cleared"
    );
    let report: Value =
        serde_json::from_slice(&std::fs::read(p.root().join("specforge-report.json")).unwrap())
            .unwrap();
    assert_eq!(report["results"], json!({}), "{report}");
}

/// `@t/probe`: one check pass that finds nothing, whose input the test reads.
fn probe() -> specforge_extension_sdk::prelude::ContributionsBuilder {
    use specforge_extension_sdk::prelude::*;
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/probe", "1.0.0"));
    c.pass("audit", |p| {
        p.phase("check")
            .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
    });
    c
}

#[test]
fn a_pass_is_sent_the_fact_stores_the_session_served() {
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[COLLECTED_EXTENSIONS.as_slice(), &["@t/probe"]].concat())
        .installing_enabled()
        .file("truths.spec", "truth T1 \"One\" {}\n");
    specforge_project::testing::stored(p.root(), "node", SHA, two_tests());
    let runtime = Arc::new(Collected::new().over(InProcessRuntime::new().with(probe)));

    p.compile_in(runtime.clone());

    let call = runtime
        .calls()
        .into_iter()
        .find(|c| c.extension == "@t/probe" && c.export.ends_with("audit"))
        .expect("the check pass ran");
    assert_eq!(
        call.input["fact_stores"],
        json!([{"collector": "node", "key": SHA, "schemes": ["js"], "facts": 2}])
    );
}

/// A project citing a test no store holds: a fresh clone (the store is git-ignored), with `extra` enabled too.
fn citing(extra: &[&str]) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[COLLECTED_EXTENSIONS.as_slice(), extra].concat())
        .installing_enabled()
        .file(
            "truths.spec",
            "truth T1 \"One\" {\n  proof [\"js:a.test.js#adds\"]\n}\n",
        )
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fresh clone with no fact store reports no unresolved reference for its fact references"
)]
fn a_fresh_clone_has_no_unresolved_reference_for_its_fact_scheme_refs() {
    let p = citing(&[]);

    let project = p.compile_in(Arc::new(Collected::new().runtime()));

    assert!(
        project.diagnostics().iter().all(|d| d.code != "E003"),
        "{:?}",
        project.diagnostics()
    );
    assert!(project.graph().edges_from("T1").is_empty());
    let unheld: Vec<&str> = project
        .graph()
        .node("T1")
        .unwrap()
        .unheld
        .iter()
        .map(|u| u.id.as_str())
        .collect();
    assert_eq!(unheld, ["js:a.test.js#adds"]);
}

#[test]
fn a_pass_is_sent_each_unheld_reference() {
    let p = citing(&["@t/probe"]);
    let runtime = Arc::new(Collected::new().over(InProcessRuntime::new().with(probe)));

    p.compile_in(runtime.clone());

    let call = runtime
        .calls()
        .into_iter()
        .find(|c| c.extension == "@t/probe" && c.export.ends_with("audit"))
        .expect("the check pass ran");
    let t1 = call.input["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "T1")
        .expect("T1 is sent")
        .clone();
    assert_eq!(
        t1["unheld"],
        json!([{"field": "proof", "id": "js:a.test.js#adds", "span": {
            "file": "truths.spec", "start_line": 2, "start_col": 11, "end_line": 2, "end_col": 28,
        }}])
    );
    assert!(
        call.input.get("fact_stores").is_none(),
        "no store was served"
    );
}
