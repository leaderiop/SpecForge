//! A human-only kind, field or member is refused in every agent-facing write (ADR 0289); `override` of the
//! `@t/core` fixture is one.

use std::sync::Arc;

use serde_json::{Map, json};
use specforge_ops::authoring::apply::{PlanOutcome, Surface, execute};
use specforge_ops::extension::command::{CommandFormat, ExtensionCommands, RunError};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GsdWrites};
use specforge_test_macros::test as specforge_test;

use crate::plans::{compiled, project, refused, revision, serving, set};

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
    typed_as(
        p,
        edits,
        &specforge_ops::authoring::apply::testing::origin(),
    )
}

fn typed_as(
    p: &specforge_project::testing::TestProject,
    edits: &[specforge_ops::authoring::edit::Edit],
    origin: &specforge_ops::authoring::apply::Origin,
) -> Option<String> {
    let compile = p.compile_in(Arc::new(GsdWrites::new().runtime()));
    specforge_ops::authoring::edit::run(
        &ProjectView::of(&compile),
        edits,
        true,
        origin,
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

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "an agents set, unset or link of an existing human-only entity is refused"
)]
fn an_agents_edit_of_an_existing_human_only_entity_is_refused() {
    use crate::edit::{link, one, set, unset};
    use specforge_ops::authoring::apply::Origin;
    let p = project().file(
        "o.spec",
        "override O1 \"O\" {\n  by \"me\"\n  phase 03\n}\n",
    );
    let human = |code: Option<String>, what: &str| {
        assert_eq!(code.as_deref(), Some("human_only"), "{what}")
    };
    human(typed(&p, &[set("O1", "at", one("now"))]), "set");
    human(typed(&p, &[unset("O1", "by")]), "unset");
    human(typed(&p, &[link("O1", "phase", "03")]), "link");
    // A person at a terminal edits it.
    let terminal = Origin::cli(true);
    assert_eq!(
        typed_as(&p, &[set("O1", "at", one("now"))], &terminal),
        None
    );
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "an agents delete of an entity holding a persons value is refused"
)]
fn an_agents_delete_of_an_entity_holding_a_persons_value_is_refused() {
    use specforge_ops::authoring::apply::Origin;
    use specforge_ops::authoring::edit::Edit;
    let delete = |id: &str| Edit::Delete {
        entity_id: id.into(),
        unlink_references: false,
    };
    let p = project()
        .file(
            "r.spec",
            "reviewed R \"R\" {\n  approvals [\n    {\n      by \"me\"\n    }\n  ]\n}\n",
        )
        .file("r2.spec", "reviewed R2 \"R2\" {}\n");
    let before = p.read("r.spec");
    assert_eq!(typed(&p, &[delete("R")]).as_deref(), Some("human_only"));
    assert_eq!(p.read("r.spec"), before);
    assert_eq!(typed(&p, &[delete("R2")]), None);
    assert_eq!(typed_as(&p, &[delete("R")], &Origin::cli(true)), None);
}

