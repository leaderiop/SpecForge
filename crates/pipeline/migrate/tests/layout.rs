//! A migration lays out the blocks its steps changed with the file's format configuration (ADR 0235), and no other.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_formatter::{FormatConfig, format_source};
use specforge_graph::FieldForms;
use specforge_migrate::{MigrationStatus, migrate_project};
use specforge_parser::CURRENT_FORMAT_VERSION;
use specforge_test_macros::test as specforge_test;
use std::path::Path;
use tempfile::TempDir;

/// Five `behavior` blocks writing the same two lists: the hoist fires (10 rows > 2 fields + 3).
const BEHAVIORS: &str = "behavior a \"T\" {\n  ports [R]\n  features [F]\n}\n\nbehavior b \"T\" {\n  ports [R]\n  features [F]\n}\n\nbehavior c \"T\" {\n  ports [R]\n  features [F]\n}\n\nbehavior d \"T\" {\n  ports [R]\n  features [F]\n}\n\nbehavior e \"T\" {\n  ports [R]\n  features [F]\n}\n";

/// `text` as a project's one source, migrated under `config`: the summary and what the plan makes of the file.
fn migrated_with(
    text: &str,
    config: &FormatConfig,
) -> (specforge_migrate::MigrationSummary, String) {
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
        &|_| config.clone(),
    );
    let after = summary
        .rewrites
        .first()
        .map_or_else(|| text.to_string(), |rewrite| rewrite.after.clone());
    (summary, after)
}

#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "every block a migration step changed is laid out with its file's format configuration"
)]
fn every_block_a_step_changed_is_laid_out_with_the_given_configuration() {
    let config = FormatConfig {
        use_tabs: true,
        ..FormatConfig::default()
    };
    let (_, after) = migrated_with(
        &format!("// Doc.\ntype C {{\n  id string\n}}\n\n{BEHAVIORS}"),
        &config,
    );

    assert!(
        after.contains("defaults behavior {\n\tports    [R]\n\tfeatures [F]\n}\n"),
        "{after}"
    );
    assert!(
        after.contains("type C \"Doc.\" {\n\tid string\n}\n"),
        "{after}"
    );
    assert!(after.contains("behavior a \"T\" {\n}\n"), "{after}");
    assert_eq!(
        format_source(&after, &config, &FieldForms::default()).formatted,
        after
    );
}

#[specforge_test(
    invariant = "migration_canonical_form",
    verify = "a block no migration step changed is left as written"
)]
fn a_block_no_step_changed_is_left_as_written() {
    let untouched = "port R \"R\" {\n      direction inbound\n}\n";
    let (_, after) = migrated_with(
        &format!("{BEHAVIORS}\n{untouched}"),
        &FormatConfig::default(),
    );

    assert!(after.contains(untouched), "{after}");
    assert!(after.contains("defaults behavior {"), "{after}");
}

#[specforge_test(
    invariant = "migration_idempotency",
    verify = "a file no migration step changes is skipped as written, unformatted"
)]
fn a_file_no_step_changes_is_skipped_as_written() {
    let (summary, after) = migrated_with(
        "port R \"R\" {\n      direction inbound\n}\n",
        &FormatConfig::default(),
    );

    assert!(summary.rewrites.is_empty(), "{summary:?}");
    assert_eq!(summary.results[0].status, MigrationStatus::Skipped);
    assert_eq!(after, "port R \"R\" {\n      direction inbound\n}\n");
}
