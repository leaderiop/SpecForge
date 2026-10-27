//! A project that declares no writer or history flag reads no git and no ledger (ADR 0287): what a session
//! watches is what it watched before.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use serde_json::json;
use specforge_project::Watched;
use specforge_project::history::{HistoryError, Located};
use specforge_project::testing::{FakeHistory, TestProject, gsd_history_runtime};

use crate::computed;
use specforge_test::prelude::*;

/// `git init` and one commit of everything under `root`.
pub(crate) fn git_repo(root: &Path) {
    for args in [
        &["init", "-q", "-b", "main"][..],
        &["add", "-A"],
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "c",
        ],
    ] {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a project that declares no history reads no git and watches no ref and no ledger"
)]
fn an_unflagged_project_records_no_git_ref_and_no_ledger_as_a_check_input() {
    let dir = TestProject::new().files(&[("a.spec", "term alpha_one \"A\" {}\n")]);
    git_repo(dir.root());
    let fake = Arc::new(FakeHistory::none());
    let _installed = specforge_project::testing::history::install(fake.clone());
    let session = dir.session_bare();
    for watched in session.inputs().watched() {
        let path = match &watched {
            Watched::Sources(p)
            | Watched::Directories(p)
            | Watched::File(p)
            | Watched::Listing(p) => p,
        };
        assert!(
            !path.components().any(|c| c.as_os_str() == ".git"),
            "{watched:?} is under .git"
        );
        assert!(!path.ends_with("specforge-harness"), "{watched:?}");
    }
    assert_eq!(fake.questions(), 0, "git was asked");
}

const DECISION: &str = "decision D1 \"D\" {\n  phase 03\n}\n";

fn compiled_with(
    fake: &Arc<FakeHistory>,
    project: &TestProject,
) -> Vec<specforge_common::Diagnostic> {
    let _installed = specforge_project::testing::history::install(fake.clone());
    project
        .compile_in(Arc::new(gsd_history_runtime(json!([]))))
        .diagnostics()
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "no history is I204 only when history is declared"
)]
fn no_history_is_i204_only_when_history_is_declared() {
    let flagged = computed::project(DECISION);
    let outside = Arc::new(FakeHistory::none());
    let found = compiled_with(&outside, &flagged);
    assert_eq!(computed::with_code(&found, "I204").len(), 1, "{found:?}");
    // an unborn HEAD
    let mut unborn = FakeHistory::repo(flagged.root(), "a".repeat(40).as_str(), Vec::new());
    unborn.head = Ok(None);
    let found = compiled_with(&Arc::new(unborn), &flagged);
    let i204 = computed::with_code(&found, "I204");
    assert_eq!(i204.len(), 1, "{found:?}");
    assert!(i204[0].message.contains("no commit yet"), "{i204:?}");
    // a commit: nothing to say
    let at_head = Arc::new(FakeHistory::repo(
        flagged.root(),
        "b".repeat(40).as_str(),
        Vec::new(),
    ));
    let found = compiled_with(&at_head, &flagged);
    assert!(computed::with_code(&found, "I204").is_empty(), "{found:?}");
    // nothing declares history: the port is never asked
    let unflagged = TestProject::new().files(&[("a.spec", "term alpha_one \"A\" {}\n")]);
    let silent = Arc::new(FakeHistory::none());
    let _installed = specforge_project::testing::history::install(silent.clone());
    let found = unflagged.compile_bare().diagnostics();
    assert!(computed::with_code(&found, "I204").is_empty(), "{found:?}");
    assert_eq!(silent.questions(), 0);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "an unreadable baseline is W164 and nothing is judged"
)]
fn an_unreadable_baseline_is_w164() {
    let project = computed::project(DECISION);
    let mut missing = FakeHistory::none();
    missing.located = Located::Unavailable("git is not installed".into());
    let found = compiled_with(&Arc::new(missing), &project);
    let w164 = computed::with_code(&found, "W164");
    assert_eq!(w164.len(), 1, "{found:?}");
    assert!(w164[0].message.contains("git is not installed"), "{w164:?}");
    let mut broken = FakeHistory::repo(project.root(), "c".repeat(40).as_str(), Vec::new());
    broken.files = Err(HistoryError("the objects are not here (shallow)".into()));
    let found = compiled_with(&Arc::new(broken), &project);
    let w164 = computed::with_code(&found, "W164");
    assert_eq!(w164.len(), 1, "{found:?}");
    assert!(w164[0].message.contains("shallow"), "{w164:?}");
    assert!(computed::with_code(&found, "I204").is_empty());
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "the baseline is cached by commit and environment"
)]
fn the_baseline_is_cached_by_commit_and_environment() {
    let project = computed::project(DECISION);
    let fake = Arc::new(FakeHistory::repo(
        project.root(),
        "d".repeat(40).as_str(),
        Vec::new(),
    ));
    compiled_with(&fake, &project);
    compiled_with(&fake, &project);
    assert_eq!(
        fake.files_asked(),
        1,
        "one commit, one environment: one compile"
    );
    // another commit
    let other = Arc::new(FakeHistory::repo(
        project.root(),
        "e".repeat(40).as_str(),
        Vec::new(),
    ));
    compiled_with(&other, &project);
    assert_eq!(other.files_asked(), 1);
    // another environment: the excludes are part of it
    let project = project.config(|c| c["exclude"] = json!(["nothing/**"]));
    compiled_with(&fake, &project);
    assert_eq!(
        fake.files_asked(),
        2,
        "a different environment compiles again"
    );
    // a module rebuilt under the same name and version is another environment: its digest is part of it
    let mut rebuilt = specforge_wasm::testing::spelled_module(
        specforge_project::testing::GSD_EXTENSION,
        "1.0.0",
        &[],
    );
    rebuilt.push(b' ');
    specforge_installed::testing::install_module(
        project.root(),
        &gsd_history_runtime(json!([])),
        specforge_project::testing::GSD_EXTENSION,
        &rebuilt,
    );
    compiled_with(&fake, &project);
    assert_eq!(
        fake.files_asked(),
        3,
        "another module digest compiles again"
    );
    // and the compile is on disk, so a process that has never seen the commit does not compile it
    let cached: Vec<_> = std::fs::read_dir(project.root().join(".specforge/baselines"))
        .unwrap()
        .collect();
    assert_eq!(cached.len(), 4);
}

