//! A selective `use` binding's imported name is a written token (ADR 0233): rename edits it, a delete with
//! `unlink_references` removes it, and an alias names nothing.

use crate::edit::{is_canonical, plan, project};
use crate::navigate::{SOFTWARE, at, compile};
use specforge_ops::authoring::edit::Edit;
use specforge_ops::authoring::rename;
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

#[ignore = "flipped by T5"]
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

#[ignore = "flipped by T6"]
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
