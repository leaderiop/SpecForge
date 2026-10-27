//! A project that declares no writer or history flag reads no git and no ledger (ADR 0287): what a session
//! watches is what it watched before.

use std::path::Path;
use std::process::Command;

use specforge_project::Watched;
use specforge_project::testing::TestProject;
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
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "no history is I204 only when history is declared"
)]
#[ignore = "K4 T7"]
fn no_history_is_i204_only_when_history_is_declared() {
    // the flagged fixture outside git, or on an unborn HEAD, gives one I204; the unflagged fixture gives nothing
    unimplemented!(
        "K4 T7: the flagged fixture outside git, or on an unborn HEAD, gives one I204; the unflagged fixture gives nothing"
    );
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "an unreadable baseline is W164 and nothing is judged"
)]
#[ignore = "K4 T7"]
fn an_unreadable_baseline_is_w164() {
    // FakeHistory files error and Unavailable give W164
    unimplemented!("K4 T7: FakeHistory files error and Unavailable give W164");
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "the baseline is cached by commit and environment"
)]
#[ignore = "K4 T7"]
fn the_baseline_is_cached_by_commit_and_environment() {
    // the second check of one sha compiles nothing; another module digest recompiles
    unimplemented!(
        "K4 T7: the second check of one sha compiles nothing; another module digest recompiles"
    );
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an immutable value that differs from its baseline is E084"
)]
#[ignore = "K4 T8"]
fn an_immutable_value_that_differs_from_head_is_e084() {
    // commit decision D-01, change origin: E084 naming HEAD short sha
    unimplemented!("K4 T8: commit decision D-01, change origin: E084 naming HEAD short sha");
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an immutable field unset at the baseline may be set once"
)]
#[ignore = "K4 T8"]
fn an_immutable_field_unset_at_head_may_be_set_once() {
    // unset to set is clean; set to removed is E084
    unimplemented!("K4 T8: unset to set is clean; set to removed is E084");
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an append-only list must extend its baseline"
)]
#[ignore = "K4 T8"]
fn an_append_only_list_must_extend_its_baseline() {
    // 3 items to 4 clean; to 2, or an edited first item: E085
    unimplemented!("K4 T8: 3 items to 4 clean; to 2, or an edited first item: E085");
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "an append-only kind keeps its entities"
)]
#[ignore = "K4 T8"]
fn an_append_only_kinds_entity_stays() {
    // a committed deviation removed: E085; changed: E084; one added: clean
    unimplemented!("K4 T8: a committed deviation removed: E085; changed: E084; one added: clean");
}

#[specforge_test(
    behavior = "judge_field_history",
    verify = "a renamed scope is judged through its move record"
)]
#[ignore = "K4 T8"]
fn a_renamed_scope_is_judged_through_its_move_record() {
    // ledger v1.0/05 moved:v1.0/03; respelled tree clean; changed D-01.origin E084 naming both paths
    unimplemented!(
        "K4 T8: ledger v1.0/05 moved:v1.0/03; respelled tree clean; changed D-01.origin E084 naming both paths"
    );
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
