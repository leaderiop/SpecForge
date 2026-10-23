//! A migration lays out what its steps write with the file's format configuration (ADR 0235): the defaults block
//! and every block the hoist shortened are format-clean after it.

use specforge_ops::authoring::format::{self, Forms, Place};
use specforge_test_macros::test as specforge_test;

use crate::migrate::{request, run};

/// `f1` and six milestones writing `status planned` and `features [f1]`, as `format` writes them at `indent`:
/// the hoist fires (6 rows > 1 field + 3).
fn milestones(indent: &str) -> String {
    let mut text = format!("feature f1 \"F1\" {{\n{indent}problem \"p\"\n}}\n");
    for i in 1..=6 {
        text.push_str(&format!(
            "\nmilestone m{i} \"M{i}\" {{\n{indent}status   planned\n{indent}features [f1]\n}}\n"
        ));
    }
    text
}

/// A project of the software and product extensions holding `text` as `a.spec`, with `fmt` as its
/// `.specforgefmt.toml` when given.
fn project(text: &str, fmt: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name": "p", "version": "0.1.0", "extensions": ["@specforge/software", "@specforge/product"]}"#,
    )
    .unwrap();
    if let Some(fmt) = fmt {
        std::fs::write(dir.path().join(".specforgefmt.toml"), fmt).unwrap();
    }
    std::fs::write(dir.path().join("a.spec"), text).unwrap();
    specforge_installed::testing::install_configured(
        dir.path(),
        &specforge_project::builtins(),
        &specforge_wasm::testing::InProcessRuntime::new(),
    );
    dir
}

/// [`project`], migrated; the file after.
fn migrated(text: &str, fmt: Option<&str>) -> (tempfile::TempDir, String) {
    let dir = project(text, fmt);
    let root = specforge_common::ProjectRoot::of(dir.path()).unwrap();
    let outcome = run(
        &request(&root),
        Some(specforge_project::testing::component_runtime()),
    );
    assert!(outcome.ok() && outcome.migrated(), "{outcome:?}");
    let after = std::fs::read_to_string(dir.path().join("a.spec")).unwrap();
    (dir, after)
}

/// Whether `format` would change `text`, the file at `path`, with its project's configuration.
fn unformatted(path: &std::path::Path, text: &str) -> bool {
    format::document(Place::File(path), text, None, None, &Forms::none()).changed()
}

#[specforge_test(
    behavior = "hoist_file_defaults",
    verify = "a hoisted block and the blocks it shortened are laid out with the file's format configuration"
)]
#[specforge_test(
    invariant = "migration_canonical_form",
    verify = "every block a migration writes is unchanged by format"
)]
fn a_hoist_leaves_the_blocks_it_shortened_format_clean() {
    let (dir, after) = migrated(&milestones("  "), None);

    assert!(
        after.starts_with("defaults milestone {\n  features [f1]\n}\n"),
        "{after}"
    );
    assert!(
        after.contains("milestone m1 \"M1\" {\n  status planned\n}\n"),
        "{after}"
    );
    assert!(!unformatted(&dir.path().join("a.spec"), &after), "{after}");
}

#[specforge_test(
    behavior = "hoist_file_defaults",
    verify = "a hoisted block and the blocks it shortened are laid out with the file's format configuration"
)]
fn a_hoisted_defaults_block_takes_the_files_indent() {
    let (dir, after) = migrated(&milestones("    "), Some("indent_width = 4\n"));

    assert!(
        after.starts_with("defaults milestone {\n    features [f1]\n}\n"),
        "{after}"
    );
    assert!(
        after.contains("milestone m6 \"M6\" {\n    status planned\n}\n"),
        "{after}"
    );
    assert!(!unformatted(&dir.path().join("a.spec"), &after), "{after}");
}

#[cfg(unix)]
#[specforge_test(
    behavior = "load_format_config",
    verify = "a bounded migration lays out with the defaults when its bound refuses the configuration"
)]
fn a_bounded_migration_lays_out_with_the_defaults_when_the_configuration_links_outside() {
    let dir = project(&milestones("  "), None);
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("c.toml"), "indent_width = 4\n").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("c.toml"),
        dir.path().join(".specforgefmt.toml"),
    )
    .unwrap();
    let root = specforge_common::ProjectRoot::of(dir.path()).unwrap();
    let containment = specforge_ops::Containment::within(dir.path());

    let outcome = run(
        &specforge_ops::authoring::migrate::Request {
            caller: specforge_ops::testing::bounded(&containment),
            ..request(&root)
        },
        Some(specforge_project::testing::component_runtime()),
    );
    assert!(outcome.ok() && outcome.migrated(), "{outcome:?}");

    let after = std::fs::read_to_string(dir.path().join("a.spec")).unwrap();
    assert!(
        after.starts_with("defaults milestone {\n  features [f1]\n}\n"),
        "{after}"
    );
    assert!(!after.contains("    features"), "{after}");
}
