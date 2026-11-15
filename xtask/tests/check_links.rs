//! `check-links` on a fixture project: one declared link, one `verify` text
//! the entity does not declare (A016), one link to an entity that is not there.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::process::Command;

fn run(root: &str) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_check-links"))
        .arg(root)
        .arg("tests")
        .output()
        .expect("check-links runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn reports_the_undeclared_text_and_the_unknown_entity() {
    let (code, stdout) = run(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/links"));
    assert_eq!(code, 1, "{stdout}");
    assert_eq!(
        stdout.lines().filter(|l| l.starts_with("A016 ")).count(),
        1,
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "A016 tests/links.rs:7 task_id_uniqueness \"concurrent task creation never produces duplicate id\""
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("declared: \"concurrent task creation never produces duplicate ids\""),
        "{stdout}"
    );
    assert!(
        stdout.contains("UNKNOWN tests/links.rs:13 no_such_invariant"),
        "{stdout}"
    );
    assert!(!stdout.contains("links.rs:2 "), "{stdout}");
}

/// A file that does not parse is reported on stderr under check-links' own name, and so is one in the macro's
/// self-test directory: its links are left out, its parse is not. Nothing else is on stderr.
#[test]
fn a_file_that_does_not_parse_is_reported_under_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let fixture =
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/links"));
    for file in ["specforge.json", "spec/invariants.spec", "tests/links.rs"] {
        let to = root.join(file);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(fixture.join(file), &to).unwrap();
    }
    let self_test = root.join("integrations/rust/specforge-test");
    std::fs::create_dir_all(&self_test).unwrap();
    std::fs::write(root.join("tests/broken.rs"), "fn (").unwrap();
    std::fs::write(self_test.join("broken.rs"), "fn (").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_check-links"))
        .arg(root)
        .args(["tests", "integrations"])
        .output()
        .expect("check-links runs");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        format!(
            "check-links: skipped {}: does not parse\ncheck-links: skipped {}: does not parse\n",
            root.join("tests/broken.rs").display(),
            self_test.join("broken.rs").display()
        )
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "the fixture's findings still stand"
    );
}
