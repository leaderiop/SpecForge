//! A collected entity, a fact (ADR 0327), is written by `specforge collect` alone: a typed edit, a new, a delete, a
//! rename and a plan refuse it for every caller (`collected`), the editor's and a person's at a terminal included,
//! and so does a rename of an entity a fact names. The fixture is `Collected` with `node`'s store served.

use std::sync::Arc;

use serde_json::json;
use specforge_ops::OpErrorKind;
use specforge_ops::authoring::apply::{COLLECTED, Origin, Surface, apply_plan};
use specforge_ops::authoring::edit::{Edit, Input};
use specforge_ops::authoring::rename::{self, RenameField};
use specforge_ops::read::navigate::Navigator;
use specforge_ops::testing::caller;
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_project::harness::{Ledger, Line};
use specforge_project::testing::{
    COLLECTED_EXTENSIONS, Collected, NODE, TestProject, stored, stored_with,
};
use specforge_test_macros::test as specforge_test;

const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const FACT: &str = "js:a.test.js#adds";
const STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";

/// A project whose truth `T1` cites the fact `FACT`, which names `T1` back, with `node`'s store recorded.
fn project() -> TestProject {
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file(
            "truths.spec",
            format!("truth T1 \"One\" {{\n  proof [\"{FACT}\"]\n}}\n"),
        );
    stored(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": FACT, "fields": {"file": "a.test.js", "covers": ["T1"]}}]),
    );
    p
}

fn compiled(p: &TestProject) -> CompiledProject {
    let compiled = p.compile_in(Arc::new(Collected::new().runtime()));
    assert!(
        compiled.graph().node(FACT).is_some(),
        "the store is served: {:?}",
        compiled.diagnostics()
    );
    compiled
}

/// Every core caller: the editor, a person at a terminal, the command line with nobody at one, MCP.
fn everyone() -> [Origin; 4] {
    [
        Origin::Core {
            surface: Surface::Lsp,
        },
        Origin::cli(true),
        Origin::cli(false),
        Origin::Core {
            surface: Surface::Mcp,
        },
    ]
}

/// The refusal of `edits` asked by `origin`: its code, kind and message.
fn typed(
    compiled: &CompiledProject,
    edits: &[Edit],
    origin: &Origin,
) -> (String, OpErrorKind, String) {
    let error = specforge_ops::authoring::edit::run(
        &ProjectView::of(compiled),
        edits,
        true,
        origin,
        caller(),
    )
    .expect_err("a fact is no edit's to write");
    (
        error.error.code.to_string(),
        error.error.kind,
        error.error.message.clone(),
    )
}

#[specforge_test(
    behavior = "validate_typed_edit",
    verify = "a collected entity is never created, edited or deleted by an edit, whoever asks"
)]
fn a_typed_edit_of_a_collected_entity_is_refused_for_every_caller() {
    let p = project();
    let compiled = compiled(&p);
    let edits = [
        crate::edit::set(FACT, "status", Input::One("failed".into())),
        crate::edit::unset(FACT, "status"),
        Edit::Retitle {
            entity_id: FACT.into(),
            title: "Adds".into(),
        },
    ];

    for origin in everyone() {
        for edit in &edits {
            let (code, kind, message) = typed(&compiled, std::slice::from_ref(edit), &origin);
            assert_eq!(code, COLLECTED, "{origin:?} {edit:?}");
            assert_eq!(kind, OpErrorKind::PermissionDenied);
            assert_eq!(
                message,
                format!(
                    "test '{FACT}' is a fact collected by 'node' ({STORE}): it changes when its source does and \
                     `specforge collect` runs, never by an edit"
                )
            );
        }
    }
    assert!(
        !std::fs::read_to_string(p.root().join(STORE))
            .unwrap()
            .contains("Adds")
    );
}

#[specforge_test(
    behavior = "create_entity",
    verify = "a collected entity is never created, edited or deleted by an edit, whoever asks"
)]
fn new_of_a_collected_kind_is_refused() {
    let p = project();
    let compiled = compiled(&p);

    for origin in everyone() {
        let (code, _, message) = typed(
            &compiled,
            &[crate::edit::create("test", "js:b.test.js#x", "b.spec", &[])],
            &origin,
        );
        assert_eq!(code, COLLECTED);
        assert_eq!(
            message,
            "test entities are facts a collector writes: `specforge collect` gives them"
        );
    }
    assert!(!p.root().join("b.spec").exists());
}

