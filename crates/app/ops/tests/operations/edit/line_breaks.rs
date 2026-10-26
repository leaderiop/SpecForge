//! A typed edit reads and writes a file in its own line break (ADR 0266).

use super::*;

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

#[specforge_test_macros::test(
    behavior = "edit_entity_field",
    verify = "a typed edit of a CRLF block holding a multi-line quoted text is planned, its text kept"
)]
fn a_typed_edit_of_a_crlf_block_holding_a_multi_line_text_is_planned() {
    let text = crlf("behavior b \"B\" {\n  category query\n  contract \"a\nb\"\n}\n");
    let c = project(&[("tasks.spec", &text)]);
    for edit in [
        set("b", "category", one("command")),
        Edit::Retitle {
            entity_id: "b".into(),
            title: "Bee".into(),
        },
        Edit::AddVerify {
            entity_id: "b".into(),
            kind: Some("unit".into()),
            text: "it works".into(),
        },
    ] {
        let planned = plan(&c, std::slice::from_ref(&edit)).unwrap_or_else(|r| {
            panic!("{edit:?} refused: {}", r.error.message);
        });
        let written = planned.change().after().remove("tasks.spec").unwrap();
        assert!(written.contains("\"a\nb\""), "{written:?}");
    }
}

#[specforge_test_macros::test(
    invariant = "typed_edit_canonical_form",
    verify = "a typed edit writes a CRLF file in CRLF, as it writes the same file in LF"
)]
#[ignore = "flipped by T6"]
fn every_typed_edit_writes_a_crlf_file_as_it_writes_its_lf_reading() {
    let lf_project = project(&[("tasks.spec", TASKS)]);
    let crlf_project = project(&[("tasks.spec", &crlf(TASKS))]);
    let edits = [
        set("list_tasks", "category", one("command")),
        unset("complete_task", "invariants"),
        link("complete_task", "invariants", "extra_rule"),
        unlink("complete_task", "invariants", "done_stamped"),
        Edit::AddVerify {
            entity_id: "complete_task".into(),
            kind: Some("unit".into()),
            text: "a new line".into(),
        },
        Edit::RemoveVerify {
            entity_id: "complete_task".into(),
            kind: Some("unit".into()),
            text: "completing sets the status".into(),
        },
        Edit::Retitle {
            entity_id: "list_tasks".into(),
            title: "Listing".into(),
        },
        create(
            "behavior",
            "archive_task",
            "tasks.spec",
            &[
                ("contract", one("When archived, the system MUST mark it.")),
                ("types", many(&["Task"])),
                ("category", one("command")),
            ],
        ),
        Edit::Delete {
            entity_id: "done_stamped".into(),
            unlink_references: true,
        },
    ];
    for edit in edits {
        let written = after(&crlf_project, "tasks.spec", std::slice::from_ref(&edit));
        assert!(
            !written.replace("\r\n", "").contains('\n'),
            "{edit:?} wrote a bare LF: {written:?}"
        );
        assert_eq!(
            written.replace("\r\n", "\n"),
            after(&lf_project, "tasks.spec", std::slice::from_ref(&edit)),
            "{edit:?}"
        );
    }
}

#[specforge_test_macros::test(
    invariant = "typed_edit_canonical_form",
    verify = "a typed edit of a file mixing CRLF and LF changes no byte outside the blocks it touches"
)]
fn a_typed_edit_of_a_mixed_file_changes_no_line_outside_its_block() {
    let (head, tail) = TASKS.split_once("behavior complete_task").unwrap();
    let mixed = format!("{}behavior complete_task{tail}", crlf(head));
    let c = project(&[("tasks.spec", &mixed)]);
    let written = after(
        &c,
        "tasks.spec",
        &[set("list_tasks", "category", one("command"))],
    );
    let start = mixed.find("behavior list_tasks").unwrap();
    assert_eq!(written[..start], mixed[..start]);
}
