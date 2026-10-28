use super::*;
use crate::writes::{changed_since, files_under};
use specforge_ops::authoring::edit::Obligation;
use specforge_ops::{Containment, OpErrorKind};
use std::path::PathBuf;

const RULE: &str = "invariant task_unique \"Task Unique\" {\n  guarantee \"g\"\n}\n";

fn root(c: &Compiled) -> PathBuf {
    c.project.environment().spec_root().to_path_buf()
}

fn behavior(id: &str) -> String {
    format!(
        "behavior {id} \"B\" {{\n  invariants [task_unique]\n  contract \"The system MUST work\"\n}}\n"
    )
}

fn two_dirs() -> Compiled {
    project(&[
        ("rules.spec", RULE),
        ("a/one.spec", &behavior("beta_one")),
        ("b/two.spec", &behavior("beta_two")),
    ])
}

fn unlink_both() -> Vec<Edit> {
    vec![
        unlink("beta_one", "invariants", "task_unique"),
        unlink("beta_two", "invariants", "task_unique"),
    ]
}

#[test]
fn apply_writes_the_planned_files() {
    let c = two_dirs();
    let root = root(&c);
    let before = files_under(&root);
    let planned = plan(&c, &unlink_both()).unwrap();
    let writes = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    assert_eq!(changed_since(&root, &before), ["a/one.spec", "b/two.spec"]);
    assert_eq!(
        crate::writes::listed(&writes, &root),
        ["a/one.spec", "b/two.spec"]
    );
}

#[specforge_test_macros::test(
    behavior = "apply_edit_plan",
    verify = "a typed edit writes the same bytes and report"
)]
fn a_typed_edit_writes_the_same_bytes_and_report() {
    let c = two_dirs();
    let root = root(&c);
    let planned = plan(&c, &unlink_both()).unwrap();
    assert_eq!(
        planned.report().affected_files,
        ["a/one.spec", "b/two.spec"]
    );
    assert_eq!(planned.report().entities, ["beta_one", "beta_two"]);
    specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    for file in ["a/one.spec", "b/two.spec"] {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        assert!(!text.contains("invariants"), "{text}");
        assert!(text.starts_with("behavior beta_"), "{text}");
    }
    // A project with no harness field gets no `.specforge/` from a write.
    let project = c.project.root().unwrap();
    assert!(!project.join(".specforge").exists());
}

#[specforge_test_macros::test(
    behavior = "validate_typed_edit",
    verify = "a failed write puts every file back"
)]
#[cfg(unix)]
fn apply_writes_every_file_or_none() {
    use std::os::unix::fs::PermissionsExt;
    let c = two_dirs();
    let root = root(&c);
    let planned = plan(&c, &unlink_both()).unwrap();
    let locked = root.join("b");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    let before = files_under(&root);
    let error = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(error.code, specforge_ops::FILE_UNWRITABLE, "{error:?}");
    assert!(changed_since(&root, &before).is_empty());
}

#[specforge_test_macros::test(
    behavior = "refuse_stale_revision",
    verify = "a typed edit naming no revision is refused file_changed when its file changed since the compile"
)]
fn apply_refuses_a_file_changed_since_the_plan() {
    let c = two_dirs();
    let root = root(&c);
    let planned = plan(&c, &unlink_both()).unwrap();
    std::fs::write(
        root.join("a/one.spec"),
        format!("\n{}", behavior("beta_one")),
    )
    .unwrap();
    let before = files_under(&root);
    let error = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    assert_eq!(error.kind, OpErrorKind::Conflict);
    assert_eq!(error.code, specforge_ops::FILE_CHANGED);
    assert!(error.message.contains("one.spec"), "{}", error.message);
    assert!(changed_since(&root, &before).is_empty());
}

#[cfg(unix)]
#[test]
fn apply_writes_nothing_outside_the_containment() {
    let c = two_dirs();
    let root = root(&c);
    let planned = plan(&c, &unlink_both()).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("elsewhere.spec");
    std::fs::write(&target, behavior("beta_one")).unwrap();
    std::fs::remove_file(root.join("a/one.spec")).unwrap();
    std::os::unix::fs::symlink(&target, root.join("a/one.spec")).unwrap();
    let containment = Containment::within(&root);
    let error = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::bounded(&containment),
    )
    .unwrap_err();
    assert_eq!(error.code, specforge_ops::OUTSIDE_ROOT, "{error:?}");
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        behavior("beta_one")
    );
    assert!(
        std::fs::read_to_string(root.join("b/two.spec"))
            .unwrap()
            .contains("invariants")
    );
}

#[cfg(unix)]
#[test]
fn apply_creates_the_new_file_and_removes_it_on_failure() {
    use std::os::unix::fs::PermissionsExt;
    let c = two_dirs();
    let root = root(&c);
    let create = Edit::Create {
        kind: "behavior".into(),
        id: "gamma_new".into(),
        title: "G".into(),
        file: "fresh/gamma.spec".into(),
        fields: [("contract".to_string(), one("The system MUST work"))].into(),
        verify: vec![Obligation {
            kind: Some("unit".into()),
            text: "works".into(),
        }],
    };
    let edits = vec![create, unlink("beta_two", "invariants", "task_unique")];
    let planned = plan(&c, &edits).unwrap();
    let before = files_under(&root);
    let locked = root.join("b");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    let error = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(error.code, specforge_ops::FILE_UNWRITABLE, "{error:?}");
    assert!(changed_since(&root, &before).is_empty());
    assert!(
        !root.join("fresh").exists(),
        "the directory it made is gone"
    );

    let writes = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    assert!(root.join("fresh/gamma.spec").is_file());
    assert_eq!(
        crate::writes::listed(&writes, &root),
        ["b/two.spec", "fresh/gamma.spec"]
    );
}

#[test]
fn an_empty_plan_writes_nothing() {
    let c = two_dirs();
    let root = root(&c);
    let before = files_under(&root);
    let planned = plan(&c, &[unlink("beta_one", "invariants", "nope")]).unwrap();
    assert!(planned.change().is_empty());
    let writes = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    assert!(writes.is_empty());
    assert!(changed_since(&root, &before).is_empty());
}

#[cfg(unix)]
#[test]
fn an_edit_that_cannot_write_a_file_is_unwritable() {
    use std::os::unix::fs::PermissionsExt;
    let c = two_dirs();
    let root = root(&c);
    let planned = plan(&c, &unlink_both()).unwrap();
    let locked = root.join("b/two.spec");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444)).unwrap();
    if std::fs::OpenOptions::new()
        .append(true)
        .open(&locked)
        .is_ok()
    {
        return; // permissions do not bind (root)
    }
    let before = files_under(&root);
    let error = specforge_ops::authoring::apply::testing::apply(
        planned.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(error.code, specforge_ops::FILE_UNWRITABLE, "{error:?}");
    assert_eq!(error.kind, OpErrorKind::PermissionDenied, "{error:?}");
    assert!(
        error
            .message
            .starts_with("cannot apply the edits: failed to write b/two.spec: "),
        "{}",
        error.message
    );
    assert!(
        error.message.ends_with("; nothing was written"),
        "{}",
        error.message
    );
    assert!(changed_since(&root, &before).is_empty());
}
