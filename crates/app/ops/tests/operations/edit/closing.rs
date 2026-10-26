//! A comment run directly above a body's closing brace is the body's: a new last member goes above it, and no
//! member's removal takes it (ADR 0274).

use super::*;
use crate::navigate::compile;

/// A behavior whose body ends with a comment run, and no verify statement.
const CLOSING: &str = "behavior keep \"Keep\" {\n  category command\n  contract \"c\"\n  // TODO: ask legal about retention\n}\n";
/// A block that writes no field and no statement, only a comment.
const BARE: &str = "event ev_one \"E1\" {\n  // TODO: pick a category\n}\n";
/// A block field whose body ends with a comment run (`requires` is `@specforge/formal`'s).
const NESTED: &str = "behavior check_graph \"Check the Graph\" {\n  category command\n  requires {\n    graph_ready \"Graph built\"\n    // TODO: a disk precondition?\n  }\n  contract \"c\"\n}\n";

fn nested() -> Compiled {
    compile(
        &["@specforge/software", "@specforge/formal"],
        &[("g.spec", NESTED)],
    )
}

fn add_verify() -> Edit {
    Edit::AddVerify {
        entity_id: "keep".into(),
        kind: Some("unit".into()),
        text: "retention is 30 days".into(),
    }
}

fn remove_verify() -> Edit {
    Edit::RemoveVerify {
        entity_id: "keep".into(),
        kind: Some("unit".into()),
        text: "retention is 30 days".into(),
    }
}

#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "an added verify line goes above the comment lines before the closing brace, and its removal leaves them"
)]
#[ignore = "flipped by T2"]
fn an_added_verify_line_and_its_removal_leave_the_closing_comment() {
    let c = project(&[("k.spec", CLOSING)]);
    let planned = plan(&c, &[add_verify(), remove_verify()]).unwrap();
    assert!(
        planned.change().is_empty(),
        "{:?}",
        planned.change().after()
    );
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "a new last field or entry goes above the comment lines before its closing brace, and its unset leaves them"
)]
#[ignore = "flipped by T2"]
fn a_first_field_and_its_unset_leave_the_closing_comment() {
    let c = project(&[("ev.spec", BARE)]);
    let planned = plan(
        &c,
        &[
            set("ev_one", "category", one("domain")),
            unset("ev_one", "category"),
        ],
    )
    .unwrap();
    assert!(
        planned.change().is_empty(),
        "{:?}",
        planned.change().after()
    );
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "a new last field or entry goes above the comment lines before its closing brace, and its unset leaves them"
)]
#[ignore = "flipped by T2"]
fn a_new_entry_and_its_unset_leave_the_block_fields_closing_comment() {
    let c = nested();
    let planned = plan(
        &c,
        &[
            set("check_graph", "requires.disk_ready", one("Disk readable")),
            unset("check_graph", "requires.disk_ready"),
        ],
    )
    .unwrap();
    assert!(
        planned.change().is_empty(),
        "{:?}",
        planned.change().after()
    );
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "a new last field or entry goes above the comment lines before its closing brace, and its unset leaves them"
)]
#[ignore = "flipped by T2"]
fn a_new_last_member_goes_above_the_closing_comment() {
    let c = project(&[("k.spec", CLOSING)]);
    let text = after(&c, "k.spec", &[add_verify()]);
    assert_eq!(
        text,
        "behavior keep \"Keep\" {\n  category command\n  contract \"c\"\n  verify unit \"retention is 30 days\"\n  // TODO: ask legal about retention\n}\n"
    );
    assert!(is_canonical(&c, &text));

    let c = project(&[("ev.spec", BARE)]);
    let text = after(&c, "ev.spec", &[set("ev_one", "category", one("domain"))]);
    assert_eq!(
        text,
        "event ev_one \"E1\" {\n  category domain\n  // TODO: pick a category\n}\n"
    );
    assert!(is_canonical(&c, &text));

    let c = nested();
    let text = after(
        &c,
        "g.spec",
        &[set(
            "check_graph",
            "requires.disk_ready",
            one("Disk readable"),
        )],
    );
    assert_eq!(
        text,
        "behavior check_graph \"Check the Graph\" {\n  category command\n  requires {\n    graph_ready \"Graph built\"\n    disk_ready  \"Disk readable\"\n    // TODO: a disk precondition?\n  }\n  contract \"c\"\n}\n"
    );
    assert!(is_canonical(&c, &text));
}
