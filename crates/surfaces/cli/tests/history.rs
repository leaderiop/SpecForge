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
