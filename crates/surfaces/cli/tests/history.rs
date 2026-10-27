//! A project that declares no history reports the same inside and outside a git repository (ADR 0287).

use std::path::Path;
use std::process::Command;

use crate::support::{Cli, TestProject};

fn fixture() -> TestProject {
    TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/product_cycle"),
    )
}

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
#[ignore = "K4 T10"]
fn check_cache_is_gone() {
    let dir = fixture();
    dir.write("specforge-cache.json", "not json");
    Cli::on(&dir).run(&["check", "--cache"]).exits(2);
    let plain = Cli::on(&dir).run(&["check", "--format", "json"]);
    assert!(!plain.stdout.contains("W144"), "{}", plain.stdout);
}

#[specforge_test_macros::test(
    behavior = "read_git_baseline",
    verify = "check since judges history against the merge-base and refuses an unknown revision"
)]
#[ignore = "K4 T11"]
fn check_since_judges_against_the_merge_base() {
    let repo = fixture();
    git_repo(repo.root());
    // The E084 half needs an extension that declares an immutable field: MCP's test drives it in process
    // (`validate_since_judges_per_request`). Here, a revision git does not know is refused.
    let run = Cli::on(&repo).run(&["check", "--since", "nosuch", "--format", "json"]);
    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("unknown_revision") || run.stderr.contains("unknown_revision"));
}
