//! A write left half done (ADR 0289): its journal is W167 until the next writer rolls it back, and a check never
//! rolls it back itself.

use std::sync::Arc;

use serde_json::json;
use specforge_project::testing::{GSD_WRITES_EXTENSIONS, GsdWrites, TestProject};
use specforge_test::prelude::*;

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "an interrupted write is W167 until the next writer rolls it back"
)]
fn an_interrupted_write_is_w167_until_rolled_back() {
    let spec = "milestone v1.0 \"One\" {}\n";
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("a.spec", spec);
    let journal = ".specforge/write-journal.json";
    project.write(
        journal,
        json!({"format": 1, "doing": "cannot rename 'a'", "files": [
            {"path": "a.spec", "before": "x\n", "after": "sha256:00"},
        ]})
        .to_string(),
    );
    let compiled = project.compile_in(Arc::new(GsdWrites::new().runtime()));
    let pending: Vec<_> = compiled
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == "W167")
        .collect();
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert!(
        pending[0].message.contains("cannot rename 'a'") && pending[0].message.contains("a.spec")
    );
    // A check never writes: the journal and the file are as they were.
    assert!(project.root().join(journal).exists());
    assert_eq!(project.read("a.spec"), spec);
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "a session reports a journal that appears and drops one that goes"
)]
fn a_session_reports_a_journal_that_appears_and_drops_one_that_goes() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("a.spec", "milestone v1.0 \"One\" {}\n");
    let mut session = project.session_in(Arc::new(GsdWrites::new().runtime()));
    let w167 = |session: &specforge_project::ProjectSession| {
        session
            .project()
            .diagnostics()
            .iter()
            .filter(|d| d.code == "W167")
            .count()
    };
    assert_eq!(w167(&session), 0);
    let journal = ".specforge/write-journal.json";
    project.write(
        journal,
        json!({"format": 1, "doing": "cannot rename 'a'", "files": [
            {"path": "a.spec", "before": "x\n", "after": "sha256:00"},
        ]})
        .to_string(),
    );
    let path = project.root().join(journal);
    let changes = session.changes([path.as_path()]);
    session.apply(&changes);
    assert_eq!(w167(&session), 1, "the journal appeared");
    std::fs::remove_file(&path).unwrap();
    let changes = session.changes([path.as_path()]);
    session.apply(&changes);
    assert_eq!(w167(&session), 0, "the journal went");
}
