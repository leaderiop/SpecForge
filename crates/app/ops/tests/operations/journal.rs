//! The write journal (ADR 0289): a write records what it is about to replace under `.specforge/write.lock`, and the
//! next writer puts back what an interrupted one left half done.

use serde_json::json;
use specforge_ops::authoring::edit::{self, Edit};
use specforge_ops::view::ProjectView;
use specforge_package::Sha256;
use specforge_project::testing::GsdWrites;
use specforge_test_macros::test as specforge_test;
use std::sync::Arc;

use crate::plans::{ONE, TWO, compiled, project};

fn digest(text: &str) -> String {
    format!("sha256:{}", Sha256::of(text.as_bytes()))
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "an interrupted write is rolled back by the next writer"
)]
fn an_interrupted_write_is_rolled_back_by_the_next() {
    let p = project();
    // A write was killed after its first file: `03-01.spec` holds the text the write gave it, `03-02.spec` was not
    // reached.
    let written = ONE.replace("}\n", "  result pass\n}\n");
    p.write("03-01.spec", &written);
    p.write(
        ".specforge/write-journal.json",
        json!({"format": 1, "doing": "cannot apply the edits", "files": [
            {"path": "03-01.spec", "before": ONE, "after": digest(&written)},
            {"path": "03-02.spec", "before": TWO, "after": digest("never written\n")},
        ]})
        .to_string(),
    );
    let runtime = Arc::new(GsdWrites::new().runtime());
    let compile = compiled(&p, &runtime);
    // The next write changes a file the journal does not name.
    edit::run(
        &ProjectView::of(&compile),
        &[Edit::Retitle {
            entity_id: "v1.0".into(),
            title: "Uno".into(),
        }],
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    assert_eq!(p.read("03-01.spec"), ONE, "the file at `after` is put back");
    assert_eq!(
        p.read("03-02.spec"),
        TWO,
        "the file at `before` is left alone"
    );
    assert!(p.read("m.spec").contains("\"Uno\""));
    assert!(!p.root().join(".specforge/write-journal.json").exists());
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "a typed edit of a project with a writing command and no harness field takes the write lock"
)]
fn a_typed_edit_in_a_project_with_a_writing_command_and_no_ledger_is_locked() {
    use specforge_project::testing::{OTHER, TestProject};
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[OTHER])
        .installing_enabled()
        .file("m.spec", "memo M \"M\" {}\n");
    let compile = compiled(&p, &Arc::new(GsdWrites::new().runtime()));
    edit::run(
        &ProjectView::of(&compile),
        &[Edit::Retitle {
            entity_id: "M".into(),
            title: "Uno".into(),
        }],
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    assert!(p.read("m.spec").contains("\"Uno\""));
    assert!(
        p.root().join(".specforge/write.lock").exists(),
        "the write took the lock"
    );
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "a journal marked done is removed by the next writer and rolls nothing back"
)]
fn a_journal_marked_done_is_removed_and_rolls_nothing_back() {
    let p = project();
    // The write committed both files and died before it removed its record.
    let written = ONE.replace("}\n", "  result pass\n}\n");
    p.write("03-01.spec", &written);
    p.write(
        ".specforge/write-journal.json",
        json!({"format": 1, "doing": "cannot apply the edits", "done": true, "files": [
            {"path": "03-01.spec", "before": ONE, "after": digest(&written)},
        ]})
        .to_string(),
    );
    let runtime = Arc::new(GsdWrites::new().runtime());
    let compile = compiled(&p, &runtime);
    edit::run(
        &ProjectView::of(&compile),
        &[Edit::Retitle {
            entity_id: "v1.0".into(),
            title: "Uno".into(),
        }],
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    assert_eq!(p.read("03-01.spec"), written, "a completed write stays");
    assert!(!p.root().join(".specforge/write-journal.json").exists());
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "a journal naming a path outside the project root or a malformed digest is refused and restores nothing"
)]
fn a_journal_naming_a_path_outside_the_project_is_refused() {
    let p = project();
    let outside = tempfile::tempdir().unwrap();
    let victim = outside.path().join("victim.spec");
    std::fs::write(&victim, "secret\n").unwrap();
    let runtime = Arc::new(GsdWrites::new().runtime());
    let retitle = |p: &specforge_project::testing::TestProject| {
        let compile = compiled(p, &runtime);
        edit::run(
            &ProjectView::of(&compile),
            &[Edit::Retitle {
                entity_id: "v1.0".into(),
                title: "Uno".into(),
            }],
            false,
            &specforge_ops::authoring::apply::testing::origin(),
            specforge_ops::testing::caller(),
        )
    };
    let absolute = victim.to_string_lossy().to_string();
    let relative = format!(
        "../{}/victim.spec",
        outside.path().file_name().unwrap().to_string_lossy()
    );
    for (path, after) in [
        (absolute, digest("secret\n")),
        (relative, digest("secret\n")),
        ("03-01.spec".to_string(), "sha256:short".to_string()),
    ] {
        p.write(
            ".specforge/write-journal.json",
            json!({"format": 1, "doing": "cannot apply the edits", "files": [
                {"path": path, "before": "overwritten\n", "after": after},
            ]})
            .to_string(),
        );
        assert!(retitle(&p).is_err(), "{path} is refused");
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "secret\n");
        assert_eq!(p.read("03-01.spec"), ONE);
    }
}