const HEAD: &str = "1a2b3c4000000000000000000000000000000000";

/// The project whose `a.spec` is `now`, checked against a `HEAD` whose `a.spec` is `head`, and a ledger.
fn judged(head: &str, now: &str, ledger: Option<&str>) -> Vec<specforge_common::Diagnostic> {
    let project = computed::project("");
    project.write("a.spec", now);
    if let Some(ledger) = ledger {
        project.write("specforge-harness", ledger);
    }
    let fake = Arc::new(FakeHistory::repo(
        project.root(),
        HEAD,
        vec![("a.spec".into(), head.as_bytes().to_vec())],
    ));
    compiled_with(&fake, &project)
}

fn at(spec: &str) -> String {
    format!("{}{spec}", computed::BASE)
}

fn decision(origin: &str) -> String {
    at(&format!("decision D1 \"D\" {{\n  phase 03\n{origin}}}\n"))
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an immutable value that differs from its baseline is E084"
)]
fn an_immutable_value_that_differs_from_head_is_e084() {
    let head = decision("  origin context\n");
    let same = judged(&head, &head, None);
    assert!(computed::with_code(&same, "E084").is_empty(), "{same:?}");
    let changed = judged(&head, &decision("  origin project\n"), None);
    let e084 = computed::with_code(&changed, "E084");
    assert_eq!(e084.len(), 1, "{changed:?}");
    assert!(e084[0].message.contains("origin"), "{e084:?}");
    assert!(e084[0].message.contains("1a2b3c4"), "{e084:?}");
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an immutable field unset at the baseline may be set once"
)]
fn an_immutable_field_unset_at_head_may_be_set_once() {
    let unset = decision("");
    let set = decision("  origin context\n");
    let found = judged(&unset, &set, None);
    assert!(computed::with_code(&found, "E084").is_empty(), "{found:?}");
    let removed = judged(&set, &unset, None);
    assert_eq!(
        computed::with_code(&removed, "E084").len(),
        1,
        "{removed:?}"
    );
}

