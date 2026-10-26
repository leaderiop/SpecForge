//! A comment inside a member's value, or trailing it, is where the parse read it: a typed edit keeps it, or refuses
//! the rewrite naming its entity (ADR 0267).

use super::*;

/// A list holding a comment inside it, a list a comment trails, a block field holding both kinds.
const COMMENTED: &str = "invariant done_stamped \"Done Stamped\" {\n  guarantee \"g\"\n}\n\ninvariant extra_rule \"Extra Rule\" {\n  guarantee \"e\"\n}\n\ninvariant task_unique \"Task Unique\" {\n  guarantee \"u\"\n}\n\nbehavior complete_task \"Complete a Task\" {\n  category   command\n  invariants [\n    done_stamped, // stamps\n    extra_rule\n  ]\n}\n\nbehavior list_tasks \"List Tasks\" {\n  category   query\n  invariants [done_stamped] // core\n}\n\nport auth_port \"Auth port\" {\n  direction inbound\n  methods {\n    // the entry point\n    login  \"sign in\" // primary\n    logout \"sign out\"\n  }\n}\n";

fn tasks() -> Compiled {
    project(&[("tasks.spec", COMMENTED)])
}

fn entries(pairs: &[(&str, &str)]) -> Input {
    Input::Entries(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "set of a value holding a comment to what it holds writes nothing"
)]
fn set_to_what_a_value_holding_a_comment_holds_writes_nothing() {
    let c = tasks();
    let same = [
        set(
            "complete_task",
            "invariants",
            many(&["done_stamped", "extra_rule"]),
        ),
        set(
            "auth_port",
            "methods",
            entries(&[("login", "sign in"), ("logout", "sign out")]),
        ),
    ];
    for edit in same {
        assert!(plan(&c, &[edit]).unwrap().change().is_empty());
    }
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "set of a value holding a comment to another value is refused not_editable, naming its entity and file"
)]
fn set_of_a_value_holding_a_comment_to_another_is_refused_naming_its_entity_and_file() {
    let c = tasks();
    let error = plan(
        &c,
        &[set("complete_task", "invariants", many(&["done_stamped"]))],
    )
    .unwrap_err()
    .error;
    assert_eq!(error.code, "not_editable");
    assert_eq!(error.entity(), Some("complete_task"));
    assert!(error.message.contains("complete_task"), "{}", error.message);
    assert!(error.message.contains("tasks.spec"), "{}", error.message);
}

#[specforge_test_macros::test(
    behavior = "edit_entity_link",
    verify = "link and unlink keep the comment trailing the list"
)]
#[ignore = "flipped by T5"]
fn link_and_unlink_keep_the_comment_trailing_the_list() {
    let c = tasks();
    let linked = after(
        &c,
        "tasks.spec",
        &[link("list_tasks", "invariants", "extra_rule")],
    );
    assert_eq!(
        linked,
        COMMENTED.replace(
            "invariants [done_stamped] // core",
            "invariants [done_stamped, extra_rule] // core"
        )
    );
    assert!(is_canonical(&c, &linked));
    let back = after(
        &c,
        "tasks.spec",
        &[
            link("list_tasks", "invariants", "extra_rule"),
            unlink("list_tasks", "invariants", "extra_rule"),
        ],
    );
    assert_eq!(back, COMMENTED);
}

#[specforge_test_macros::test(
    behavior = "edit_entity_link",
    verify = "a link to a target already there writes nothing, whatever comment the list holds"
)]
#[ignore = "flipped by T5"]
fn a_link_to_a_target_already_there_writes_nothing_whatever_comment_the_list_holds() {
    let c = tasks();
    for id in ["complete_task", "list_tasks"] {
        assert!(
            plan(&c, &[link(id, "invariants", "done_stamped")])
                .unwrap()
                .change()
                .is_empty(),
            "{id}"
        );
    }
}

#[specforge_test_macros::test(
    behavior = "delete_entity",
    verify = "delete with unlink_references through a list holding a comment is refused naming the referrer and its file"
)]
#[ignore = "flipped by T5"]
fn delete_through_a_list_holding_a_comment_is_refused_naming_the_referrer_and_its_file() {
    let c = tasks();
    let edit = Edit::Delete {
        entity_id: "extra_rule".into(),
        unlink_references: true,
    };
    let error = plan(&c, &[edit]).unwrap_err().error;
    assert_eq!(error.code, "not_editable");
    assert_eq!(error.entity(), Some("complete_task"));
    assert!(error.message.contains("complete_task"), "{}", error.message);
    assert!(error.message.contains("tasks.spec"), "{}", error.message);
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "set keeps the comment trailing the value, annotations given or not"
)]
fn set_with_annotations_keeps_the_comment_trailing_the_value() {
    let c = tasks();
    let annotated = Edit::Set {
        entity_id: "list_tasks".into(),
        field: "invariants".into(),
        value: many(&["done_stamped", "extra_rule"]),
        annotations: Some(vec!["note=why".into()]),
    };
    let text = after(&c, "tasks.spec", &[annotated]);
    assert_eq!(
        text,
        COMMENTED.replace(
            "invariants [done_stamped] // core",
            "invariants [done_stamped, extra_rule] @note \"why\" // core"
        )
    );
    assert!(is_canonical(&c, &text));
    let entry = after(
        &c,
        "tasks.spec",
        &[set("auth_port", "methods.logout", one("bye"))],
    );
    assert_eq!(entry, COMMENTED.replace("\"sign out\"", "\"bye\""));
}
