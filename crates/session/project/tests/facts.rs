//! The fact stores a session serves (ADR 0327 D5, D9): each one `specforge-harness` records is read-only entities of
//! the graph, gated off when no collector-written kind is loaded; a store or the ledger that changes is a source
//! change of the stores' keys. The fixture is `Collected` (`@t/code`'s `test`, `@t/node`'s `node`, `@t/spec`'s
//! `truth`).

use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, TestProject, stored};
use specforge_project::{CompiledProject, ProjectSession};
use specforge_test::prelude::*;

const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";

/// `node`'s facts: two tests of `a.test.js`.
fn two_tests() -> Value {
    json!([
        {"kind": "test", "id": "js:a.test.js#adds", "title": "adds", "fields": {"file": "a.test.js", "status": "passed"}},
        {"kind": "test", "id": "js:a.test.js#subtracts", "fields": {"file": "a.test.js", "status": "failed"}},
    ])
}

/// A project enabling the fixture, whose truth cites one of the tests.
fn collected() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file(
            "truths.spec",
            "truth T1 \"One\" {\n  proof [\"js:a.test.js#adds\"]\n}\n",
        )
}

fn compiled(p: &TestProject) -> CompiledProject {
    p.compile_in(Arc::new(Collected::new().runtime()))
}

fn session(p: &TestProject) -> ProjectSession {
    p.session_in(Arc::new(Collected::new().runtime()))
}

fn codes(project: &CompiledProject) -> Vec<String> {
    project
        .diagnostics()
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

fn export(project: &CompiledProject) -> String {
    specforge_emitter::emit(project.graph(), &specforge_emitter::EmitOptions::default()).unwrap()
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a project that loads no collector-written kind reads no fact store and compiles byte-identically"
)]
fn with_no_collected_kind_a_fact_store_on_disk_changes_no_output() {
    let p = TestProject::new().config(|c| c["name"] = "p".into()).file(
        "a.spec",
        "thing alpha \"A\" {\n  uses beta\n}\n\nthing beta \"B\" {}\n",
    );
    let before = p.compile();

    stored(p.root(), "node", KEY, two_tests());
    let after = p.compile();

    assert_eq!(
        format!("{:?}", before.diagnostics()),
        format!("{:?}", after.diagnostics())
    );
    assert_eq!(export(&before), export(&after));
    assert_eq!(before.source_keys(), after.source_keys());
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a fact store specforge-harness records is served as read-only entities; one it does not is W191, one that does not read W192"
)]
fn a_served_fact_is_an_entity_every_query_reads() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());

    let project = compiled(&p);

    let adds = project
        .graph()
        .node("js:a.test.js#adds")
        .expect("the fact is a node");
    assert_eq!(adds.kind.raw.as_str(), "test");
    let cited: Vec<&str> = project
        .graph()
        .edges_from("T1")
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(cited, ["js:a.test.js#adds"]);
    assert!(export(&project).contains("js:a.test.js#subtracts"));
    assert!(project.source_keys().contains(STORE));
    assert!(codes(&project).is_empty(), "{:?}", project.diagnostics());
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a fact's word is read bare, and a collector's fact is never W159"
)]
fn a_facts_word_is_read_bare_and_no_fact_is_w159() {
    let p = collected();
    // `status` is a word field: its JSON string is the word. `file` is a text field, given a number.
    let facts = json!([
        {"kind": "test", "id": "js:a.test.js#adds", "fields": {"file": 7, "status": "passed"}},
    ]);
    stored(p.root(), "node", KEY, facts);

    let project = compiled(&p);

    let adds = project.graph().node("js:a.test.js#adds").expect("the fact");
    assert_eq!(
        adds.fields.get("status"),
        Some(&specforge_parser::FieldValue::Identifier("passed".into()))
    );
    // A fact has no text to rewrite: not even the number on a text field is W159.
    assert!(
        !codes(&project).contains(&"W159".to_string()),
        "{:?}",
        project.diagnostics()
    );
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a fact's single-value reference to another fact links like a written one"
)]
fn a_facts_single_value_reference_links_like_a_written_one() {
    use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta, FieldType};
    use specforge_project::testing::CODE;
    use specforge_wasm::testing::InProcessRuntime;

    // `@t/run`'s `run` names the `test` it ran in one reference, as a runner pack's `test_run` does.
    let run = || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/run", "1.0.0"));
        c.peer(CODE, "^1.0");
        c.kind("run", |k| {
            k.collected(&["run"]).field("test", |f| {
                f.field_type(FieldType::Reference).target_kind("test");
            });
        });
        c
    };
    let mut enabled = COLLECTED_EXTENSIONS.to_vec();
    enabled.push("@t/run");
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&enabled)
        .installing_enabled();
    let mut facts = two_tests();
    facts.as_array_mut().unwrap().push(
        json!({"kind": "run", "id": "run:js:a.test.js#adds", "fields": {"test": "js:a.test.js#adds"}}),
    );
    stored(p.root(), "node", KEY, facts);

    // Before its value kept a span, the edge was held by no written token: the debug build's edge check panicked.
    let project = p.compile_in(Arc::new(
        Collected::new().over(InProcessRuntime::new().with(run)),
    ));

    let ran: Vec<&str> = project
        .graph()
        .edges_from("run:js:a.test.js#adds")
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(ran, ["js:a.test.js#adds"]);
    assert!(codes(&project).is_empty(), "{:?}", project.diagnostics());
}