fn discussion(entries: &[&str]) -> String {
    let items: String = entries
        .iter()
        .map(|t| format!("{{ who a text \"{t}\" }} "))
        .collect();
    at(&format!(
        "discussion X \"X\" {{\n  entries [ {items}]\n}}\n"
    ))
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an append-only list must extend its baseline"
)]
fn an_append_only_list_must_extend_its_baseline() {
    let head = discussion(&["one", "two", "three"]);
    let longer = judged(&head, &discussion(&["one", "two", "three", "four"]), None);
    assert!(
        computed::with_code(&longer, "E085").is_empty(),
        "{longer:?}"
    );
    let shorter = judged(&head, &discussion(&["one", "two"]), None);
    assert_eq!(
        computed::with_code(&shorter, "E085").len(),
        1,
        "{shorter:?}"
    );
    let edited = judged(&head, &discussion(&["ONE", "two", "three"]), None);
    assert_eq!(computed::with_code(&edited, "E085").len(), 1, "{edited:?}");
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an append-only kind keeps its entities"
)]
fn an_append_only_kinds_entity_stays() {
    let deviation =
        |id: &str, note: &str| format!("deviation {id} \"{id}\" {{\n  note \"{note}\"\n}}\n");
    let head = at(&format!(
        "{}{}",
        deviation("DV1", "a"),
        deviation("DV2", "b")
    ));
    let removed = judged(&head, &at(&deviation("DV1", "a")), None);
    let e085 = computed::with_code(&removed, "E085");
    assert_eq!(e085.len(), 1, "{removed:?}");
    assert!(e085[0].message.contains("DV2"), "{e085:?}");
    let changed = judged(
        &head,
        &at(&format!(
            "{}{}",
            deviation("DV1", "changed"),
            deviation("DV2", "b")
        )),
        None,
    );
    assert_eq!(
        computed::with_code(&changed, "E084").len(),
        1,
        "{changed:?}"
    );
    let added = judged(
        &head,
        &at(&format!(
            "{}{}{}",
            deviation("DV1", "a"),
            deviation("DV2", "b"),
            deviation("DV3", "c")
        )),
        None,
    );
    assert!(computed::with_code(&added, "E084").is_empty(), "{added:?}");
    assert!(computed::with_code(&added, "E085").is_empty(), "{added:?}");
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "a renamed scope is judged through its move record"
)]
fn a_renamed_scope_is_judged_through_its_move_record() {
    use specforge_project::harness::{Ledger, Line};
    let tree = |phase: &str, origin: &str| {
        format!(
            "milestone v1.0 \"One\" {{}}\n\nphase {phase} \"P\" {{\n  milestone v1.0\n}}\n\n\
             decision D-01 \"D\" {{\n  phase {phase}\n  origin {origin}\n}}\n"
        )
    };
    let mut ledger = Ledger::default();
    ledger.insert(Line::moved("v1.0/05", "v1.0/03"));
    let ledger = ledger.canonical();
    let head = tree("03", "context");
    let renamed = judged(&head, &tree("05", "context"), Some(&ledger));
    assert!(
        computed::with_code(&renamed, "E084").is_empty(),
        "{renamed:?}"
    );
    let by_hand = judged(&head, &tree("05", "context"), None);
    assert!(
        computed::with_code(&by_hand, "E084").is_empty(),
        "an unmatched entity is new: {by_hand:?}"
    );
    let changed = judged(&head, &tree("05", "project"), Some(&ledger));
    let e084 = computed::with_code(&changed, "E084");
    assert_eq!(e084.len(), 1, "{changed:?}");
    assert!(e084[0].message.contains("v1.0/05/D-01"), "{e084:?}");
    assert!(e084[0].message.contains("as 'v1.0/03/D-01'"), "{e084:?}");
}

