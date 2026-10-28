//! A human-only kind, field or member is refused in every agent-facing write (ADR 0289); `override` of the
//! `@t/core` fixture is one.

use std::sync::Arc;

use serde_json::json;
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GsdWrites};
use specforge_test_macros::test as specforge_test;

use crate::plans::{project, refused};

const OVERRIDE: &str = "override O1 \"O\" {\n  phase 03\n}\n";

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a human-only value is refused in an agents write"
)]
fn a_human_only_value_is_refused_in_an_agents_write() {
    let p = project();
    // From a command's plan.
    let create = json!([{"op": "create", "kind": "override", "id": "O1", "title": "O",
        "file": "o.spec", "revision": "absent", "fields": {"phase": "03"}}]);
    assert_eq!(
        refused(&p, CORE, "core", "record", create).as_deref(),
        Some("human_only")
    );
    assert!(!p.root().join("o.spec").exists());
    // From a typed edit, which MCP `specforge.edit` and the CLI without a terminal both are.
    let compile = p.compile_in(Arc::new(GsdWrites::new().runtime()));
    let edit = crate::edit::create("override", "O1", "o.spec", &[]);
    let error = specforge_ops::authoring::edit::run(
        &ProjectView::of(&compile),
        &[edit],
        true,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    assert_eq!(error.error.code, "human_only");
    // By hand it compiles clean.
    let by_hand = project().file("o.spec", OVERRIDE);
    let errors: Vec<_> = by_hand
        .compile_in(Arc::new(GsdWrites::new().runtime()))
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == "E080")
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

fn typed(
    p: &specforge_project::testing::TestProject,
    edits: &[specforge_ops::authoring::edit::Edit],
) -> Option<String> {
    let compile = p.compile_in(Arc::new(GsdWrites::new().runtime()));
    specforge_ops::authoring::edit::run(
        &ProjectView::of(&compile),
        edits,
        true,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .err()
    .map(|e| e.error.code.to_string())
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a human-only or computed member is refused in the value of a set, append or create"
)]
fn a_human_only_or_computed_member_is_refused_in_the_value_of_a_write() {
    use crate::edit::{create, set};
    use specforge_ops::authoring::edit::{Edit, Input};
    use std::collections::BTreeMap;
    let block = |pairs: &[(&str, &str)]| -> BTreeMap<String, Input> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Input::One(v.to_string())))
            .collect()
    };
    let p = project()
        .file("r.spec", "reviewed R \"R\" {}\n")
        .file("d.spec", "deliverable D \"D\" {}\n");
    let by = Input::Blocks(vec![block(&[("by", "me")])]);
    assert_eq!(
        typed(&p, &[set("R", "approvals", by.clone())]).as_deref(),
        Some("human_only"),
        "set"
    );
    let append = Edit::Append {
        entity_id: "R".into(),
        field: "approvals".into(),
        value: Input::Block(block(&[("by", "me")])),
    };
    assert_eq!(
        typed(&p, &[append]).as_deref(),
        Some("human_only"),
        "append"
    );
    assert_eq!(
        typed(
            &p,
            &[create("reviewed", "R2", "r2.spec", &[("approvals", by)])]
        )
        .as_deref(),
        Some("human_only"),
        "create"
    );
    let status = Input::Blocks(vec![block(&[
        ("kind", "a"),
        ("ref", "b"),
        ("status", "passed"),
    ])]);
    assert_eq!(
        typed(&p, &[set("D", "checks", status)]).as_deref(),
        Some("computed_field")
    );
    // The members a person does not own are written.
    let note = Input::Blocks(vec![block(&[("note", "ok")])]);
    assert_eq!(typed(&p, &[set("R", "approvals", note)]), None);
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a delete that unlinks references and a rename do not change a human-only entity"
)]
fn a_delete_that_unlinks_and_a_rename_do_not_change_a_human_only_entity() {
    use specforge_ops::authoring::edit::Edit;
    use specforge_ops::authoring::rename;
    use specforge_ops::read::navigate::Navigator;
    let p = project()
        .file("p.spec", "phase 04 \"Four\" {\n  milestone v1.0\n}\n")
        .file("o.spec", "override O1 \"O\" {\n  phase v1.0/04\n}\n");
    let delete = Edit::Delete {
        entity_id: "v1.0/04".into(),
        unlink_references: true,
    };
    assert_eq!(typed(&p, &[delete]).as_deref(), Some("human_only"));
    let compile = p.compile_in(Arc::new(GsdWrites::new().runtime()));
    let error = rename::run(
        &Navigator::new(ProjectView::of(&compile)),
        "v1.0/04",
        "05",
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    assert_eq!(error.error.code, "human_only");
    assert!(p.read("o.spec").contains("phase v1.0/04"));
}
