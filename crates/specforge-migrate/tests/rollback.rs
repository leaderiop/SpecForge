//! What `specforge migrate --rollback` restores.

use specforge_migrate::{migrate_project, run_rollback};
use specforge_parser::CURRENT_FORMAT_VERSION;
use tempfile::TempDir;

const OLD: &str = "// specforge-format: 0.9\nbehavior alpha \"Alpha\" {\n}\n";

/// A project with one source, `spec/a.spec`, at format 0.9.
fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"p","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("spec")).unwrap();
    std::fs::write(dir.path().join("spec/a.spec"), OLD).unwrap();
    dir
}

// pin (16-T0): flipped by T8.
#[test]
fn a_rollback_restores_a_file_edited_since_the_migration() {
    let dir = project();
    let target = CURRENT_FORMAT_VERSION;
    let summary = migrate_project(dir.path(), &target, false, false);
    assert_eq!(summary.migrated_count, 1, "{summary:?}");
    let a = dir.path().join("spec/a.spec");
    let migrated = std::fs::read_to_string(&a).unwrap();
    assert_ne!(migrated, OLD);
    std::fs::write(&a, format!("{migrated}// edited\n")).unwrap();

    let rolled = run_rollback(dir.path());

    assert_eq!(rolled.restored_count, 1, "{rolled:?}");
    assert_eq!(
        std::fs::read_to_string(&a).unwrap(),
        OLD,
        "the stale backup replaced the edit"
    );
}