/// A project over `@test/gsd` with a committed decision, in a real repository, opened as a session.
fn committed_session(origin: &str) -> (TestProject, specforge_project::ProjectSession) {
    let project = computed::project(&format!(
        "decision D \"D\" {{\n  phase 03\n  origin {origin}\n}}\n"
    ));
    git_repo(project.root());
    let runtime = Arc::new(gsd_history_runtime(json!([])));
    let session = project.session_in(runtime);
    (project, session)
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

#[specforge_test(
    behavior = "watch_baseline_refs",
    verify = "a commit re-checks the session"
)]
fn a_commit_rechecks_the_session() {
    let _installed =
        specforge_project::testing::history::install(Arc::new(specforge_git::GitCli::default()));
    let (project, mut session) = committed_session("context");
    let root = project.root().canonicalize().unwrap();
    let e084 = |session: &specforge_project::ProjectSession| {
        computed::with_code(&session.project().diagnostics(), "E084").len()
    };
    assert_eq!(e084(&session), 0);
    // an edit of the committed value: E084
    project.write(
        "a.spec",
        project
            .read("a.spec")
            .replace("origin context", "origin project"),
    );
    let changes = session.changes([root.join("a.spec").as_path()]);
    session.apply(&changes).expect("a source changed");
    assert_eq!(e084(&session), 1);
    // a commit: the ref is a check input, and the check judges against the new commit
    git(&root, &["commit", "-q", "-am", "decide"]);
    let branch = root.join(".git/refs/heads/main");
    let changes = session.changes([branch.as_path()]);
    assert_eq!(changes.check_inputs(), std::slice::from_ref(&branch));
    assert!(changes.sources().is_empty(), "no source is read again");
    session.apply(&changes).expect("a check input changed");
    assert_eq!(e084(&session), 0);
}

#[specforge_test(
    behavior = "watch_baseline_refs",
    verify = "refs outside the root are watched by their directories own entries"
)]
fn refs_outside_the_root_are_watched_by_their_directorys_entries() {
    let _installed =
        specforge_project::testing::history::install(Arc::new(specforge_git::GitCli::default()));
    let project = computed::project("decision D \"D\" {\n  phase 03\n  origin context\n}\n");
    // The git directory lives outside the project (what a linked worktree's common directory is).
    let elsewhere = tempfile::tempdir().unwrap();
    let gitdir = elsewhere.path().join("g");
    git(
        project.root(),
        &[
            "init",
            "-q",
            "-b",
            "main",
            "--separate-git-dir",
            gitdir.to_str().unwrap(),
        ],
    );
    git(project.root(), &["add", "-A"]);
    git(project.root(), &["commit", "-q", "-m", "c"]);
    let session = project.session_in(Arc::new(gsd_history_runtime(json!([]))));
    let gitdir = gitdir.canonicalize().unwrap();
    let root = project.root().canonicalize().unwrap();
    let roots = session.inputs().watch_roots();
    for watch in &roots {
        assert_eq!(watch.recursive, watch.dir == root, "{roots:?}");
    }
    for dir in [gitdir.clone(), gitdir.join("refs/heads")] {
        assert!(roots.iter().any(|r| r.dir == dir), "{dir:?} in {roots:?}");
    }
}

/// `now` checked against a `HEAD` holding `head`, over a project whose only history flags sit on block members.
fn judged_members(journal: bool, head: &str, now: &str) -> Vec<specforge_common::Diagnostic> {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[specforge_project::testing::GSD_EXTENSION])
        .installing_enabled()
        .file("a.spec", now);
    let fake = Arc::new(FakeHistory::repo(
        project.root(),
        HEAD,
        vec![("a.spec".into(), head.as_bytes().to_vec())],
    ));
    let _installed = specforge_project::testing::history::install(fake);
    project
        .compile_in(Arc::new(
            specforge_project::testing::gsd_member_history_runtime(journal),
        ))
        .diagnostics()
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "a history flag that sits only on a block member is judged"
)]
fn a_flag_only_on_a_member_is_judged() {
    let checks = |kind: &str| {
        format!("deliverable D \"D\" {{\n  checks [ {{ kind {kind}  ref \"t\" }} ]\n}}\n")
    };
    let same = judged_members(false, &checks("unit"), &checks("unit"));
    assert!(computed::with_code(&same, "E084").is_empty(), "{same:?}");
    assert!(computed::with_code(&same, "I204").is_empty(), "{same:?}");
    let changed = judged_members(false, &checks("unit"), &checks("e2e"));
    assert_eq!(
        computed::with_code(&changed, "E084").len(),
        1,
        "{changed:?}"
    );
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "a change inside a block of an append-only kind is one E084"
)]
fn a_member_change_in_an_append_only_kind_is_reported_once() {
    let journal = |kind: &str| format!("journal J \"J\" {{\n  parts [ {{ kind {kind} }} ]\n}}\n");
    let changed = judged_members(true, &journal("a"), &journal("b"));
    assert_eq!(
        computed::with_code(&changed, "E084").len(),
        1,
        "{changed:?}"
    );
}

