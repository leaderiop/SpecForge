use super::*;

fn tasks() -> Compiled {
    project(&[("tasks.spec", TASKS)])
}

fn add(id: &str, kind: Option<&str>, text: &str) -> Edit {
    Edit::AddVerify {
        entity_id: id.into(),
        kind: kind.map(String::from),
        text: text.into(),
    }
}

fn remove(id: &str, kind: Option<&str>, text: &str) -> Edit {
    Edit::RemoveVerify {
        entity_id: id.into(),
        kind: kind.map(String::from),
        text: text.into(),
    }
}

#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "a verify line is added after the last one"
)]
fn a_verify_line_is_added_after_the_last() {
    let c = tasks();
    let text = after(
        &c,
        "tasks.spec",
        &[add("complete_task", Some("unit"), "a new check")],
    );
    assert_eq!(
        text,
        TASKS.replace(
            "  verify integration \"an event fires\"\n",
            "  verify integration \"an event fires\"\n  verify unit \"a new check\"\n"
        )
    );
}

#[test]
fn a_verify_line_is_added_to_a_block_with_none() {
    let c = tasks();
    let text = after(
        &c,
        "tasks.spec",
        &[add("list_tasks", Some("unit"), "lists")],
    );
    assert!(
        text.contains("  \"\"\"\n  verify unit \"lists\"\n}"),
        "{text}"
    );
}

#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "a verify line is removed by kind and text"
)]
fn a_verify_line_is_removed_by_kind_and_text() {
    let c = tasks();
    let text = after(
        &c,
        "tasks.spec",
        &[remove(
            "complete_task",
            Some("unit"),
            "completing sets the status",
        )],
    );
    assert_eq!(
        text,
        TASKS.replace("  verify unit \"completing sets the status\"\n", "")
    );
    assert!(
        plan(
            &c,
            &[remove(
                "complete_task",
                Some("integration"),
                "completing sets the status"
            )]
        )
        .unwrap()
        .change()
        .is_empty()
    );
}

#[test]
fn an_equal_verify_line_is_not_added_twice() {
    let c = tasks();
    assert!(
        plan(
            &c,
            &[add(
                "complete_task",
                Some("unit"),
                "completing sets the status"
            )]
        )
        .unwrap()
        .change()
        .is_empty()
    );
}

fn with(kind: Option<&str>, text: &str) -> Obligation {
    Obligation {
        kind: kind.map(String::from),
        text: text.into(),
    }
}

#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "a prefix is an obligation's kind only when it is one of the verify kinds its entity's kind accepts"
)]
fn a_prefix_is_a_kind_only_when_it_is_one_of_the_verify_kinds() {
    let kinds = ["unit".to_string(), "property".to_string()];
    let parsed = |text| Obligation::parse(text, &kinds);
    assert_eq!(parsed("unit: rejects x"), with(Some("unit"), "rejects x"));
    assert_eq!(parsed("rejects: a b"), with(None, "rejects: a b"));
    assert_eq!(parsed("http://x.y works"), with(None, "http://x.y works"));
    assert_eq!(parsed("Unit: x"), with(None, "Unit: x"));
    assert_eq!(parsed("two words: x"), with(None, "two words: x"));
    assert_eq!(parsed("plain"), with(None, "plain"));
    assert_eq!(Obligation::parse("unit: x", &[]), with(None, "unit: x"));
}

#[specforge_test_macros::test(
    behavior = "edit_entity_obligation",
    verify = "an obligation the entity writes in either reading of a prefix is the one its text names"
)]
fn an_obligation_written_in_either_reading_is_the_one_named() {
    let c = compile(
        &["@specforge/software", "@specforge/testing"],
        &[(
            "a.spec",
            "type user \"User\" {\n  verify \"unit: whole\"\n  verify Given \"kinded\"\n}\n",
        )],
    );
    let view = ProjectView::of(&c.project);
    let on = |id, text| Obligation::on(&view, id, text);
    assert_eq!(on("user", "unit: whole"), with(None, "unit: whole"));
    assert_eq!(on("user", "Given: kinded"), with(Some("Given"), "kinded"));
    assert_eq!(on("user", "unit: new"), with(Some("unit"), "new"));
    assert_eq!(on("user", "Given: new"), with(None, "Given: new"));
    assert_eq!(on("nobody", "unit: x"), with(None, "unit: x"));
}