#[specforge_test(
    behavior = "delete_entity",
    verify = "a collected entity is never created, edited or deleted by an edit, whoever asks"
)]
fn a_delete_of_a_collected_entity_is_refused() {
    let p = project();
    let compiled = compiled(&p);

    for unlink_references in [false, true] {
        let delete = Edit::Delete {
            entity_id: FACT.into(),
            unlink_references,
        };
        let (code, _, _) = typed(&compiled, &[delete], &Origin::cli(true));
        assert_eq!(code, COLLECTED);
    }
    // Unlinking the references to `T1` would rewrite the fact that names it.
    let delete = Edit::Delete {
        entity_id: "T1".into(),
        unlink_references: true,
    };
    let (code, _, message) = typed(&compiled, &[delete], &Origin::cli(true));
    assert_eq!(code, COLLECTED);
    assert!(
        message.contains("of the fact 'js:a.test.js#adds'"),
        "{message}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of a collected entity, or of one a fact names with no place, is refused with why"
)]
fn a_rename_of_a_collected_entity_is_refused_with_why() {
    let p = project();
    let compiled = compiled(&p);
    let view = ProjectView::of(&compiled);
    let nav = Navigator::new(view);

    let planned = rename::plan(&nav, FACT, "renamed").unwrap_err();
    assert_eq!(planned.error.code, COLLECTED);
    // A permission refusal rejects no value of the request: it names no field.
    assert_eq!(planned.subject, None::<RenameField>);
    assert!(
        planned.error.message.contains("collected by 'node'"),
        "{}",
        planned.error.message
    );
    for origin in everyone() {
        let ran = rename::run(&nav, FACT, "renamed", false, &origin, caller()).unwrap_err();
        assert_eq!(ran.error.code, COLLECTED, "{origin:?}");
    }
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of a collected entity, or of one a fact names with no place, is refused with why"
)]
fn a_rename_of_an_entity_a_fact_names_with_no_place_is_refused() {
    let p = project();
    let compiled = compiled(&p);
    let nav = Navigator::new(ProjectView::of(&compiled));

    let refused = rename::plan(&nav, "T1", "T2").unwrap_err();

    assert_eq!(refused.error.code, COLLECTED);
    assert_eq!(
        refused.error.message,
        format!(
            "T1 is named by 'covers' of the fact '{FACT}', which records no place it is written; rename it in the \
             source, then run `specforge collect`"
        )
    );
    let ran = rename::run(&nav, "T1", "T2", false, &Origin::cli(true), caller()).unwrap_err();
    assert_eq!(ran.error.code, COLLECTED);
    assert!(
        std::fs::read_to_string(p.root().join("truths.spec"))
            .unwrap()
            .contains("T1")
    );
}

/// The test file the collector read: `T1` is written at line 1, columns 11..13, and again at line 3 (a cfg twin).
const SOURCE: &str = "// covers T1\ntest('adds', () => {});\n// covers T1\n";