/// Phases `03` and `04`, each with a finding and a decision, the phases numbered as `first` and `second` say.
fn renumbered(first: &str, second: &str, notes: [&str; 2]) -> String {
    let phase = |n: &str, note: &str| {
        format!(
            "phase {n} \"P\" {{\n  milestone v1.0\n}}\n\nfinding F \"F\" {{\n  phase {n}\n  note \"{note}\"\n}}\n\n"
        )
    };
    format!(
        "milestone v1.0 \"One\" {{}}\n\n{}{}",
        phase(first, notes[0]),
        phase(second, notes[1])
    )
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "a renumbering chain is judged through both move records at once"
)]
fn a_renumbering_chain_is_judged_through_both_move_records() {
    use specforge_project::harness::{Ledger, Line};
    let mut ledger = Ledger::default();
    ledger.insert(Line::moved("v1.0/05", "v1.0/04"));
    ledger.insert(Line::moved("v1.0/04", "v1.0/03"));
    let ledger = ledger.canonical();
    // HEAD: 03 and 04. Now: the old 04 is 05 and the old 03 is 04.
    let head = renumbered("03", "04", ["a", "b"]);
    let same = judged(&head, &renumbered("04", "05", ["a", "b"]), Some(&ledger));
    for code in ["E084", "E085"] {
        assert!(computed::with_code(&same, code).is_empty(), "{same:?}");
    }
    // the finding that was 03 and is 04 changed: judged against 03's, not skipped and not against 04's
    let changed = judged(&head, &renumbered("04", "05", ["A", "b"]), Some(&ledger));
    let e084 = computed::with_code(&changed, "E084");
    assert_eq!(e084.len(), 1, "{changed:?}");
    assert!(e084[0].message.contains("v1.0/04/F"), "{e084:?}");
    assert!(e084[0].message.contains("as 'v1.0/03/F'"), "{e084:?}");
    // a record the baseline commit's own ledger holds was applied already: nothing is moved again
    let project = computed::project("");
    project.write("a.spec", renumbered("04", "05", ["a", "b"]));
    project.write("specforge-harness", &ledger);
    let fake = Arc::new(FakeHistory::repo(
        project.root(),
        HEAD,
        vec![
            (
                "a.spec".into(),
                renumbered("04", "05", ["a", "b"]).into_bytes(),
            ),
            ("specforge-harness".into(), ledger.clone().into_bytes()),
        ],
    ));
    let found = compiled_with(&fake, &project);
    for code in ["E084", "E085"] {
        assert!(computed::with_code(&found, code).is_empty(), "{found:?}");
    }
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "only a reference value is respelled by a move record"
)]
fn only_a_reference_is_respelled_by_a_move_record() {
    use specforge_project::harness::{Ledger, Line};
    let mut ledger = Ledger::default();
    ledger.insert(Line::moved("v1.0/05", "v1.0/03"));
    let ledger = ledger.canonical();
    // `component` is free text that happens to equal the old path
    let tree = |phase: &str| {
        format!(
            "milestone v1.0 \"One\" {{}}\n\nphase {phase} \"P\" {{\n  milestone v1.0\n}}\n\n\
             finding F \"F\" {{\n  phase {phase}\n  component \"v1.0/03\"\n}}\n"
        )
    };
    let found = judged(&tree("03"), &tree("05"), Some(&ledger));
    for code in ["E084", "E085"] {
        assert!(computed::with_code(&found, code).is_empty(), "{found:?}");
    }
}
