//! `check-links` on the gsd fixture project (ADR 0374 D4): every spec-link attribute names a requirement of the
//! spec and carries no `verify` text.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::process::Command;

/// The gsd fixture (ADR 0374 D4): an attribute whose kind word is `requirement` names a requirement of the spec, and
/// carries no `verify` text. One that names no entity is `UNKNOWN`, one that names an entity of another kind (the
/// project) or uses another kind word is `KIND`, one with a `verify` text is `VERIFY`; two on one line are each
/// judged; the fixture's own tests, two stacked on one function, pass.
#[test]
fn requirement_links_name_requirements_of_the_spec() {
    let fixture = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/requirement-links"
    ));
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for file in ["specforge.json", "spec/project.spec", "src/lib.rs"] {
        let to = root.join(file);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(fixture.join(file), &to).unwrap();
    }
    std::fs::create_dir_all(root.join("tests/fixtures/other/src")).unwrap();
    std::fs::write(
        root.join("tests/bad.rs"),
        "#[specforge_test(requirement = \"NOPE-99\")]\nfn a() {}\n\
         #[specforge_test(requirement = \"tracer\")]\nfn b() {}\n\
         #[specforge_test(requirement = \"TRACER-01\", verify = \"x\")]\nfn c() {}\n\
         #[specforge_test(behavior = \"TRACER-01\")]\nfn d() {}\n\
         #[specforge_test(requirement = \"NOPE-97\")] #[specforge_test(requirement = \"NOPE-96\")]\n\
         fn f() {}\n",
    )
    .unwrap();
    // A helper of this project's tests under `tests/fixtures/` is checked; a crate there is another project's.
    let unknown = "#[specforge_test(requirement = \"NOPE-98\")]\nfn e() {}\n";
    std::fs::write(root.join("tests/fixtures/helper.rs"), unknown).unwrap();
    std::fs::write(root.join("tests/fixtures/other/Cargo.toml"), "").unwrap();
    std::fs::write(root.join("tests/fixtures/other/src/lib.rs"), unknown).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_check-links"))
        .arg(root)
        .args(["tests", "src"])
        .output()
        .expect("check-links runs");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "UNKNOWN tests/bad.rs:1 NOPE-99\n\
         KIND tests/bad.rs:3 requirement tracer is a project, not a requirement\n\
         VERIFY tests/bad.rs:5 TRACER-01: a requirement takes no verify text\n\
         KIND tests/bad.rs:7 behavior TRACER-01: a test names a requirement\n\
         UNKNOWN tests/bad.rs:9 NOPE-97\n\
         UNKNOWN tests/bad.rs:9 NOPE-96\n\
         UNKNOWN tests/fixtures/helper.rs:1 NOPE-98\n"
    );
    assert_eq!(out.status.code(), Some(1));

    let committed = Command::new(env!("CARGO_BIN_EXE_check-links"))
        .arg(fixture)
        .arg("src")
        .output()
        .expect("check-links runs");
    assert_eq!(
        (
            committed.status.code(),
            String::from_utf8_lossy(&committed.stdout).into_owned()
        ),
        (Some(0), String::new()),
        "{}",
        String::from_utf8_lossy(&committed.stderr)
    );
}

/// A file that does not parse is reported on stderr under check-links' own name, and so is one in the macro's
/// self-test directory: its links are left out, its parse is not. Nothing else is on stderr.
#[test]
fn a_file_that_does_not_parse_is_reported_under_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let fixture = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/requirement-links"
    ));
    for file in ["specforge.json", "spec/project.spec"] {
        let to = root.join(file);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(fixture.join(file), &to).unwrap();
    }
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("tests/links.rs"),
        "#[specforge_test(requirement = \"NOPE-99\")]\nfn a() {}\n",
    )
    .unwrap();
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