/// [`project`] whose fact records where `T1` is written in `a.test.js`, at `places` of [`SOURCE`]'s lines, and the
/// file's digest as the collect read it.
fn located(lines: &[usize]) -> TestProject {
    let p = project();
    std::fs::write(p.root().join("a.test.js"), SOURCE).unwrap();
    let places: Vec<_> = lines
        .iter()
        .map(|line| json!({"file": "a.test.js", "start_line": line, "start_col": 11, "end_line": line, "end_col": 13}))
        .collect();
    let sources = [(
        "a.test.js".to_string(),
        specforge_ops::revision::of_bytes(SOURCE.as_bytes()),
    )]
    .into();
    stored_with(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": FACT, "fields": {"file": "a.test.js", "covers": ["T1"]},
                "at": [{"file": "a.test.js", "start_line": 2, "start_col": 1, "end_line": 2, "end_col": 23}],
                "named_at": {"covers": {"T1": places}}}]),
        sources,
    );
    p
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of an entity a fact names rewrites the source where the fact read it and marks its store stale"
)]
fn a_rename_of_an_entity_a_fact_names_rewrites_the_source_and_marks_the_store_stale() {
    let p = located(&[1]);
    let compiled = compiled(&p);
    let nav = Navigator::new(ProjectView::of(&compiled));
    let store = std::fs::read(p.root().join(STORE)).unwrap();

    let renamed = rename::run(&nav, "T1", "T2", false, &Origin::cli(true), caller()).unwrap();

    assert_eq!(
        renamed.plan.change().affected_files().collect::<Vec<_>>(),
        ["truths.spec", "a.test.js", "specforge-harness"]
    );
    let source = std::fs::read_to_string(p.root().join("a.test.js")).unwrap();
    assert_eq!(source, SOURCE.replacen("T1", "T2", 1));
    let ledger =
        Ledger::parse(&std::fs::read_to_string(p.root().join("specforge-harness")).unwrap()).ledger;
    assert!(
        ledger.contains(&Line::moved("T2", "T1")),
        "{}",
        ledger.canonical()
    );
    assert!(
        ledger.contains(&Line::stale("node", KEY, &store, "covers", "T1", "T2")),
        "{}",
        ledger.canonical()
    );
    // The store is the collector's bytes still; a recompile follows the move, and says the store is stale.
    assert_eq!(std::fs::read(p.root().join(STORE)).unwrap(), store);
    let after = compiled_quietly(&p);
    let covered: Vec<&str> = after
        .graph()
        .edges_from(FACT)
        .iter()
        .map(|e| e.target.as_str())
        .collect();
    assert_eq!(covered, ["T2"], "{:?}", after.diagnostics());
    assert!(
        after.diagnostics().iter().any(|d| d.code == "W197"),
        "{:?}",
        after.diagnostics()
    );
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename whose source changed since the collect is refused as source_moved"
)]
fn a_rename_whose_source_moved_since_the_collect_is_source_moved() {
    let p = located(&[1]);
    std::fs::write(p.root().join("a.test.js"), format!("\n{SOURCE}")).unwrap();
    let compiled = compiled(&p);
    let nav = Navigator::new(ProjectView::of(&compiled));

    let refused = rename::run(&nav, "T1", "T2", false, &Origin::cli(true), caller()).unwrap_err();

    assert_eq!(refused.error.code, rename::SOURCE_MOVED);
    assert_eq!(
        refused.error.message,
        "cannot rename 'T1': a.test.js changed since `specforge collect` read it"
    );
    assert!(
        std::fs::read_to_string(p.root().join("truths.spec"))
            .unwrap()
            .contains("T1")
    );
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        format!("\n{SOURCE}")
    );
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename whose source changed since the collect is refused as source_moved"
)]
fn a_second_rename_before_the_collect_is_source_moved() {
    let p = located(&[1]);
    let first = compiled(&p);
    rename::run(
        &Navigator::new(ProjectView::of(&first)),
        "T1",
        "T2",
        false,
        &Origin::cli(true),
        caller(),
    )
    .unwrap();
    let renamed = std::fs::read_to_string(p.root().join("a.test.js")).unwrap();
    let second = compiled_quietly(&p);
    let nav = Navigator::new(ProjectView::of(&second));

    // The store still holds `T1`; served, it names `T2` at the place the first rename rewrote.
    let refused = rename::run(&nav, "T2", "T3", false, &Origin::cli(true), caller()).unwrap_err();

    assert_eq!(refused.error.code, rename::SOURCE_MOVED);
    assert_eq!(
        refused.error.message,
        "cannot rename 'T2': a.test.js changed since `specforge collect` read it"
    );
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        renamed
    );
    assert!(
        std::fs::read_to_string(p.root().join("truths.spec"))
            .unwrap()
            .contains("T2")
    );
}

