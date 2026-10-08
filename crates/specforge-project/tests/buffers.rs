//! A project session and the editor buffers it is given (behavior
//! `hold_editor_buffers`, ADR 0046).

use std::fs;
use std::sync::Arc;

use specforge_extension_sdk::prelude::*;
use specforge_project::{ProjectSession, SharedRuntime, SourceChange};
use specforge_wasm::testing::InProcessRuntime;
use tempfile::TempDir;

const EXT: &str = "@test/count";
const ALPHA: &str = "type alpha \"A\" {}\n";
const OMEGA: &str = "type omega \"O\" {}\n";
const ZETA: &str = "type zeta \"Z\" {}\n";

/// An extension whose one check pass reports I990: each run of the checks is
/// one call, and the session reports I990 exactly when the checks ran last.
fn counting() -> Arc<InProcessRuntime> {
    Arc::new(InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.pass("count", |p| {
            p.phase("check").run(|_: &PassInput| {
                vec![PassDiagnostic::new(
                    "I990",
                    PassSeverity::Info,
                    "the checks ran",
                )]
            });
        });
        c
    }))
}

fn check_runs(runtime: &InProcessRuntime) -> usize {
    runtime
        .calls()
        .iter()
        .filter(|c| c.export == "__pass_count")
        .count()
}

/// A project holding `a.spec` (alpha) that enables the counting extension,
/// opened over `runtime`.
fn opened(runtime: &Arc<InProcessRuntime>) -> (TempDir, ProjectSession) {
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({"name": "p", "version": "0.1.0", "extensions": [EXT]});
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    specforge_installed::testing::install(dir.path(), &[EXT]);
    fs::write(dir.path().join("a.spec"), ALPHA).unwrap();
    let session =
        ProjectSession::open_with_runtime(dir.path(), Some(Arc::clone(runtime) as SharedRuntime));
    (dir, session)
}

fn has(session: &ProjectSession, id: &str) -> bool {
    session.project().graph().node(id).is_some()
}

fn hold(session: &mut ProjectSession, text: &str) {
    session.update(SourceChange::Buffer {
        path: "a.spec",
        text: Some(text),
    });
}

/// Pin (flipped by T2): a buffer's file written on disk replaces the buffer
/// when the session catches up.
#[test]
fn pin_a_buffers_file_written_on_disk_replaces_it() {
    let rt = counting();
    let (dir, mut session) = opened(&rt);
    hold(&mut session, OMEGA);
    fs::write(dir.path().join("a.spec"), ZETA).unwrap();
    assert_eq!(session.stale().sources, ["a.spec"]);
    assert!(session.ensure_fresh().is_some());
    assert!(has(&session, "zeta") && !has(&session, "omega"));
}

/// Pin (flipped by T2): a buffer's deleted file leaves the project.
#[test]
fn pin_a_buffers_deleted_file_leaves_the_project() {
    let rt = counting();
    let (dir, mut session) = opened(&rt);
    hold(&mut session, OMEGA);
    fs::remove_file(dir.path().join("a.spec")).unwrap();
    assert_eq!(session.stale().sources, ["a.spec"]);
    assert!(session.ensure_fresh().is_some());
    assert!(!has(&session, "omega"));
}

/// Pin (flipped by T2): a buffer whose text is its file's still runs the
/// checks.
#[test]
fn pin_a_buffer_whose_text_is_its_file_runs_the_checks() {
    let rt = counting();
    let (_dir, mut session) = opened(&rt);
    let before = check_runs(&rt);
    hold(&mut session, ALPHA);
    assert_eq!(check_runs(&rt) - before, 1);
}

/// Pin (flipped by T3): a reload rebuilds from disk and forgets the buffers.
#[test]
fn pin_a_reload_rebuilds_from_disk_without_the_buffers() {
    let rt = counting();
    let (_dir, mut session) = opened(&rt);
    hold(&mut session, OMEGA);
    let before = check_runs(&rt);
    session.reload_environment();
    assert_eq!(check_runs(&rt) - before, 1);
    assert!(has(&session, "alpha") && !has(&session, "omega"));
}
