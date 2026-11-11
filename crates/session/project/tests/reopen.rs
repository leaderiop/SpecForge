//! A project opened in a session's place (ADR 0109).

use std::cell::Cell;
use std::sync::Arc;

use specforge_project::testing::{TestProject, disk_changed};
use specforge_project::{
    Buffer, COUNTED_DEADLINE, ProjectSession, RuntimeSource, SharedRuntime, UpdateKind,
};
use specforge_test::prelude::*;

use crate::buffers::{check_runs, counting, has, opened};

const ALPHA: &str = "type alpha \"A\" {}\n";
const OMEGA: &str = "type omega \"O\" {}\n";
const ZETA: &str = "type zeta \"Z\" {}\n";
const GAMMA: &str = "type gamma \"G\" {}\n";

/// A session given a parse deadline holds every environment it loads to it, reloads included; one given none
/// holds them to production's.
#[test]
fn a_session_parses_within_the_deadline_it_was_given_across_reloads() {
    let dir = TestProject::new().file("a.spec", ALPHA);
    let deadline = |session: &ProjectSession| session.project().environment().syntax().deadline();
    let mut session = ProjectSession::detached().with_parse_deadline(COUNTED_DEADLINE);
    session.reopen(&dir.project_root(), RuntimeSource::Fixed(None), |_| {});
    assert_eq!(deadline(&session), COUNTED_DEADLINE);
    session.reload_environment();
    assert_eq!(deadline(&session), COUNTED_DEADLINE, "a reload keeps it");

    assert_eq!(
        deadline(&dir.session_bare()),
        COUNTED_DEADLINE,
        "a test project's session"
    );
    let root = dir.project_root();
    let opened = ProjectSession::open_with_runtime(&root, None);
    assert_eq!(deadline(&opened), COUNTED_DEADLINE, "a test's open");
    let production = ProjectSession::open_from(&root, RuntimeSource::Fixed(None));
    assert_eq!(deadline(&production), specforge_source::PARSE_DEADLINE);
}

#[specforge_test(
    behavior = "validate_delta_correctness",
    verify = "an environment reload keeps whether the session verifies its updates"
)]
fn a_reload_keeps_whether_the_session_verifies() {
    let dir = TestProject::new().file("a.spec", ALPHA);
    let mut session = dir.session_bare();

    session.set_verify_incremental(false);
    session.reload_environment();
    dir.write("a.spec", "type alpha \"B\" {}\n");
    let update = disk_changed(&mut session, &["a.spec"]).unwrap();
    assert_eq!(update.verification, None, "verification stayed off");

    session.set_verify_incremental(true);
    session.reload_environment();
    dir.write("a.spec", "type alpha \"C\" {}\n");
    let update = disk_changed(&mut session, &["a.spec"]).unwrap();
    assert_eq!(update.verification, Some(Ok(())), "verification stayed on");
}

#[specforge_test(
    behavior = "hold_editor_buffers",
    verify = "a project opened in a session's place keeps its buffers and whether it verifies"
)]
fn a_project_opened_in_a_sessions_place_keeps_its_buffers_and_whether_it_verifies() {
    let a = TestProject::new().file("a.spec", ALPHA);
    let b = TestProject::new()
        .file("b.spec", ZETA)
        .file("c.spec", GAMMA);
    let mut session = a.session_bare();
    session.set_verify_incremental(false);
    // B's file, outside A: held, and building nothing in A.
    session.hold(&[Buffer::new(b.root().join("b.spec"), OMEGA).at_version(Some(3))]);
    assert!(!has(&session, "omega"));

    session.reopen(&b.project_root(), RuntimeSource::Fixed(None), |_| {});

    assert!(has(&session, "omega") && has(&session, "gamma"));
    assert!(!has(&session, "zeta") && !has(&session, "alpha"));
    assert_eq!(session.buffer("b.spec").unwrap().version, Some(3));
    b.write("c.spec", "type gamma \"H\" {}\n");
    let update = disk_changed(&mut session, &["c.spec"]).unwrap();
    assert_eq!(update.verification, None, "verification stayed off");
}

#[specforge_test_macros::test(
    behavior = "complete_keywords",
    verify = "the registered kinds are handed to keyword completion before any .spec file is read"
)]
fn a_reopen_hands_over_the_environment_before_it_reads_a_source() {
    let rt = counting();
    let (dir, mut session) = opened(&rt);
    let before = check_runs(&rt);
    let seen = Cell::new(None);
    let calls = Cell::new(0);

    session.reopen(
        &dir.project_root(),
        RuntimeSource::Fixed(Some(Arc::clone(&rt) as SharedRuntime)),
        |environment| {
            calls.set(calls.get() + 1);
            seen.set(Some((
                check_runs(&rt),
                environment.registries().declarations().len(),
            )));
        },
    );

    assert_eq!(calls.get(), 1);
    assert_eq!(seen.get(), Some((before, 1)));
    assert_eq!(check_runs(&rt), before + 1);
}

#[test]
fn a_reopen_is_the_update_of_the_replacement() {
    let rt = counting();
    let (dir, _) = opened(&rt);
    let mut session =
        ProjectSession::detached().with_parse_deadline(specforge_project::COUNTED_DEADLINE);

    let update = session.reopen(
        &dir.project_root(),
        RuntimeSource::Fixed(Some(Arc::clone(&rt) as SharedRuntime)),
        |_| {},
    );

    assert_eq!(update.kind, UpdateKind::Environment);
    assert!(update.inputs_changed);
    assert_eq!(update.rebuilt_files, ["a.spec"]);
    assert!(update.previous_diagnostics.is_empty());
    assert_eq!(update.verification, None);
    // The source given was used, not the detached session's.
    assert!(update.diagnostics.iter().any(|d| d.code.ends_with("/I990")));
}
