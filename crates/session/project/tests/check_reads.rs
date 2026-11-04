//! What a session's check inputs are: the reads its checks made (behavior
//! `classify_project_changes`, ADR 0107).

use std::fs;
use std::sync::Arc;

use serde_json::json;
use specforge_project::testing::TestProject;
use specforge_project::{ProjectSession, SharedRuntime, UpdateKind, Watched};
use specforge_test::prelude::*;

use crate::check_passes::{passes_extension, project as passes_project};

/// `passes_project(spec)` with each `gadget.docs` item read by `@specforge/checks:file` from the project root.
pub(crate) fn docs_project(spec: &str) -> TestProject {
    passes_project(spec)
        .config(|c| {
            c["checks"] = json!({ "fields": { "gadget.docs": ["@specforge/checks:file"] } })
        })
        .enabling(&["@test/passes", "@specforge/checks"])
}

/// `passes_extension()`, with the builtins' component beside it for `@specforge/checks`.
pub(crate) fn docs_runtime() -> SharedRuntime {
    Arc::new(passes_extension().beside(specforge_project::testing::component_runtime()))
}

fn e016(session: &ProjectSession) -> usize {
    session
        .project()
        .diagnostics()
        .iter()
        .filter(|d| d.code == "E016")
        .count()
}

/// A path a files check reads that is a symlink is read through the link: its
/// target, created or deleted, must re-run the checks. The target lies in the
/// project: a link out of it is E094 (ADR 0335).
#[cfg(unix)]
#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a check input that is a symlink is followed to its target"
)]
fn a_symlinked_check_inputs_target_reruns_the_checks() {
    let dir = docs_project("gadget doc \"Doc\" {\n  docs [\"link.md\"]\n}\n");
    let root = fs::canonicalize(dir.root()).unwrap();
    fs::create_dir_all(root.join("real")).unwrap();
    let target = root.join("real/target.md");
    std::os::unix::fs::symlink(&target, root.join("link.md")).unwrap();
    let mut session = dir.session_in(docs_runtime());
    assert_eq!(e016(&session), 1, "{:?}", session.project().diagnostics());

    fs::write(&target, "# target\n").unwrap();
    let update = session
        .apply(&session.changes([target.as_path()]))
        .expect("the target of a symlinked check input appeared");
    assert_eq!(update.kind, UpdateKind::Checks);
    assert_eq!(e016(&session), 0, "{:?}", session.project().diagnostics());

    fs::remove_file(&target).unwrap();
    let update = session
        .apply(&session.changes([target.as_path()]))
        .expect("the target of a symlinked check input went");
    assert_eq!(update.kind, UpdateKind::Checks);
    assert_eq!(e016(&session), 1, "{:?}", session.project().diagnostics());
}

/// A path a files check reads that is a symlink and is pointed elsewhere is
/// its own change: the link is keyed by its name, not by its old target.
#[cfg(unix)]
#[specforge_test(
    behavior = "classify_project_changes",
    verify = "an input that is a symlink pointed elsewhere is its change"
)]
fn a_check_input_link_pointed_elsewhere_reruns_the_checks() {
    let dir = docs_project("gadget doc \"Doc\" {\n  docs [\"link.md\"]\n}\n");
    let root = fs::canonicalize(dir.root()).unwrap();
    fs::create_dir_all(root.join("real")).unwrap();
    fs::write(root.join("real/target.md"), "# target\n").unwrap();
    let link = root.join("link.md");
    std::os::unix::fs::symlink(root.join("real/target.md"), &link).unwrap();
    let mut session = dir.session_in(docs_runtime());
    assert_eq!(e016(&session), 0, "{:?}", session.project().diagnostics());

    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(root.join("real/missing.md"), &link).unwrap();
    let update = session
        .apply(&session.changes([link.as_path()]))
        .expect("the link was pointed elsewhere");
    assert_eq!(update.kind, UpdateKind::Checks);
    assert_eq!(e016(&session), 1, "{:?}", session.project().diagnostics());
}

/// A files check's missing file lists its directory for the close-sibling help, so a file created beside it
/// re-runs the checks and changes that help.
#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a file beside a missing file a files check names changes its suggestion"
)]
fn a_file_beside_a_missing_checked_file_changes_its_suggestion() {
    let dir = docs_project("gadget doc \"Doc\" {\n  docs [\"guide.md\"]\n}\n");
    let root = dir.root();
    let mut session = dir.session_in(docs_runtime());
    let suggestion = |session: &ProjectSession| {
        session
            .project()
            .diagnostics()
            .iter()
            .find(|d| d.code == "E016")
            .unwrap_or_else(|| panic!("{:?}", session.project().diagnostics()))
            .suggestion
            .clone()
    };
    assert_eq!(suggestion(&session), None);

    let near = root.join("guid.md");
    fs::write(&near, "notes\n").unwrap();
    let changes = session.changes([near.as_path()]);
    assert!(!changes.is_empty(), "{changes:?}");
    session
        .apply(&changes)
        .expect("a file beside the missing one");
    assert_eq!(
        suggestion(&session).as_deref(),
        Some("did you mean 'guid.md'?")
    );
}

/// The inputs are the reads: each path as written, and the directories `Contained` listed on its way
/// (ADR 0335), the missing one among them.
#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a session's check inputs are the paths its checks read"
)]
fn the_check_inputs_are_the_paths_the_checks_asked() {
    let dir = docs_project("gadget doc \"Doc\" {\n  docs [\"a.md\", \"missing/b.md\"]\n}\n");
    let root = fs::canonicalize(dir.root()).unwrap();
    fs::write(root.join("a.md"), "# a\n").unwrap();
    let ext = docs_runtime();
    dir.record_declarations(&*ext);
    let session = ProjectSession::open_with_runtime(
        &specforge_common::ProjectRoot::of(&root).unwrap(),
        Some(ext),
    );
    let watched = session.inputs().watched();
    for expected in [
        Watched::File(root.join("a.md")),
        Watched::File(root.join("missing/b.md")),
        Watched::Listing(root.clone()),
        Watched::Listing(root.join("missing")),
    ] {
        assert!(watched.contains(&expected), "{expected:?} in {watched:?}");
    }
}
