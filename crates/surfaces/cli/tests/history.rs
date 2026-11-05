//! A project that declares no history reports the same inside and outside a git repository (ADR 0287).

use std::path::Path;

use crate::support::{Cli, TestProject};

fn fixture() -> TestProject {
    TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/product_cycle"),
    )
}

/// A repository at `root` with one commit of everything under it, built with no git program.
pub(crate) fn git_repo(root: &Path) {
    specforge_git::testing::Repo::init(root).commit_all("c");
}

#[specforge_test_macros::test(
    behavior = "read_git_baseline",
    verify = "check reports the same inside and outside a git repository when nothing declares history"
)]
fn check_reports_the_same_inside_and_outside_a_repository() {
    let plain = fixture();
    let repo = fixture();
    git_repo(repo.root());
    let outside = Cli::on(&plain).run(&["check", "--format", "json"]);
    let inside = Cli::on(&repo).run(&["check", "--format", "json"]);
    assert_eq!(outside.code, inside.code);
    assert_eq!(outside.stdout, inside.stdout);
}

#[specforge_test_macros::test(
    behavior = "run_check_phase_passes",
    verify = "check --cache is a usage error and a specforge-cache.json is not read"
)]
fn check_cache_is_gone() {
    let dir = fixture();
    dir.write("specforge-cache.json", "not json");
    Cli::on(&dir).run(&["check", "--cache"]).exits(2);
    let plain = Cli::on(&dir).run(&["check", "--format", "json"]);
    assert!(!plain.stdout.contains("W144"), "{}", plain.stdout);
}

#[specforge_test_macros::test(
    behavior = "read_git_baseline",
    verify = "check since reads history with no git program on PATH"
)]
fn check_since_reads_history_with_no_git_on_path() {
    let repo = fixture();
    git_repo(repo.root());
    // Nothing to install: the history is read in process, so a machine with no git program reads it too.
    let empty = tempfile::tempdir().unwrap();
    let unknown = Cli::on(&repo)
        .env("PATH", empty.path())
        .run(&["check", "--since", "nosuch", "--format", "json"]);
    assert_eq!(unknown.code, 2, "{}{}", unknown.stdout, unknown.stderr);
    assert!(
        unknown.stdout.contains("unknown_revision") || unknown.stderr.contains("unknown_revision"),
        "{}{}",
        unknown.stdout,
        unknown.stderr
    );
    let head = Cli::on(&repo)
        .env("PATH", empty.path())
        .run(&["check", "--since", "HEAD", "--format", "json"]);
    assert!(!head.stdout.contains("W164"), "{}", head.stdout);
}

#[specforge_test_macros::test(
    behavior = "read_git_baseline",
    verify = "check since judges history against the merge-base and refuses an unknown revision"
)]
fn check_since_judges_against_the_merge_base() {
    let repo = fixture();
    git_repo(repo.root());
    // The E084 half needs an extension that declares an immutable field: MCP's test drives it in process
    // (`validate_since_judges_per_request`). Here, a revision git does not know is refused.
    let run = Cli::on(&repo).run(&["check", "--since", "nosuch", "--format", "json"]);
    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("unknown_revision") || run.stderr.contains("unknown_revision"));
}
