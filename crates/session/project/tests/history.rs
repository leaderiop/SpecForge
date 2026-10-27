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
    // and the compile is on disk, so a process that has never seen the commit does not compile it
    let cached: Vec<_> = std::fs::read_dir(project.root().join(".specforge/baselines"))
        .unwrap()
        .collect();
    assert_eq!(cached.len(), 3);
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

#[specforge_test(
    behavior = "watch_baseline_refs",
    verify = "a commit re-checks the session"
)]
#[ignore = "K4 T12"]
fn a_commit_rechecks_the_session() {
    // edit origin: E084; git commit: changes() names the ref as a check input; apply: E084 gone
    unimplemented!(
        "K4 T12: edit origin: E084; git commit: changes() names the ref as a check input; apply: E084 gone"
    );
}

#[specforge_test(
    behavior = "watch_baseline_refs",
    verify = "refs outside the root are watched by their directories own entries"
)]
#[ignore = "K4 T12"]
fn refs_outside_the_root_are_watched_by_their_directorys_entries() {
    // in a linked worktree watch_roots holds the common dir and git dir with recursive false only
    unimplemented!(
        "K4 T12: in a linked worktree watch_roots holds the common dir and git dir with recursive false only"
    );
}
