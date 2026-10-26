//! A selective `use` binding's imported name is a written token (ADR 0233): rename edits it, a delete with
//! `unlink_references` removes it, and an alias names nothing.

use crate::edit::{is_canonical, plan, project};
use crate::navigate::{SOFTWARE, at, compile};
use specforge_ops::authoring::edit::Edit;
use specforge_ops::authoring::rename;
use specforge_ops::read::navigate::{Direction, ReferenceQuery};
use specforge_test_macros::test as specforge_test;

const RULES: &str = "invariant inv_a \"A\" {\n  guarantee \"g\"\n}\n\ninvariant inv_b \"B\" {\n  guarantee \"g\"\n}\n";
const BOUND: &str = "use { inv_a, inv_b } from \"rules.spec\"\n\nbehavior login \"Login\" {\n  category   command\n  contract   \"c\"\n  invariants [inv_a, inv_b]\n}\n";
const ALIASED: &str = "use { inv_a as rule } from \"rules.spec\"\n";
const LAST: &str = "use { inv_b, inv_a } from \"rules.spec\"\n";

fn delete(id: &str, unlink_references: bool) -> Edit {
    Edit::Delete {
        entity_id: id.into(),
        unlink_references,
    }
}

#[specforge_test(
    behavior = "rename_entity_id",
    verify = "rename edits a use binding's imported name, never its alias"
)]
fn rename_edits_a_use_binding_and_never_its_alias() {
    let p = compile(
        SOFTWARE,
        &[
            ("rules.spec", RULES),
            ("b.spec", BOUND),
            ("alias.spec", ALIASED),
        ],
    );
    let plan = rename::plan(&p.navigator(), "inv_a", "inv_z").unwrap();
    let mut edits: Vec<String> = plan.edits().iter().map(|e| at(&e.span)).collect();
    edits.sort();
    assert_eq!(
        edits,
        [
            "alias.spec 1:7-1:12",
            "b.spec 1:7-1:12",
            "b.spec 6:15-6:20",
            "rules.spec 1:11-1:16"
        ]
    );
    assert_eq!(
        plan.change().after()["alias.spec"],
        "use { inv_z as rule } from \"rules.spec\"\n"
    );
}

#[specforge_test(
    behavior = "delete_entity",
    verify = "delete with unlink_references removes a use binding that imports it"
)]
fn delete_with_unlink_references_removes_a_use_binding() {
    let c = project(&[
        ("rules.spec", RULES),
        ("b.spec", BOUND),
        ("alias.spec", ALIASED),
        ("last.spec", LAST),
    ]);
    let planned = plan(&c, &[delete("inv_a", true)]).unwrap();
    let after = planned.change().after();
    assert!(
        after["b.spec"].starts_with("use { inv_b } from \"rules.spec\"\n\nbehavior login"),
        "{}",
        after["b.spec"]
    );
    assert!(
        after["b.spec"].contains("invariants [inv_b]"),
        "{}",
        after["b.spec"]
    );
    assert_eq!(after["alias.spec"], "");
    assert_eq!(after["last.spec"], "use { inv_b } from \"rules.spec\"\n");
    assert!(is_canonical(&c, &after["b.spec"]));
}

#[test]
fn a_rename_onto_a_use_binding_naming_nothing_is_refused() {
    let p = compile(
        SOFTWARE,
        &[
            ("rules.spec", RULES),
            ("c.spec", "use { inv_z } from \"rules.spec\"\n"),
        ],
    );
    let refusal = rename::plan(&p.navigator(), "inv_a", "inv_z").unwrap_err();
    assert_eq!(refusal.error.code.to_string(), rename::CAPTURED);
    assert!(
        refusal.error.message.contains("c.spec:1 (use, rules.spec)"),
        "{}",
        refusal.error.message
    );
}

#[test]
fn the_editors_tokens_hold_a_use_binding_and_find_references_no_holder() {
    let p = compile(SOFTWARE, &[("rules.spec", RULES), ("b.spec", BOUND)]);
    let nav = p.navigator();
    let found: Vec<String> = nav
        .tokens("inv_a", false)
        .unwrap()
        .iter()
        .map(|t| at(&t.span))
        .collect();
    assert!(found.contains(&"b.spec 1:7-1:12".to_string()), "{found:?}");
    let refs = nav
        .references(
            "inv_a",
            ReferenceQuery {
                direction: Direction::Incoming,
                include_declaration: false,
            },
        )
        .unwrap();
    assert!(
        refs.iter()
            .all(|o| !(o.span.file == "b.spec" && o.span.start_line == 1)),
        "{refs:?}"
    );
}

#[test]
fn a_delete_that_leaves_a_use_binding_reports_w027() {
    let c = project(&[
        ("rules.spec", RULES),
        ("c.spec", "use { inv_b } from \"rules.spec\"\n"),
    ]);
    let planned = plan(&c, &[delete("inv_b", false)]).unwrap();
    let added = &planned.report().added.0;
    let w027: Vec<_> = added.iter().filter(|d| d.code == "W027").collect();
    assert_eq!(w027.len(), 1, "{added:?}");
    assert!(w027[0].message.contains("inv_b"), "{}", w027[0].message);
}

#[specforge_test(
    behavior = "delete_entity",
    verify = "delete with unlink_references removes the comment above a use statement it removes, never the format header"
)]
fn delete_with_unlink_references_removes_the_comment_above_a_use_it_removes() {
    let doc = "// specforge-format: 1.0\n// the rule login needs\nuse { inv_a } from \"rules.spec\"\n\nbehavior login \"Login\" {\n  category   command\n  contract   \"c\"\n  invariants [inv_a, inv_b]\n}\n";
    let c = project(&[("rules.spec", RULES), ("doc.spec", doc)]);
    let planned = plan(&c, &[delete("inv_a", true)]).unwrap();
    assert_eq!(
        planned.change().after()["doc.spec"],
        "// specforge-format: 1.0\nbehavior login \"Login\" {\n  category   command\n  contract   \"c\"\n  invariants [inv_b]\n}\n"
    );
}
