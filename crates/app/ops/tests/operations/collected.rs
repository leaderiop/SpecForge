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
use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, NODE, TestProject, stored};
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
    verify = "a rename of a collected entity, or of one a fact names, is refused with why"
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
    verify = "a rename of a collected entity, or of one a fact names, is refused with why"
)]
fn a_rename_of_an_entity_a_collected_fact_names_is_refused() {
    let p = project();
    let compiled = compiled(&p);
    let nav = Navigator::new(ProjectView::of(&compiled));

    let refused = rename::plan(&nav, "T1", "T2").unwrap_err();

    assert_eq!(refused.error.code, COLLECTED);
    assert_eq!(
        refused.error.message,
        format!(
            "T1 is named by 'covers' of the fact '{FACT}', which an edit cannot rewrite; rename it, then run \
             `specforge collect`"
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