#[test]
fn a_store_the_ledger_does_not_record_is_w191_and_serves_nothing() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());
    fs::remove_file(p.root().join("specforge-harness")).unwrap();

    let project = compiled(&p);

    assert!(project.graph().node("js:a.test.js#adds").is_none());
    assert!(
        codes(&project).contains(&"W191".to_string()),
        "{:?}",
        project.diagnostics()
    );
}

#[test]
fn a_rewritten_fact_store_is_a_source_change_of_its_key() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());
    let mut session = session(&p);
    assert!(
        session
            .project()
            .graph()
            .node("js:a.test.js#adds")
            .is_some()
    );

    stored(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": "js:a.test.js#subtracts"}]),
    );
    let root = fs::canonicalize(p.root()).unwrap();
    let changes = session.changes([root.join(STORE).as_path()]);

    assert_eq!(changes.sources(), [STORE]);
    assert!(session.apply(&changes).is_some());
    let graph = session.project().graph();
    assert!(graph.node("js:a.test.js#adds").is_none());
    assert!(graph.node("js:a.test.js#subtracts").is_some());
    assert!(session.stale().is_empty(), "{:?}", session.stale());
}

#[specforge_test(
    behavior = "watch_file_system_for_changes",
    verify = "a fact store and the harness ledger are watched inputs when a collector-written kind is loaded"
)]
fn a_changed_harness_ledger_reserves_the_fact_keys() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());
    let mut session = session(&p);
    assert!(
        session
            .project()
            .graph()
            .node("js:a.test.js#adds")
            .is_some()
    );

    // Another branch's ledger: it records no store.
    fs::write(
        p.root().join("specforge-harness"),
        "# specforge-harness 1\n",
    )
    .unwrap();
    let root = fs::canonicalize(p.root()).unwrap();
    let changes = session.changes([root.join("specforge-harness").as_path()]);

    assert_eq!(changes.sources(), [STORE]);
    assert!(session.apply(&changes).is_some());
    assert!(
        session
            .project()
            .graph()
            .node("js:a.test.js#adds")
            .is_none()
    );
    assert!(codes(session.project()).contains(&"W191".to_string()));
    assert!(session.stale().is_empty(), "{:?}", session.stale());
}

#[specforge_test_macros::test(
    behavior = "dispatch_collector",
    verify = "the recorded facts of a kind are those of the stores the ledger records, sorted"
)]
fn recorded_facts_are_the_ledgers_stores_of_the_kinds_asked() {
    let p = collected();
    stored(p.root(), "node", KEY, two_tests());
    // A store the ledger records with other bytes serves nothing.
    stored(p.root(), "other", KEY, two_tests());
    fs::write(
        p.root()
            .join(".specforge/facts")
            .join(KEY)
            .join("other.json"),
        "{}",
    )
    .unwrap();
    let disk = specforge_registry::PlainDisk;

    let facts = specforge_project::facts::recorded_facts(&disk, p.root(), &["test", "item"]);

    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["js:a.test.js#adds", "js:a.test.js#subtracts"]);
    assert!(specforge_project::facts::recorded_facts(&disk, p.root(), &["item"]).is_empty());
    assert!(specforge_project::facts::recorded_facts(&disk, p.root(), &[]).is_empty());
}

/// A place in `file` on `line`, from byte column `from` to `to`, as the wire writes it.
fn place(file: &str, line: usize, from: usize, to: usize) -> Value {
    json!({"file": file, "start_line": line, "start_col": from, "end_line": line, "end_col": to})
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a fact's reference that names no entity, or one of another kind, is dangling: no E003 or E022, and the passes are sent it at its name span"
)]
fn a_facts_dangling_reference_is_no_kernel_diagnostic_and_reaches_the_passes() {
    use specforge_extension_sdk::prelude::*;
    use specforge_project::testing::CODE;
    use specforge_wasm::testing::InProcessRuntime;

    // `@t/judge` reports each dangling reference a pass is sent, at the span it is sent.
    fn judge(input: &PassInput) -> Vec<PassDiagnostic> {
        input
            .entities
            .iter()
            .flat_map(|e| &e.dangling)
            .map(|d| {
                let reason = serde_json::to_value(d.reason).unwrap();
                let message = format!("{} {} {}", d.field, d.id, reason.as_str().unwrap());
                let span = d.span.clone().expect("its name span");
                PassDiagnostic::warning("W001", message).with_span(span)
            })
            .collect()
    }
    let judging = || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/judge", "1.0.0"));
        c.peer(CODE, "^1.0");
        c.code("W001", |d| {
            d.title("Dangling").explanation("Dangling.");
        });
        c.pass("judge", |p| {
            p.phase("check").run(judge);
        });
        c
    };
    let mut enabled = COLLECTED_EXTENSIONS.to_vec();
    enabled.push("@t/judge");
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&enabled)
        .installing_enabled();
    let facts = json!([
        {"kind": "test", "id": "js:a.test.js#adds", "fields": {"covers": ["nobody"]},
         "at": [place("a.test.js", 3, 1, 30)],
         "named_at": {"covers": {"nobody": [place("a.test.js", 2, 12, 18)]}}},
    ]);
    stored(p.root(), "node", KEY, facts);

    let project = p.compile_in(Arc::new(
        Collected::new().over(InProcessRuntime::new().with(judging)),
    ));

    let diagnostics = project.diagnostics();
    assert!(
        !diagnostics.iter().any(|d| d.code == "E003"),
        "{diagnostics:?}"
    );
    let judged: Vec<(&str, &str, usize, usize)> = diagnostics
        .iter()
        .filter_map(|d| {
            let span = d.span.as_ref()?;
            (d.code.ends_with("W001")).then(|| {
                (
                    d.message.as_str(),
                    span.file.as_str(),
                    span.start_line,
                    span.start_col,
                )
            })
        })
        .collect();
    assert_eq!(judged, [("covers nobody unresolved", "a.test.js", 2, 12)]);
}

