//! The journal of a write that removes files, and of the host files install writes (ADR 0292).

use super::*;
use specforge_common::Exclude;
use specforge_test_macros::test as specforge_test;

fn sources(root: &std::path::Path) -> SourceRoot {
    SourceRoot::new(root.to_path_buf(), root.to_path_buf(), Exclude::default())
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "rollback restores a removed file and removes a created host file, and refuses a path no write records"
)]
fn rollback_puts_back_a_removed_host_file_and_unwrites_a_created_one() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
    let old = root.join(".claude/agents/old.md");
    let new = root.join(".claude/agents/new.md");
    let mut record = Record::new(&sources(root));
    record.add_removal(".claude/agents/old.md", b"# old\n");
    record.add(".claude/agents/new.md", None, b"# new\n");
    record.begin("cannot write the host files").unwrap();
    // The write is killed after both files were written.
    std::fs::write(&new, "# new\n").unwrap();
    assert!(!old.exists());

    let put_back = rollback(&sources(root), crate::testing::caller()).unwrap();
    assert_eq!(put_back.len(), 2);
    assert_eq!(std::fs::read_to_string(&old).unwrap(), "# old\n");
    assert!(!new.exists());
    assert!(!root.join(JOURNAL_FILE).exists());
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "rollback leaves a removed file alone when something else was written at its name since"
)]
fn rollback_leaves_a_removed_file_that_was_written_again() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
    let mut record = Record::new(&sources(root));
    record.add_removal(".claude/agents/old.md", b"# old\n");
    record.begin("cannot write the host files").unwrap();
    std::fs::write(root.join(".claude/agents/old.md"), "mine\n").unwrap();
    rollback(&sources(root), crate::testing::caller()).unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join(".claude/agents/old.md")).unwrap(),
        "mine\n"
    );
}

#[specforge_test(
    behavior = "roll_back_interrupted_write",
    verify = "rollback refuses a journal that names a file no write of the project records"
)]
fn rollback_refuses_a_path_no_write_records() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut record = Record::new(&sources(root));
    record.add_removal("src/main.rs", b"fn main() {}\n");
    record.begin("cannot write the host files").unwrap();
    assert!(rollback(&sources(root), crate::testing::caller()).is_err());
}