const HELD: &str = "reviewed R \"R\" {\n  approvals [\n    {\n      by \"me\"\n    }\n  ]\n}\n";

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a plan that renames an entity and deletes it under the new id is refused as the delete would be"
)]
fn a_rename_then_a_delete_in_one_plan_is_checked_against_the_original() {
    let p = project()
        .file("r.spec", HELD)
        .file("a.spec", "reviewed A \"A\" {}\n")
        .file("r2.spec", "reviewed R2 \"R2\" {}\n");
    let plan = |ops: serde_json::Value| refused(&p, CORE, "core", "record", ops);
    let rename = |from: &str, to: &str, file: &str| json!({"op": "rename", "entity": from, "to": to, "revision": crate::plans::revision(&p, file)});
    let delete = |id: &str| json!({"op": "delete", "entity": id, "revision": "absent"});
    assert_eq!(
        plan(json!([rename("R", "R9", "r.spec"), delete("R9")])).as_deref(),
        Some("human_only")
    );
    // Delete a harmless A, take its id with R, delete again: the second delete is R's.
    assert_eq!(
        plan(json!([
            delete("A"),
            rename("R", "A", "r.spec"),
            delete("A")
        ]))
        .as_deref(),
        Some("human_only")
    );
    // An entity holding nothing of a person's is renamed and deleted freely.
    assert_eq!(
        plan(json!([rename("R2", "R3", "r2.spec"), delete("R3")])),
        None
    );
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "an agents unset, remove or replacing set of a persons member value is refused"
)]
fn an_agents_write_that_drops_a_persons_member_value_is_refused() {
    use crate::edit::{set, unset};
    use specforge_ops::authoring::apply::Origin;
    use specforge_ops::authoring::edit::{Edit, Input};
    use std::collections::BTreeMap;
    let p = project().file("r.spec", HELD).file(
        "n.spec",
        "reviewed N \"N\" {\n  approvals [\n    {\n      note \"x\"\n    }\n  ]\n}\n",
    );
    let note = Input::Blocks(vec![BTreeMap::from([(
        "note".to_string(),
        Input::One("x".into()),
    )])]);
    let remove = Edit::Remove {
        entity_id: "R".into(),
        field: "approvals".into(),
        index: 0,
    };
    for (what, edit) in [
        ("unset", unset("R", "approvals")),
        ("remove", remove.clone()),
        ("replacing set", set("R", "approvals", note.clone())),
    ] {
        assert_eq!(typed(&p, &[edit]).as_deref(), Some("human_only"), "{what}");
    }
    // A person at a terminal edits it, and a value no person wrote is dropped freely.
    let terminal = Origin::cli(true);
    assert_eq!(typed_as(&p, &[unset("R", "approvals")], &terminal), None);
    assert_eq!(typed_as(&p, &[remove], &terminal), None);
    assert_eq!(typed(&p, &[unset("N", "approvals")]), None);
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "an agent deletes an entity holding only computed or harness values"
)]
fn a_delete_of_an_entity_with_only_computed_or_harness_values_is_not_refused() {
    use specforge_ops::authoring::edit::Edit;
    let p = project().file(
        "u.spec",
        "uat_test U9 \"U\" {\n  phase 03\n  result pass\n}\n",
    );
    let delete = Edit::Delete {
        entity_id: "v1.0/03/U9".into(),
        unlink_references: false,
    };
    assert_eq!(typed(&p, &[delete]), None);
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "an agent still writes a non-human field of a non-human kind"
)]
fn an_agent_still_writes_a_non_human_field_of_a_non_human_kind() {
    use crate::edit::{set, unset};
    use specforge_ops::authoring::edit::{Edit, Input};
    use std::collections::BTreeMap;
    let note = |text: &str| BTreeMap::from([("note".to_string(), Input::One(text.into()))]);
    let p = project().file("n.spec", "reviewed N \"N\" {}\n");
    assert_eq!(
        typed(&p, &[set("N", "approvals", Input::Blocks(vec![note("a")]))]),
        None,
        "set"
    );
    let append = Edit::Append {
        entity_id: "N".into(),
        field: "approvals".into(),
        value: Input::Block(note("b")),
    };
    assert_eq!(typed(&p, &[append]), None, "append");
    assert_eq!(typed(&p, &[unset("N", "approvals")]), None, "unset");
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a human-attested command is refused on every agent surface and runs for a person at a terminal"
)]
fn a_human_command_is_refused_on_every_agent_surface() {
    let p = project();
    let ops = json!([set(
        "v1.0/03/U1",
        "result",
        json!("pass"),
        &revision(&p, "03-01.spec")
    )]);
    let runtime = serving(&[(CORE, "cmd__attest", 0, ops)]);
    let built = compiled(&p, &runtime);
    let view = ProjectView::of(&built);
    let commands = ExtensionCommands::build(view.registries());
    let command = commands
        .of("core")
        .find(|c| c.id() == "attest")
        .expect("the human command is served");
    let run = |surface| {
        execute(
            &view,
            command,
            &Map::new(),
            CommandFormat::Json,
            false,
            specforge_ops::testing::caller(),
            surface,
        )
    };
    for surface in [Surface::Mcp, Surface::Cli { interactive: false }] {
        match run(surface) {
            Err(RunError::Gate(error)) => assert_eq!(error.code, "human_only", "{surface:?}"),
            other => panic!("{surface:?}: {other:?}"),
        }
    }
    assert!(runtime.calls().iter().all(|c| c.export != "cmd__attest"));
    assert!(p.read("03-01.spec").find("result").is_none());
    let person = run(Surface::Cli { interactive: true }).expect("a person at a terminal");
    assert!(
        matches!(person.plan, Some(PlanOutcome::Applied(_))),
        "{person:?}"
    );
}