#[specforge_test(
    behavior = "build_in_memory_graph",
    verify = "a finding on a fact is placed where it is written, not at its store"
)]
fn a_finding_on_a_fact_is_placed_where_it_is_written() {
    use specforge_extension_sdk::prelude::*;
    use specforge_project::testing::CODE;
    use specforge_wasm::testing::InProcessRuntime;

    // `@t/judge` reports every test, naming it and giving no span: the host places the finding.
    fn judge(input: &PassInput) -> Vec<PassDiagnostic> {
        input
            .entities
            .iter()
            .filter(|e| e.kind == "test")
            .map(|e| PassDiagnostic::warning("W001", "judged").with_entity(e.id.clone()))
            .collect()
    }
    let judging = || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/judge", "1.0.0"));
        c.peer(CODE, "^1.0");
        c.code("W001", |d| {
            d.title("Judged").explanation("Judged.");
        });
        c.pass("judge", |p| {
            p.phase("check").run(judge);
        });
        c
    };
    let mut enabled = COLLECTED_EXTENSIONS.to_vec();
    enabled.push("@t/judge");
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&enabled)
        .installing_enabled();
    let facts = json!([
        {"kind": "test", "id": "js:a.test.js#adds", "at": [place("a.test.js", 7, 3, 40)]},
        {"kind": "test", "id": "js:a.test.js#subtracts"},
    ]);
    stored(p.root(), "node", KEY, facts);

    let project = p.compile_in(Arc::new(
        Collected::new().over(InProcessRuntime::new().with(judging)),
    ));

    let placed: Vec<(String, usize)> = project
        .diagnostics()
        .iter()
        .filter(|d| d.message == "judged")
        .map(|d| {
            let span = d.span.as_ref().expect("placed");
            (span.file.as_str().to_string(), span.start_line)
        })
        .collect();
    assert_eq!(
        placed,
        [(STORE.to_string(), 1), ("a.test.js".to_string(), 7)],
        "a located fact's finding is at its place, an unlocated one's at its store"
    );
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "a fact id two stores hold is E117 naming both collectors, not E002"
)]
fn a_fact_id_two_stores_hold_is_e117_naming_both_collectors() {
    let p = collected();
    let fact = json!([{"kind": "test", "id": "js:a.test.js#adds"}]);
    stored(p.root(), "node", KEY, fact.clone());
    stored(p.root(), "unit", KEY, fact);

    let project = compiled(&p);

    let diagnostics = project.diagnostics();
    assert!(
        !codes(&project).contains(&"E002".to_string()),
        "{diagnostics:?}"
    );
    let e117: Vec<_> = diagnostics.iter().filter(|d| d.code == "E117").collect();
    assert_eq!(e117.len(), 1, "{diagnostics:?}");
    let unit = STORE.replace("node.json", "unit.json");
    assert_eq!(
        e117[0].message,
        format!(
            "fact 'js:a.test.js#adds' is collected by both 'node' ({STORE}) and 'unit' ({unit}); the first is served"
        )
    );
    assert_eq!(
        e117[0].suggestion.as_deref(),
        Some(
            "two collectors mint one id: load only one of them, or fix the pack that mints it, then run \
             `specforge collect`"
        )
    );
}

#[test]
fn a_bare_fact_scheme_reference_is_one_e120_and_no_e003() {
    let p = collected().file(
        "truths.spec",
        "truth T1 \"One\" {\n  proof [js:p1, run:x]\n}\n",
    );

    let project = compiled(&p);

    let diagnostics = project.diagnostics();
    let found = codes(&project);
    assert_eq!(found, ["E120", "E120"], "{diagnostics:?}");
    assert_eq!(
        diagnostics[0].message,
        "a reference holding ':' is written quoted: \"js:p1\""
    );
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some("write \"js:p1\"")
    );
}
