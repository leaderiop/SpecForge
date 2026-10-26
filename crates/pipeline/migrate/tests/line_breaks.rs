//! A migration reads and writes a file in its own line break (ADR 0266).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_formatter::FormatConfig;
use specforge_graph::FieldForms;
use specforge_migrate::migrate_project;
use specforge_parser::CURRENT_FORMAT_VERSION;
use specforge_test_macros::test as specforge_test;
use std::path::Path;
use tempfile::TempDir;

/// `text` as a project's one source, migrated to the current version; what the file holds after.
fn migrated(text: &str) -> String {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"p","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("spec")).unwrap();
    let a = dir.path().join("spec/a.spec");
    std::fs::write(&a, text).unwrap();
    let summary = migrate_project(
        &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
        std::slice::from_ref(&a),
        &CURRENT_FORMAT_VERSION,
        &|path: &Path| std::fs::read_to_string(path),
        &FieldForms::default(),
        &|_| FormatConfig::default(),
    );
    summary
        .rewrites
        .first()
        .map_or_else(|| text.to_string(), |rewrite| rewrite.after.clone())
}

const OLD: &str = "// specforge-format: 0.9\n// The first.\nfeature f1 {\n  status proposed\n  problem \"p\"\n}\n\nbehavior a \"T\" {\n  ports [R]\n  features [F]\n}\n\nbehavior b \"T\" {\n  ports [R]\n  features [F]\n}\n\nbehavior c \"T\" {\n  ports [R]\n  features [F]\n}\n";

#[specforge_test(
    invariant = "migration_canonical_form",
    verify = "a migration writes a CRLF file in CRLF, as it migrates the same file in LF"
)]
#[ignore = "flipped by T7"]
fn a_migration_writes_a_crlf_file_as_it_migrates_its_lf_reading() {
    let (lf, crlf) = (migrated(OLD), migrated(&OLD.replace('\n', "\r\n")));
    assert_ne!(lf, OLD, "the LF file is migrated");
    assert!(!crlf.replace("\r\n", "").contains('\n'), "{crlf:?}");
    assert_eq!(crlf.replace("\r\n", "\n"), lf);
}