#[test]
fn a_plans_second_rename_of_a_name_in_a_rewritten_source_is_source_rewritten() {
    // `a.test.js` writes `T1` on line 1 and `T5` on line 2; one plan renames both.
    let p = project();
    p.write(
        "truths.spec",
        format!("truth T1 \"One\" {{\n  proof [\"{FACT}\"]\n}}\n\ntruth T5 \"Five\" {{}}\n"),
    );
    let source = "// covers T1\n// covers T5\n";
    std::fs::write(p.root().join("a.test.js"), source).unwrap();
    let at = |line: usize| json!([{"file": "a.test.js", "start_line": line, "start_col": 11, "end_line": line, "end_col": 13}]);
    stored_with(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": FACT, "fields": {"file": "a.test.js", "covers": ["T1", "T5"]},
                "named_at": {"covers": {"T1": at(1), "T5": at(2)}}}]),
        [(
            "a.test.js".to_string(),
            specforge_ops::revision::of_bytes(source.as_bytes()),
        )]
        .into(),
    );
    let compiled = compiled(&p);
    let revision = crate::plans::revision(&p, "truths.spec");
    let plan = serde_json::from_value(json!({"ops": [
        {"op": "rename", "entity": "T1", "to": "T2", "revision": revision},
        {"op": "rename", "entity": "T5", "to": "T6", "revision": revision},
    ]}))
    .unwrap();

    let refused = apply_plan(
        &ProjectView::of(&compiled),
        &plan,
        &Origin::cli(true),
        false,
        caller(),
    )
    .unwrap_err();

    assert_eq!(refused.error.code, rename::SOURCE_REWRITTEN);
    assert!(
        refused
            .error
            .message
            .ends_with("cannot rename 'T5': an earlier rename of the plan rewrites a.test.js"),
        "{}",
        refused.error.message
    );
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        source
    );
}

#[test]
fn a_rename_rewrites_every_place_of_a_cfg_twin() {
    let p = located(&[1, 3]);
    let compiled = compiled(&p);
    let nav = Navigator::new(ProjectView::of(&compiled));

    let plan = rename::plan(&nav, "T1", "T2").unwrap();

    let source: Vec<(usize, &str)> = plan
        .edits()
        .iter()
        .filter(|e| e.span.file == "a.test.js")
        .map(|e| (e.span.start_line, e.new_text.as_str()))
        .collect();
    assert_eq!(source, [(1, "T2"), (3, "T2")]);
    rename::run(&nav, "T1", "T2", false, &Origin::cli(true), caller()).unwrap();
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        SOURCE.replace("T1", "T2")
    );
}

#[test]
fn a_delete_of_an_entity_a_fact_names_is_still_refused() {
    let p = located(&[1]);
    let compiled = compiled(&p);

    let delete = Edit::Delete {
        entity_id: "T1".into(),
        unlink_references: true,
    };
    let (code, _, message) = typed(&compiled, &[delete], &Origin::cli(true));

    assert_eq!(code, COLLECTED);
    assert!(
        message.ends_with(
            "which an edit cannot rewrite; change the source it is read from, run `specforge collect`, then delete it"
        ),
        "{message}"
    );
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        SOURCE
    );
}

/// `p` compiled as [`compiled`] does, without asserting it is clean.
fn compiled_quietly(p: &TestProject) -> CompiledProject {
    p.compile_in(Arc::new(Collected::new().runtime()))
}

#[test]
fn a_collectors_plan_that_creates_a_collected_entity_is_refused() {
    let p = project();
    let compiled = compiled(&p);
    let plan = serde_json::from_value(json!({"ops": [
        {"op": "create", "kind": "test", "id": "js:b.test.js#x", "title": "x", "file": "b.spec",
         "revision": "absent", "fields": {}},
    ]}))
    .unwrap();
    let origin = Origin::Collector {
        extension: NODE.into(),
    };

    let refused =
        apply_plan(&ProjectView::of(&compiled), &plan, &origin, false, caller()).unwrap_err();

    assert_eq!(refused.error.code, COLLECTED);
    assert_eq!(refused.error.data.as_deref(), Some(&json!({"op": 1})));
    assert!(
        refused.error.message.starts_with("operation 1: "),
        "{}",
        refused.error.message
    );
    assert!(!p.root().join("b.spec").exists());
}
