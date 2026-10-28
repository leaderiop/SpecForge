//! A file the change removes (ADR 0292): checked like a replacement, unlinked, written again when the change is put
//! back.

use super::*;
use crate::Containment;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

fn scene() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    (dir, root)
}

fn held(root: &Path, name: &str) -> Held {
    Containment::within(root).resolve(Path::new(name)).unwrap()
}

#[test]
fn a_staged_removal_unlinks_the_file() {
    let (_dir, root) = scene();
    std::fs::write(root.join("gone"), "old").unwrap();
    let mut change = ProjectChange::new(&root, "cannot test");
    change.stage_removal(held(&root, "gone"), b"old".to_vec());
    let writes = change.commit().unwrap();
    assert_eq!(writes.len(), 1);
    assert!(!root.join("gone").exists());
}

#[test]
fn a_removal_over_a_file_that_changed_is_refused_and_removes_nothing() {
    let (_dir, root) = scene();
    std::fs::write(root.join("gone"), "edited since").unwrap();
    let mut change = ProjectChange::new(&root, "cannot test");
    change.stage_removal(held(&root, "gone"), b"old".to_vec());
    let error = change.commit().unwrap_err();
    assert_eq!(error.code, FILE_CHANGED);
    assert_eq!(
        std::fs::read_to_string(root.join("gone")).unwrap(),
        "edited since"
    );
}

#[test]
#[cfg(unix)]
fn a_removal_is_put_back_when_a_later_file_cannot_be_written() {
    let (_dir, root) = scene();
    std::fs::write(root.join("gone"), "old").unwrap();
    // The directory the second file needs cannot be written to.
    std::fs::create_dir(root.join("blocked")).unwrap();
    std::fs::set_permissions(root.join("blocked"), std::fs::Permissions::from_mode(0o555)).unwrap();
    let mut change = ProjectChange::new(&root, "cannot test");
    change.stage_removal(held(&root, "gone"), b"old".to_vec());
    change.stage(held(&root, "blocked/new"), None, b"new".to_vec());
    let error = change.commit().unwrap_err();
    assert_eq!(error.code, FILE_UNWRITABLE);
    assert_eq!(std::fs::read_to_string(root.join("gone")).unwrap(), "old");
    std::fs::set_permissions(root.join("blocked"), std::fs::Permissions::from_mode(0o755)).unwrap();
}
