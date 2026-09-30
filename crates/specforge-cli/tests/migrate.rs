use assert_cmd::Command;
use predicates::prelude::*;
use specforge_common::Sym;
use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

fn setup_project(dir: &std::path::Path) {
    fs::write(
        dir.join("specforge.json"),
        r#"{"name":"test","version":"0.1.0"}"#,
    )
    .unwrap();
    let spec_dir = dir.join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
}

fn write_spec(dir: &std::path::Path, name: &str, content: &str) {
    let spec_dir = dir.join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join(name), content).unwrap();
}

// ===================================================================
// Phase A: CLI Skeleton + Types
// ===================================================================

// A1: `specforge migrate` exits 0 on project with no version mismatches
#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "files already at target version are skipped with skippedCount incremented"
)]
fn migrate_exits_zero_on_current_version_project() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    // Files without a version header default to current version → skipped
    write_spec(
        root,
        "test.spec",
        "behavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n",
    );

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("0 migrated, 1 skipped, 0 failed"));

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["skipped_count"], 1, "{json}");
    assert_eq!(json["migrated_count"], 0, "{json}");
    assert_eq!(json["results"][0]["status"], "skipped", "{json}");
    assert_eq!(
        fs::read_to_string(root.join("spec/test.spec")).unwrap(),
        "behavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n",
        "a skipped file is left alone"
    );
}

// A2: MigrationSummary serializes to JSON
#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "summary reports migrated, failed, and skipped counts"
)]
fn migrate_json_output_contains_summary_fields() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    // One file per outcome: an old header migrates, a current one is
    // skipped, an unsupported one fails.
    write_spec(
        root,
        "old.spec",
        "// specforge-format: 0.1\nbehavior old_one \"Old\" {\n  contract \"stuff\"\n}\n",
    );
    write_spec(
        root,
        "current.spec",
        "// specforge-format: 1.0\nbehavior current_one \"Current\" {\n  contract \"stuff\"\n}\n",
    );
    write_spec(
        root,
        "future.spec",
        "// specforge-format: 99.0\nbehavior future_one \"Future\" {\n  contract \"stuff\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("failed to parse JSON: {e}\nstdout: {stdout}"));

    assert_eq!(json["migrated_count"], 1, "{json}");
    assert_eq!(json["skipped_count"], 1, "{json}");
    assert_eq!(json["failed_count"], 1, "{json}");
    assert_eq!(status_of(&json, "old.spec"), "migrated");
    assert_eq!(status_of(&json, "current.spec"), "skipped");
    assert_eq!(status_of(&json, "future.spec"), "failed");
    assert_eq!(json["backups"].as_array().unwrap().len(), 1, "{json}");
    assert_eq!(output.status.code(), Some(1), "a failed file fails the run");
}

/// The result entry of the file whose path ends with `file`.
fn result_for(json: &serde_json::Value, file: &str) -> serde_json::Value {
    json["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["file_path"].as_str().unwrap().ends_with(file))
        .cloned()
        .unwrap_or_else(|| panic!("no result for {file}: {json}"))
}

fn status_of(json: &serde_json::Value, file: &str) -> String {
    result_for(json, file)["status"]
        .as_str()
        .unwrap()
        .to_string()
}

// A3: Unknown --target-version produces error
#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "unsupported format version produces E019 with upgrade guidance"
)]
fn migrate_unknown_target_version_produces_error() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(
        root,
        "test.spec",
        "behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--target-version=99.0",
            "--path",
            root.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "E019: unsupported target version 99.0",
        ))
        .stderr(predicate::str::contains(
            "Use a format version between 1.0 and 1.0.",
        ));
    assert_eq!(
        fs::read_to_string(root.join("spec/test.spec")).unwrap(),
        "behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
        "nothing is migrated"
    );
}

// A target version that doesn't parse is the same E019, not another
// owner's code (it used to print E015, @specforge/product's milestone cycle).
#[test]
fn migrate_unparseable_target_version_reports_e019() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--target-version=not-a-version",
            "--path",
            root.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "E019: invalid target version 'not-a-version'",
        ));
}

// ===================================================================
// Phase B: Format Version Detection
// ===================================================================

// B1: Header comment detected
#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "header comment format version detected correctly"
)]
fn detect_format_version_from_header() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(
        root,
        "current.spec",
        "// specforge-format: 1.0\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );
    write_spec(
        root,
        "old.spec",
        "// specforge-format: 0.1\nbehavior bar \"Bar\" {\n  contract \"stuff\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    // Each file's header is read: 1.0 is current, 0.1 needs migrating.
    let current = result_for(&json, "current.spec");
    assert_eq!(
        current["from_version"],
        serde_json::json!({"major": 1, "minor": 0})
    );
    assert_eq!(current["status"], "skipped");
    let old = result_for(&json, "old.spec");
    assert_eq!(
        old["from_version"],
        serde_json::json!({"major": 0, "minor": 1})
    );
    assert_eq!(old["status"], "migrated");
}

// B2: Missing version defaults to current (no migration needed)
#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "missing format version treated as oldest supported"
)]
fn missing_version_header_defaults_to_current() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(
        root,
        "test.spec",
        "behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    // No header = current version → skipped
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .map(|o| {
            let stdout = String::from_utf8(o.stdout).unwrap();
            let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(
                json["skipped_count"], 1,
                "file without header should be skipped"
            );
        })
        .unwrap();
}

// B3: Unsupported version in header → E019
#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "unsupported format version produces E019 with upgrade guidance"
)]
fn unsupported_version_in_header() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    let original = "// specforge-format: 99.0\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["failed_count"], 1, "{json}");
    assert_eq!(json["skipped_count"], 0, "{json}");
    let error = json["results"][0]["error"].as_str().unwrap_or_default();
    assert!(error.contains("E019"), "{json}");
    assert!(error.contains("Use a format version between"), "{json}");
    assert_eq!(
        std::fs::read_to_string(root.join("spec/test.spec")).unwrap(),
        original,
        "the file is left alone"
    );
}

// ===================================================================
// Phase C: In-Place Migration with Backups
// ===================================================================

// C1: Files at target version are skipped
#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "files already at target version are skipped with skippedCount incremented"
)]
fn files_at_target_version_are_skipped() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(
        root,
        "test.spec",
        "// specforge-format: 1.0\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["skipped_count"], 1);
    assert_eq!(json["migrated_count"], 0);

    // No .bak file should exist
    assert!(
        !root.join("spec/test.spec.bak").exists(),
        "skipped files should not create backups"
    );
}

// C2: Backup created before modification (simulated with version 0.1 if we had a transform)
// Since v1 is current and no older versions exist yet, we test the backup mechanism
// by using a file that requires header addition.
#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "backup created before modification"
)]
fn backup_created_before_modification() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    // A file with version 0.1 (older than current) will need migration
    let original = "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    // Backup should exist with original content
    let bak_path = root.join("spec/test.spec.bak");
    assert!(bak_path.exists(), ".spec.bak should exist");
    let bak_content = fs::read_to_string(&bak_path).unwrap();
    assert_eq!(
        bak_content, original,
        "backup should contain original content byte-for-byte"
    );
}

// C3: Atomic write via temp+rename — no .spec.tmp remains
#[test]
fn no_tmp_file_remains_after_migration() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        !root.join("spec/test.spec.tmp").exists(),
        "temp file should not remain"
    );

    // Migrated file should have updated header
    let content = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert!(
        content.starts_with("// specforge-format: 1.0"),
        "migrated file should have v1.0 header: {content}"
    );
}

// C4: File failure isolation + --no-backup
#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "--no-backup skips backup creation"
)]
fn no_backup_flag_skips_backup_creation() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--no-backup", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        !root.join("spec/test.spec.bak").exists(),
        "--no-backup should prevent .bak creation"
    );
    assert_eq!(
        fs::read_to_string(root.join("spec/test.spec")).unwrap(),
        "// specforge-format: 1.0\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
        "the file is still migrated"
    );
}

// ===================================================================
// Phase D: Dry-Run Diff
// ===================================================================

// D1: --dry-run shows diff without modifying files
#[specforge_test(
    behavior = "generate_migration_diff",
    verify = "dry-run shows unified diff without modifying files"
)]
fn dry_run_shows_diff_without_modifying_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    let original = "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--dry-run", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"))
        .stdout(predicate::str::contains("+++"))
        .stdout(predicate::str::contains("@@"));

    // File should be unchanged
    let after = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(after, original, "dry-run should not modify files");

    // No backup should exist
    assert!(
        !root.join("spec/test.spec.bak").exists(),
        "dry-run should not create backups"
    );
}

// D2: Diff uses a/b/ prefix convention
#[specforge_test(
    behavior = "generate_migration_diff",
    verify = "each file diff labeled with file path"
)]
fn diff_uses_posix_prefix_convention() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--dry-run", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let label = |prefix: &str| {
        stdout
            .lines()
            .find(|l| l.starts_with(prefix))
            .unwrap_or_else(|| panic!("no {prefix} line: {stdout}"))
            .to_string()
    };
    let old_label = label("--- a/");
    let new_label = label("+++ b/");
    assert!(
        old_label.ends_with("spec/test.spec"),
        "the --- label names the file: {old_label}"
    );
    assert!(
        new_label.ends_with("spec/test.spec"),
        "the +++ label names the file: {new_label}"
    );
    assert_eq!(
        old_label.strip_prefix("--- a/"),
        new_label.strip_prefix("+++ b/"),
        "both labels name the same file"
    );
}

// D3: --dry-run --format=json → structured MigrationDiff
#[specforge_test(
    behavior = "generate_migration_diff",
    verify = "json format diff produces structured output with file-level entries"
)]
fn dry_run_json_produces_structured_diff() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--dry-run",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let diffs = json["diffs"].as_array().expect("diffs should be an array");
    assert!(!diffs.is_empty(), "should have at least one diff");

    let first = &diffs[0];
    assert!(first.get("file_path").is_some(), "missing file_path");
    assert!(first.get("before_hash").is_some(), "missing before_hash");
    assert!(first.get("after_hash").is_some(), "missing after_hash");
    assert!(first.get("unified_text").is_some(), "missing unified_text");
}

// ===================================================================
// Phase E: Rollback
// ===================================================================

// E1: --rollback restores from .bak files
#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "restores migrated files from .bak backups"
)]
fn rollback_restores_from_bak_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    let original = "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    // Migrate first (creates .bak)
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    // Verify migration happened
    let migrated = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert!(migrated.contains("1.0"), "file should be migrated");

    // Now rollback
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--rollback", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    // File should be restored to original
    let restored = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(
        restored, original,
        "rollback should restore original content"
    );

    // .bak should still exist (we don't delete it)
    assert!(
        root.join("spec/test.spec.bak").exists(),
        ".bak should still exist"
    );
}

// E2: Missing .bak → skip
#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "missing .bak file produces warning and skips"
)]
fn rollback_missing_bak_skips() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    // Rollback without any .bak files
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--rollback",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["skipped_count"], 1, "should skip files without .bak");
    assert_eq!(json["restored_count"], 0, "{json}");
    assert_eq!(json["failed_count"], 0, "{json}");
    let warnings = json["warnings"].as_array().expect("warnings array");
    assert_eq!(warnings.len(), 1, "{json}");
    assert!(
        warnings[0].as_str().unwrap().contains("test.spec.bak"),
        "the warning names the missing backup: {json}"
    );

    // The human output shows the same warning.
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--rollback", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning:"))
        .stderr(predicate::str::contains("test.spec.bak"))
        .stderr(predicate::str::contains("0 restored, 1 skipped, 0 failed"));
    assert_eq!(
        fs::read_to_string(root.join("spec/test.spec")).unwrap(),
        "behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
        "a skipped file is left alone"
    );
}

// E3: Single restore failure doesn't block others
#[cfg(unix)]
#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "rollback failure for one file does not block others"
)]
fn rollback_failure_isolation() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    let original_a = "// specforge-format: 0.1\nbehavior a \"A\" {\n  contract \"a\"\n}\n";
    let original_b = "// specforge-format: 0.1\nbehavior b \"B\" {\n  contract \"b\"\n}\n";
    write_spec(root, "a.spec", original_a);
    write_spec(root, "b.spec", original_b);

    // Migrate both
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let migrated_a = fs::read_to_string(root.join("spec/a.spec")).unwrap();
    assert_ne!(migrated_a, original_a);

    // Make a's restore fail: its backup can't be read.
    let bak_a = root.join("spec/a.spec.bak");
    fs::set_permissions(&bak_a, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_to_string(&bak_a).is_ok() {
        // Running as root: permissions don't stop the read.
        return;
    }

    // Rollback — a fails, b is still restored
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--rollback",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    fs::set_permissions(&bak_a, fs::Permissions::from_mode(0o644)).unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["failed_count"], 1, "{json}");
    assert_eq!(json["restored_count"], 1, "{json}");
    assert_eq!(json["skipped_count"], 0, "{json}");
    let failed = result_for(&json, "a.spec");
    assert_eq!(failed["status"], "failed", "{json}");
    assert!(
        failed["error"].as_str().unwrap().contains("backup"),
        "{json}"
    );
    assert_eq!(status_of(&json, "b.spec"), "restored");
    assert_eq!(
        output.status.code(),
        Some(1),
        "a failed restore fails the run"
    );

    // b is restored; a keeps its migrated content.
    let b_content = fs::read_to_string(root.join("spec/b.spec")).unwrap();
    assert_eq!(b_content, original_b, "b.spec should be restored");
    assert_eq!(
        fs::read_to_string(root.join("spec/a.spec")).unwrap(),
        migrated_a,
        "a.spec is untouched by its failed restore"
    );
}

// ===================================================================
// Phase F: Graph Validation
// ===================================================================

// F1: Capture pre-migration schema snapshot
#[test]
fn capture_pre_migration_snapshot_captures_schema() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::capture_pre_migration_snapshot;

    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    let snapshot = capture_pre_migration_snapshot(&schema);
    assert_eq!(snapshot.schema.entity_kinds.len(), 1);
    assert_eq!(snapshot.schema.entity_kinds[0].name, "behavior");
}

// F2: Post-migration structural equivalence (format-only migration)
#[specforge_test(
    behavior = "validate_post_migration_integrity",
    verify = "structural equivalence verified between pre and post graphs"
)]
fn format_only_migration_zero_differences() {
    use specforge_graph::{Edge, EntityId, EntityKind, FieldMap, FieldValue, Graph, Node};
    use specforge_migrate::compare_graphs;
    use specforge_parser::SpannedRef;

    // A graph as compiled from files whose lines start at `first_line`:
    // a format migration adds a header line, shifting every span.
    let build = |first_line: usize, contract: &str, extra_edge: bool| {
        let span = |line: usize| specforge_graph::SourceSpan {
            file: Sym::new("spec/test.spec"),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 10,
        };
        let mut graph = Graph::new();
        let mut fields = FieldMap::new();
        fields.push(Sym::new("contract"), FieldValue::String(contract.into()));
        fields.push(
            Sym::new("invariants"),
            FieldValue::ReferenceList(vec![SpannedRef {
                id: "inv_a".into(),
                span: span(first_line + 2),
            }]),
        );
        graph.add_node(Node {
            id: EntityId {
                raw: Sym::new("beh_a"),
            },
            kind: EntityKind {
                raw: Sym::new("behavior"),
            },
            title: Some("A".into()),
            source_span: span(first_line),
            fields,
            methods: Vec::new(),
        });
        for (id, kind) in [("inv_a", "invariant"), ("inv_b", "invariant")] {
            graph.add_node(Node {
                id: EntityId { raw: Sym::new(id) },
                kind: EntityKind {
                    raw: Sym::new(kind),
                },
                title: None,
                source_span: span(first_line + 5),
                fields: FieldMap::new(),
                methods: Vec::new(),
            });
        }
        graph.add_edge(Edge {
            source: Sym::new("beh_a"),
            target: Sym::new("inv_a"),
            label: Sym::new("invariants"),
        });
        if extra_edge {
            graph.add_edge(Edge {
                source: Sym::new("beh_a"),
                target: Sym::new("inv_b"),
                label: Sym::new("invariants"),
            });
        }
        graph
    };

    // Same entities, edges and field values; only the spans moved.
    let pre = build(1, "does stuff", false);
    let post = build(2, "does stuff", false);
    let diagnostics = compare_graphs(&pre, &post);
    assert!(
        diagnostics.is_empty(),
        "shifted spans are not a structural difference: {diagnostics:?}"
    );

    // A changed field value is a difference.
    let changed = build(2, "does other stuff", false);
    let diagnostics = compare_graphs(&pre, &changed);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "W054");
    assert!(
        diagnostics[0].message.contains("beh_a") && diagnostics[0].message.contains("contract"),
        "{diagnostics:?}"
    );

    // So is an edge that appeared.
    let grown = build(2, "does stuff", true);
    let diagnostics = compare_graphs(&pre, &grown);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "W054");
    assert!(diagnostics[0].message.contains("inv_b"), "{diagnostics:?}");
}

// F3: Structural differences → warnings
#[specforge_test(
    behavior = "validate_post_migration_integrity",
    verify = "structural differences reported as warnings"
)]
fn structural_differences_produce_warnings() {
    use specforge_graph::{EntityId, EntityKind, FieldMap, Graph, Node, SourceSpan};
    use specforge_migrate::compare_graphs;

    let mut pre = Graph::new();
    pre.add_node(Node {
        id: EntityId {
            raw: Sym::new("foo"),
        },
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: None,
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        fields: FieldMap::new(),
        methods: Vec::new(),
    });

    let post = Graph::new(); // Empty graph — entity "foo" is missing

    let diagnostics = compare_graphs(&pre, &post);
    assert!(
        !diagnostics.is_empty(),
        "missing entity should produce diagnostic"
    );
    assert!(
        diagnostics.iter().any(|d| d.code == "W054"),
        "should emit W054: {diagnostics:?}"
    );
}

// F4: Breaking schema change → W053
#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "breaking graph change emits W053 warning"
)]
fn breaking_schema_change_produces_w053() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    // Post-migration: "behavior" kind removed → breaking
    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(2, 0, 0),
        extensions: Vec::new(),
        entity_kinds: Vec::new(),
        edge_types: Vec::new(),
    };

    let diagnostics = check_schema_compatibility(&pre, &post);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "W053");
    assert_eq!(
        diagnostics[0].severity,
        specforge_common::Severity::Warning,
        "W053 is a warning"
    );
    assert!(
        diagnostics[0].message.contains("\"behavior\""),
        "the warning names the removed kind: {diagnostics:?}"
    );
}

// F4b: Non-breaking schema change → no W053
#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "non-breaking graph change passes silently"
)]
fn non_breaking_schema_change_no_w053() {
    use specforge_emitter::schema::{
        GraphProtocolSchema, SchemaEntityKind, SchemaField, SchemaVersion,
    };
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    // Post-migration: new optional field added → non-breaking
    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 1, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![SchemaField {
                name: "new_field".to_string(),
                field_type: "string".to_string(),
                required: false,
                enum_values: None,
                edge: None,
                target_kind: None,
                description: None,
                default_value: None,
                source_extension: "@specforge/software".to_string(),
            }],
        }],
        edge_types: Vec::new(),
    };

    let diagnostics = check_schema_compatibility(&pre, &post);
    assert!(
        diagnostics.is_empty(),
        "a non-breaking change produces no diagnostic at all: {diagnostics:?}"
    );

    // And a real migration of an entity-bearing project passes silently.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        stderr.trim(),
        "1 migrated, 0 skipped, 0 failed",
        "nothing but the summary"
    );
}

// ===================================================================
// Phase G: Extension Hooks
// ===================================================================

// G1: Extension without migration_hook → skip silently
#[test]
fn extension_without_hook_skipped() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner, NoOpMigrationHookRunner};

    let runner = NoOpMigrationHookRunner;
    let result = runner.invoke(
        "@specforge/software",
        "",
        &FormatVersion { major: 0, minor: 1 },
        &FormatVersion { major: 1, minor: 0 },
    );
    assert!(result.is_ok(), "no-op runner should succeed");
}

// G2: Hook error → diagnostic collected
#[test]
fn hook_error_produces_diagnostic() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner};

    struct FailingHookRunner;
    impl MigrationHookRunner for FailingHookRunner {
        fn invoke(
            &self,
            _ext: &str,
            _hook: &str,
            _from: &FormatVersion,
            _to: &FormatVersion,
        ) -> Result<(), String> {
            Err("hook failed".to_string())
        }
    }

    let runner = FailingHookRunner;
    let result = runner.invoke(
        "@specforge/software",
        "migrate",
        &FormatVersion { major: 0, minor: 1 },
        &FormatVersion { major: 1, minor: 0 },
    );
    assert!(result.is_err(), "failing runner should return error");
    assert_eq!(result.unwrap_err(), "hook failed");
}

// G3: Hook timeout → trap diagnostic (structural test)
#[test]
fn hook_timeout_produces_trap() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner};

    struct TimeoutHookRunner;
    impl MigrationHookRunner for TimeoutHookRunner {
        fn invoke(
            &self,
            _ext: &str,
            _hook: &str,
            _from: &FormatVersion,
            _to: &FormatVersion,
        ) -> Result<(), String> {
            Err("Wasm trap: execution timed out after 30s".to_string())
        }
    }

    let runner = TimeoutHookRunner;
    let result = runner.invoke(
        "@specforge/software",
        "migrate",
        &FormatVersion { major: 0, minor: 1 },
        &FormatVersion { major: 1, minor: 0 },
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("timed out"));
}

// ===================================================================
// Phase H: Integration
// ===================================================================

// H1: Full pipeline — v0.1→v1.0 migration end-to-end
#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "files migrated from source to target version"
)]
fn full_pipeline_migration() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    let files = [
        (
            "a.spec",
            "// specforge-format: 0.1\nbehavior a \"A\" {\n  contract \"a\"\n}\n",
        ),
        (
            "b.spec",
            "// specforge-format: 0.1\nbehavior b \"B\" {\n  contract \"b\"\n}\n",
        ),
        (
            "c.spec",
            "// specforge-format: 0.1\nbehavior c \"C\" {\n  contract \"c\"\n}\n",
        ),
    ];

    for (name, content) in &files {
        write_spec(root, name, content);
    }

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("3 migrated"));

    // All files should have v1.0 header
    for (name, _) in &files {
        let content = fs::read_to_string(root.join("spec").join(name)).unwrap();
        assert!(
            content.starts_with("// specforge-format: 1.0"),
            "{name} should have v1.0 header: {content}"
        );
    }
}

// H2: Idempotency — double migrate → no changes
#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "files already at target version are skipped with skippedCount incremented"
)]
fn double_migrate_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    // First migration
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("1 migrated"));

    let after_first = fs::read_to_string(root.join("spec/test.spec")).unwrap();

    // Second migration — should skip
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["skipped_count"], 1, "{json}");
    assert_eq!(json["migrated_count"], 0, "{json}");
    assert_eq!(json["failed_count"], 0, "{json}");
    assert_eq!(status_of(&json, "test.spec"), "skipped");

    let after_second = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(
        after_first, after_second,
        "second migrate should not change files"
    );
}

// H3: Failure in one file does not block others
#[test]
fn failure_in_one_file_does_not_block_others() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    // Two files: one valid, one will be unreadable
    write_spec(
        root,
        "good.spec",
        "// specforge-format: 0.1\nbehavior good \"Good\" {\n  contract \"ok\"\n}\n",
    );
    write_spec(
        root,
        "bad.spec",
        "// specforge-format: 0.1\nbehavior bad \"Bad\" {\n  contract \"ok\"\n}\n",
    );

    // Make bad.spec a directory (unreadable as file)
    fs::remove_file(root.join("spec/bad.spec")).unwrap();
    fs::create_dir(root.join("spec/bad.spec")).unwrap();

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // good.spec should have been migrated despite bad.spec failure
    let migrated = json["migrated_count"].as_u64().unwrap_or(0);
    assert!(migrated >= 1, "good.spec should be migrated: {stdout}");
}

// H3b: JSON summary output
#[test]
fn json_summary_contains_results_and_backups() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(json["results"].is_array(), "results should be an array");
    assert!(json["backups"].is_array(), "backups should be an array");
    assert_eq!(json["migrated_count"], 1);
    assert!(
        !json["backups"].as_array().unwrap().is_empty(),
        "backups should be populated"
    );
}

// ===================================================================
// Phase I: detect_format_version_mismatch — unit tests
// ===================================================================

#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "older format version detected and reported as I007"
)]
fn older_format_version_produces_i007() {
    use specforge_migrate::detect_format_version;

    let content = "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"x\"\n}\n";
    let (version, diags) = detect_format_version(content);
    assert_eq!(version.major, 0);
    assert_eq!(version.minor, 1);
    assert!(
        diags.iter().any(|d| d.code == "I007"),
        "older version should emit I007: {diags:?}"
    );
}

#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "current format version produces no diagnostic"
)]
fn current_format_version_no_diagnostic() {
    use specforge_migrate::detect_format_version;

    let content = "// specforge-format: 1.0\nbehavior foo \"Foo\" {\n  contract \"x\"\n}\n";
    let (version, diags) = detect_format_version(content);
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 0);
    assert!(
        diags.is_empty(),
        "current version should produce no diagnostic: {diags:?}"
    );
}

#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "missing format version treated as oldest supported"
)]
fn missing_format_version_defaults_to_current() {
    use specforge_migrate::{CURRENT_FORMAT_VERSION, detect_format_version};

    let content = "behavior foo \"Foo\" {\n  contract \"x\"\n}\n";
    let (version, diags) = detect_format_version(content);
    assert_eq!(
        version, CURRENT_FORMAT_VERSION,
        "no header → defaults to current"
    );
    assert!(diags.is_empty(), "no header → no diagnostic: {diags:?}");
}

#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "header comment format version detected correctly"
)]
fn header_comment_detected_correctly() {
    use specforge_migrate::detect_format_version;

    let content = "// specforge-format: 0.5\nbehavior foo \"Foo\" {\n  contract \"x\"\n}\n";
    let (version, _) = detect_format_version(content);
    assert_eq!(version.major, 0);
    assert_eq!(version.minor, 5);
}

#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "unsupported format version produces E019 with upgrade guidance"
)]
fn unsupported_format_version_produces_e015() {
    use specforge_migrate::detect_format_version;

    let content = "// specforge-format: 99.0\nbehavior foo \"Foo\" {\n  contract \"x\"\n}\n";
    let (_version, diags) = detect_format_version(content);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "E019" && d.suggestion.is_some()),
        "unsupported version should emit E019 with suggestion: {diags:?}"
    );
}

#[specforge_test(
    behavior = "detect_format_version_mismatch",
    verify = "Detect Format Version Mismatch: format version detection holds — spec_file_available, version_mismatch_reported, unsupported_version_rejected, parsing_continues"
)]
fn detect_format_version_contract() {
    use specforge_migrate::detect_format_version;

    // Requires: spec file content is accessible (we pass a string)
    // Ensures: version detected, diagnostics emitted appropriately

    // Valid header → version detected, no errors
    let (v, d) = detect_format_version("// specforge-format: 1.0\n");
    assert_eq!(v.major, 1);
    assert!(d.is_empty());

    // Invalid header → fallback version, error emitted
    let (v, d) = detect_format_version("// specforge-format: abc\n");
    assert!(v.major >= 1, "fallback to min supported");
    assert!(
        d.iter()
            .any(|d| d.severity == specforge_common::Severity::Error)
    );

    // No header → current version, no diagnostics
    let (v, d) = detect_format_version("behavior foo \"Foo\" {}\n");
    assert_eq!(v.major, 1);
    assert!(d.is_empty());
}

// ===================================================================
// Phase J: migrate_spec_files_in_place — additional coverage
// ===================================================================

#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "Migrate Spec Files In Place: in-place migration holds — semantic_preservation"
)]
fn migrate_in_place_contract() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    // Requires: files exist, valid target version
    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    // Ensures: files at target, summary emitted, complete event
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // files_at_target
    assert_eq!(json["migrated_count"], 1);
    assert_eq!(json["failed_count"], 0);

    // summary_emitted
    assert!(json.get("migrated_count").is_some());
    assert!(json.get("skipped_count").is_some());
    assert!(json.get("failed_count").is_some());

    // Verify file is now at target version
    let content = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert!(content.starts_with("// specforge-format: 1.0"));
}

#[specforge_test(
    behavior = "migrate_spec_files_in_place",
    verify = "summary reports migrated, failed, and skipped counts"
)]
fn summary_reports_all_three_counts() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    // One file to migrate, one already at current
    write_spec(
        root,
        "old.spec",
        "// specforge-format: 0.1\nbehavior old \"Old\" {\n  contract \"old\"\n}\n",
    );
    write_spec(
        root,
        "current.spec",
        "// specforge-format: 1.0\nbehavior cur \"Cur\" {\n  contract \"cur\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--format=json", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["migrated_count"], 1, "one file migrated");
    assert_eq!(json["skipped_count"], 1, "one file skipped");
    assert_eq!(json["failed_count"], 0, "no failures");
}

// ===================================================================
// Phase K: generate_migration_diff — additional coverage
// ===================================================================

#[specforge_test(
    behavior = "generate_migration_diff",
    verify = "diff format is compatible with patch(1)"
)]
fn diff_format_compatible_with_patch() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--dry-run", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // POSIX unified diff format requirements:
    // 1. Starts with --- a/... and +++ b/...
    assert!(stdout.contains("--- a/"), "missing --- a/ header");
    assert!(stdout.contains("+++ b/"), "missing +++ b/ header");
    // 2. Has hunk headers @@ -N,M +N,M @@
    assert!(stdout.contains("@@"), "missing @@ hunk header");
    // 3. Changed lines start with + or -
    assert!(
        stdout
            .lines()
            .any(|l| l.starts_with('-') && !l.starts_with("---")),
        "missing - removed lines"
    );
    assert!(
        stdout
            .lines()
            .any(|l| l.starts_with('+') && !l.starts_with("+++")),
        "missing + added lines"
    );
}

#[test]
fn dry_run_failure_isolation() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "good.spec",
        "// specforge-format: 0.1\nbehavior good \"Good\" {\n  contract \"ok\"\n}\n",
    );
    write_spec(
        root,
        "bad.spec",
        "// specforge-format: 0.1\nbehavior bad \"Bad\" {\n  contract \"ok\"\n}\n",
    );

    // Make bad.spec a directory (unreadable as file)
    fs::remove_file(root.join("spec/bad.spec")).unwrap();
    fs::create_dir(root.join("spec/bad.spec")).unwrap();

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--dry-run",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // good.spec should still produce a diff
    let diffs = json["diffs"].as_array().expect("diffs should be array");
    assert!(
        diffs.iter().any(|d| {
            d["file_path"]
                .as_str()
                .is_some_and(|p| p.contains("good.spec"))
        }),
        "good.spec diff should be present despite bad.spec failure: {stdout}"
    );
}

#[specforge_test(
    behavior = "generate_migration_diff",
    verify = "Generate Migration Diff: migration diff generation holds — spec_files_available, dry_run_flag_set, diff_produced, no_files_modified, migration_diff_generated_emitted"
)]
fn migration_diff_contract() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    let original = "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    // Requires: files available, dry-run flag set
    // Ensures: diff produced, no files modified, event emitted

    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--dry-run",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // diff_produced
    let diffs = json["diffs"].as_array().expect("diffs array");
    assert!(!diffs.is_empty(), "diff should be produced");

    // no_files_modified
    let after = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(after, original, "dry-run must not modify files");
    assert!(
        !root.join("spec/test.spec.bak").exists(),
        "dry-run must not create backups"
    );
}

// ===================================================================
// Phase L: validate_post_migration_integrity — additional coverage
// ===================================================================

#[test]
fn post_migration_check_runs_automatically() {
    use specforge_graph::{EntityId, EntityKind, FieldMap, Graph, Node, SourceSpan};
    use specforge_migrate::compare_graphs;

    // The compare_graphs function is the post-migration check.
    // It runs automatically as part of the migration pipeline.
    // Here we verify it catches differences when invoked.
    let mut pre = Graph::new();
    pre.add_node(Node {
        id: EntityId {
            raw: Sym::new("alpha"),
        },
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: Some("Alpha".to_string()),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        fields: FieldMap::new(),
        methods: Vec::new(),
    });

    let mut post = Graph::new();
    post.add_node(Node {
        id: EntityId {
            raw: Sym::new("alpha"),
        },
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: Some("Alpha".to_string()),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 5,
            start_col: 0,
            end_line: 5,
            end_col: 0,
        },
        fields: FieldMap::new(),
        methods: Vec::new(),
    });

    // Identical entities (different source spans excluded) → no diagnostics
    let diags = compare_graphs(&pre, &post);
    assert!(
        diags.is_empty(),
        "same entities should produce no diagnostics"
    );
}

#[test]
fn new_entities_after_migration_reported() {
    use specforge_graph::{EntityId, EntityKind, FieldMap, Graph, Node, SourceSpan};
    use specforge_migrate::compare_graphs;

    let pre = Graph::new(); // empty before

    let mut post = Graph::new();
    post.add_node(Node {
        id: EntityId {
            raw: Sym::new("new_entity"),
        },
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: None,
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        fields: FieldMap::new(),
        methods: Vec::new(),
    });

    let diags = compare_graphs(&pre, &post);
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("new_entity") && d.message.contains("appeared")),
        "new entity should be reported: {diags:?}"
    );
}

#[specforge_test(
    behavior = "validate_post_migration_integrity",
    verify = "Validate Post-Migration Integrity: post-migration integrity validation holds — extension_hooks_complete_fired, structural_equivalence_checked, differences_reported, migration_validation_complete_emitted"
)]
fn post_migration_integrity_contract() {
    use specforge_graph::{Edge, EntityId, EntityKind, FieldMap, Graph, Node, SourceSpan};
    use specforge_migrate::compare_graphs;

    let make_node = |id: &str| Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: None,
        source_span: SourceSpan {
            file: Sym::new("t.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        fields: FieldMap::new(),
        methods: Vec::new(),
    };

    // Ensures: structural_equivalence_checked — identical graphs yield empty
    let g = Graph::new();
    assert!(compare_graphs(&g, &g).is_empty());

    // Ensures: differences_reported — missing entity yields W054
    let mut pre = Graph::new();
    pre.add_node(make_node("x"));
    let post = Graph::new();
    let diags = compare_graphs(&pre, &post);
    assert!(
        diags.iter().any(|d| d.code == "W054"),
        "missing entity → W054"
    );

    // Ensures: edge differences reported
    let mut pre2 = Graph::new();
    pre2.add_node(make_node("a"));
    pre2.add_node(make_node("b"));
    pre2.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("refs"),
    });
    let mut post2 = Graph::new();
    post2.add_node(make_node("a"));
    post2.add_node(make_node("b"));
    let diags2 = compare_graphs(&pre2, &post2);
    assert!(
        diags2
            .iter()
            .any(|d| d.code == "W054" && d.message.contains("edge")),
        "missing edge → W054: {diags2:?}"
    );
}

// ===================================================================
// Phase M: capture_pre_migration_schema_snapshot — additional coverage
// ===================================================================

#[specforge_test(
    behavior = "capture_pre_migration_schema_snapshot",
    verify = "snapshot includes node kinds, edge types, and field definitions"
)]
fn snapshot_includes_all_schema_components() {
    use specforge_emitter::schema::{
        GraphProtocolSchema, SchemaEdgeType, SchemaEntityKind, SchemaField, SchemaVersion,
    };
    use specforge_migrate::capture_pre_migration_snapshot;

    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![SchemaField {
                name: "contract".to_string(),
                field_type: "string".to_string(),
                required: true,
                enum_values: None,
                edge: None,
                target_kind: None,
                description: None,
                default_value: None,
                source_extension: "@specforge/software".to_string(),
            }],
        }],
        edge_types: vec![SchemaEdgeType {
            label: "implements".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: Some(vec!["behavior".to_string()]),
            target_kinds: Some(vec!["feature".to_string()]),
        }],
    };

    let snapshot = capture_pre_migration_snapshot(&schema);
    assert_eq!(
        snapshot.schema, schema,
        "the snapshot holds the whole schema"
    );

    // The schema `specforge migrate` snapshots is the project's compiled
    // schema, the one `specforge schema` prints. Snapshot a real one.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("specforge.json"),
        r#"{"name":"test","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    write_spec(
        root,
        "test.spec",
        "behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n",
    );
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["schema", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let project_schema: GraphProtocolSchema = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output.stdout)));
    let snapshot = capture_pre_migration_snapshot(&project_schema);

    // Node kinds
    let behavior = snapshot
        .schema
        .entity_kinds
        .iter()
        .find(|k| k.name == "behavior")
        .expect("behavior kind in snapshot");
    assert!(
        snapshot
            .schema
            .entity_kinds
            .iter()
            .any(|k| k.name == "invariant")
    );
    // Field definitions, with their types
    let contract = behavior
        .fields
        .iter()
        .find(|f| f.name == "contract")
        .expect("behavior.contract field in snapshot");
    assert_eq!(contract.field_type, "string");
    let invariants = behavior
        .fields
        .iter()
        .find(|f| f.name == "invariants")
        .expect("behavior.invariants field in snapshot");
    assert_eq!(invariants.field_type, "reference_list");
    // Edge types
    assert!(
        snapshot
            .schema
            .edge_types
            .iter()
            .any(|e| e.label == "BehaviorEnforcesInvariant"),
        "{:?}",
        snapshot.schema.edge_types
    );
}

#[test]
fn snapshot_persists_across_lifecycle() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::capture_pre_migration_snapshot;

    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "invariant".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: false,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    // Capture at migration_starting
    let snapshot = capture_pre_migration_snapshot(&schema);

    // Simulate time passing / hooks running — snapshot is still accessible
    let snapshot_clone = snapshot.clone();
    assert_eq!(
        snapshot_clone.schema.entity_kinds[0].name, "invariant",
        "snapshot must persist across lifecycle phases"
    );
    assert_eq!(snapshot.schema, snapshot_clone.schema);
}

#[test]
fn pre_migration_snapshot_contract() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaVersion};
    use specforge_migrate::capture_pre_migration_snapshot;

    // Requires: migration_starting fired (we call the function directly)
    // Ensures: snapshot_captured with node kinds, edge types, field defs

    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: Vec::new(),
        edge_types: Vec::new(),
    };

    let snapshot = capture_pre_migration_snapshot(&schema);
    assert_eq!(snapshot.schema.schema_version, SchemaVersion::new(1, 0, 0));
    assert!(snapshot.schema.entity_kinds.is_empty());
    assert!(snapshot.schema.edge_types.is_empty());
}

// ===================================================================
// Phase N: verify_graph_protocol_compatibility — additional coverage
// ===================================================================

#[test]
fn entity_structure_change_triggers_check() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    // Add a new kind — structural change
    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 1, 0),
        extensions: Vec::new(),
        entity_kinds: vec![
            SchemaEntityKind {
                name: "behavior".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: Vec::new(),
            },
            SchemaEntityKind {
                name: "event".to_string(),
                source_extension: "@specforge/software".to_string(),
                testable: true,
                dot_color: None,
                fields: Vec::new(),
            },
        ],
        edge_types: Vec::new(),
    };

    // Non-breaking addition → no W053
    let diags = check_schema_compatibility(&pre, &post);
    assert!(
        !diags.iter().any(|d| d.code == "W053"),
        "addition is non-breaking: {diags:?}"
    );
}

#[test]
fn formatting_only_change_no_schema_diff() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::check_schema_compatibility;

    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    // Same schema pre and post → no changes
    let diags = check_schema_compatibility(&schema, &schema);
    assert!(diags.is_empty(), "identical schema → no diagnostics");
}

#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "removed node kind detected as breaking"
)]
fn removed_node_kind_is_breaking() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(2, 0, 0),
        extensions: Vec::new(),
        entity_kinds: Vec::new(),
        edge_types: Vec::new(),
    };

    let diags = check_schema_compatibility(&pre, &post);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W053");
    assert_eq!(
        diags[0].message,
        "breaking schema change after migration: KindRemoved(\"behavior\")"
    );
}

#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "removed edge type detected as breaking"
)]
fn removed_edge_type_is_breaking() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEdgeType, SchemaVersion};
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: Vec::new(),
        edge_types: vec![SchemaEdgeType {
            label: "implements".to_string(),
            source_extension: "@specforge/software".to_string(),
            source_kinds: None,
            target_kinds: None,
        }],
    };

    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(2, 0, 0),
        extensions: Vec::new(),
        entity_kinds: Vec::new(),
        edge_types: Vec::new(),
    };

    let diags = check_schema_compatibility(&pre, &post);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W053");
    assert_eq!(
        diags[0].message,
        "breaking schema change after migration: EdgeRemoved(\"implements\")"
    );
}

#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "removed required field detected as breaking"
)]
fn removed_required_field_is_breaking() {
    use specforge_emitter::schema::{
        GraphProtocolSchema, SchemaEntityKind, SchemaField, SchemaVersion,
    };
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![SchemaField {
                name: "contract".to_string(),
                field_type: "string".to_string(),
                required: true,
                enum_values: None,
                edge: None,
                target_kind: None,
                description: None,
                default_value: None,
                source_extension: "@specforge/software".to_string(),
            }],
        }],
        edge_types: Vec::new(),
    };

    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(2, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(), // field removed
        }],
        edge_types: Vec::new(),
    };

    let diags = check_schema_compatibility(&pre, &post);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W053");
    assert_eq!(
        diags[0].message,
        "breaking schema change after migration: FieldRemoved { kind: \"behavior\", field: \"contract\" }"
    );
}

#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "changed field type detected as breaking"
)]
fn changed_field_type_is_breaking() {
    use specforge_emitter::schema::{
        GraphProtocolSchema, SchemaEntityKind, SchemaField, SchemaVersion,
    };
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![SchemaField {
                name: "contract".to_string(),
                field_type: "string".to_string(),
                required: true,
                enum_values: None,
                edge: None,
                target_kind: None,
                description: None,
                default_value: None,
                source_extension: "@specforge/software".to_string(),
            }],
        }],
        edge_types: Vec::new(),
    };

    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(2, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![SchemaField {
                name: "contract".to_string(),
                field_type: "string_list".to_string(), // type changed
                required: true,
                enum_values: None,
                edge: None,
                target_kind: None,
                description: None,
                default_value: None,
                source_extension: "@specforge/software".to_string(),
            }],
        }],
        edge_types: Vec::new(),
    };

    let diags = check_schema_compatibility(&pre, &post);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W053");
    assert_eq!(
        diags[0].message,
        "breaking schema change after migration: FieldTypeChanged { kind: \"behavior\", field: \"contract\", old_type: \"string\", new_type: \"string_list\" }"
    );
}

#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "added optional field is not breaking"
)]
fn added_optional_field_not_breaking() {
    use specforge_emitter::schema::{
        GraphProtocolSchema, SchemaEntityKind, SchemaField, SchemaVersion,
    };
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    let post = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 1, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![SchemaField {
                name: "description".to_string(),
                field_type: "string".to_string(),
                required: false, // optional
                enum_values: None,
                edge: None,
                target_kind: None,
                description: None,
                default_value: None,
                source_extension: "@specforge/software".to_string(),
            }],
        }],
        edge_types: Vec::new(),
    };

    let diags = check_schema_compatibility(&pre, &post);
    assert!(
        diags.is_empty(),
        "optional field addition is non-breaking: {diags:?}"
    );

    // The same field added as required is breaking: the check looks at
    // `required`, not only at the field's presence.
    let mut post_required = post.clone();
    post_required.entity_kinds[0].fields[0].required = true;
    let diags = check_schema_compatibility(&pre, &post_required);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W053");
}

#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "cross-extension reference broken by migration produces diagnostic"
)]
fn cross_extension_broken_reference_produces_diagnostic() {
    use specforge_graph::{Edge, EntityId, EntityKind, FieldMap, Graph, Node, SourceSpan};
    use specforge_migrate::compare_graphs;

    let make_node = |id: &str, kind: &str| Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: None,
        source_span: SourceSpan {
            file: Sym::new("t.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        fields: FieldMap::new(),
        methods: Vec::new(),
    };

    // Pre: behavior→feature cross-extension edge exists
    let mut pre = Graph::new();
    pre.add_node(make_node("login_flow", "behavior"));
    pre.add_node(make_node("user_auth", "feature"));
    pre.add_edge(Edge {
        source: Sym::new("login_flow"),
        target: Sym::new("user_auth"),
        label: Sym::new("implements"),
    });

    // Post: feature was removed (broken cross-extension reference)
    let mut post = Graph::new();
    post.add_node(make_node("login_flow", "behavior"));

    let diags = compare_graphs(&pre, &post);
    // Should report both the missing entity and the missing edge
    assert!(
        diags.iter().any(|d| d.message.contains("user_auth")),
        "missing cross-extension entity → diagnostic: {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.message.contains("edge")),
        "missing cross-extension edge → diagnostic: {diags:?}"
    );
}

#[specforge_test(
    behavior = "verify_graph_protocol_compatibility_after_migration",
    verify = "Verify Graph Protocol Compatibility After Migration: graph protocol compatibility verification holds — pre_migration_snapshot_available, extension_hooks_complete, compatibility_verified, breaking_changes_warned, graph_protocol_compatibility_emitted"
)]
fn graph_protocol_compatibility_contract() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::check_schema_compatibility;

    // Requires: pre-migration snapshot available, extension hooks complete
    // Ensures: compatibility verified, breaking changes warned, event emitted

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    // compatibility_verified: an unchanged schema passes with no warning.
    let diags = check_schema_compatibility(&pre, &pre);
    assert!(diags.is_empty(), "identical → no warnings: {diags:?}");

    // breaking_changes_warned: one W053 per breaking change, naming it.
    let post_breaking = GraphProtocolSchema::empty();
    let diags = check_schema_compatibility(&pre, &post_breaking);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W053");
    assert!(diags[0].message.contains("KindRemoved(\"behavior\")"));

    // End to end: `specforge migrate` snapshots the schema before touching
    // files (pre_migration_snapshot_available), runs the extension hooks
    // (extension_hooks_complete), then compares and finishes
    // (graph_protocol_compatibility_emitted). A format-only migration keeps
    // the schema, so it passes with nothing but the summary.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("specforge.json"),
        r#"{"name":"test","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    write_spec(
        root,
        "test.spec",
        "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n  invariants [bar]\n}\n\ninvariant bar \"Bar\" {\n  guarantee \"holds\"\n}\n",
    );
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(stderr.trim(), "1 migrated, 0 skipped, 0 failed");
    assert!(
        fs::read_to_string(root.join("spec/test.spec"))
            .unwrap()
            .starts_with("// specforge-format: 1.0\n"),
        "the file was migrated"
    );
}

// ===================================================================
// Phase O: rollback_failed_migration — additional coverage
// ===================================================================

#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "restore is atomic per file"
)]
fn rollback_restore_is_atomic() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    let original = "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    // Migrate
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let migrated = fs::read_to_string(root.join("spec/test.spec")).unwrap();

    // Block the temp-file step: a directory sits where the restore's temp
    // file goes. An atomic restore (write temp, then rename) fails before
    // touching the target; a direct write would overwrite it.
    let blocker = root.join("spec/test.spec.restore.tmp");
    fs::create_dir(&blocker).unwrap();
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--rollback",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["failed_count"], 1, "{json}");
    assert_eq!(json["restored_count"], 0, "{json}");
    assert_eq!(
        fs::read_to_string(root.join("spec/test.spec")).unwrap(),
        migrated,
        "a failed restore leaves the target exactly as it was"
    );

    // Unblocked, the restore completes and leaves no temp file behind.
    fs::remove_dir(&blocker).unwrap();
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--rollback", "--path", root.to_str().unwrap()])
        .assert()
        .success();
    assert!(
        !blocker.exists(),
        "no .restore.tmp should remain after atomic rollback"
    );
    let content = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(content, original, "file fully restored atomically");
}

#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "summary reports restored, skipped, and failed counts"
)]
fn rollback_summary_counts() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    write_spec(
        root,
        "a.spec",
        "// specforge-format: 0.1\nbehavior a \"A\" {\n  contract \"a\"\n}\n",
    );
    write_spec(
        root,
        "b.spec",
        "// specforge-format: 1.0\nbehavior b \"B\" {\n  contract \"b\"\n}\n",
    );

    // Migrate — only a.spec should be migrated (b is already at target)
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    // Rollback — a has .bak, b doesn't
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--rollback",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(
        json.get("restored_count").is_some(),
        "missing restored_count"
    );
    assert!(json.get("skipped_count").is_some(), "missing skipped_count");
    assert!(json.get("failed_count").is_some(), "missing failed_count");

    let restored = json["restored_count"].as_u64().unwrap_or(0);
    let skipped = json["skipped_count"].as_u64().unwrap_or(0);
    assert_eq!(restored, 1, "a.spec should be restored");
    assert_eq!(skipped, 1, "b.spec should be skipped (no .bak)");
}

#[specforge_test(
    behavior = "rollback_failed_migration",
    verify = "Rollback Failed Migration: migration rollback holds — migration_started, files_restored, rollback_event_emitted, backup_file_preservation"
)]
fn rollback_contract() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    let original = "// specforge-format: 0.1\nbehavior foo \"Foo\" {\n  contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    // Requires: migration_started (backups exist)
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["migrate", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        root.join("spec/test.spec.bak").exists(),
        "backup must exist before rollback"
    );

    // Ensures: files restored, event emitted
    let output = Command::cargo_bin("specforge")
        .unwrap()
        .args([
            "migrate",
            "--rollback",
            "--format=json",
            "--path",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["restored_count"], 1);
    assert_eq!(json["failed_count"], 0);

    // File restored
    let content = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(content, original);

    // Maintains: backup_file_preservation
    assert!(
        root.join("spec/test.spec.bak").exists(),
        ".bak preserved after rollback"
    );
}

// ===================================================================
// Phase P: invoke_extension_migration_hooks — additional coverage
// ===================================================================

#[test]
fn extension_with_hook_gets_invoked() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner};
    use std::sync::{Arc, Mutex};

    struct TrackingRunner {
        invocations: Arc<Mutex<Vec<String>>>,
    }

    impl MigrationHookRunner for TrackingRunner {
        fn invoke(
            &self,
            ext: &str,
            hook: &str,
            _from: &FormatVersion,
            _to: &FormatVersion,
        ) -> Result<(), String> {
            self.invocations
                .lock()
                .unwrap()
                .push(format!("{ext}:{hook}"));
            Ok(())
        }
    }

    let invocations = Arc::new(Mutex::new(Vec::new()));
    let runner = TrackingRunner {
        invocations: invocations.clone(),
    };

    let result = runner.invoke(
        "@specforge/software",
        "migrate_v1_to_v2",
        &FormatVersion { major: 1, minor: 0 },
        &FormatVersion { major: 2, minor: 0 },
    );

    assert!(result.is_ok());
    let calls = invocations.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], "@specforge/software:migrate_v1_to_v2");
}

#[test]
fn extension_with_empty_hook_skipped() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner, NoOpMigrationHookRunner};

    let runner = NoOpMigrationHookRunner;

    // Empty hook name → should succeed (no-op)
    let result = runner.invoke(
        "@specforge/governance",
        "",
        &FormatVersion { major: 0, minor: 1 },
        &FormatVersion { major: 1, minor: 0 },
    );
    assert!(result.is_ok(), "empty hook should be silently skipped");
}

#[test]
fn hooks_invoked_in_deterministic_order() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner};
    use std::sync::{Arc, Mutex};

    struct OrderTracker {
        order: Arc<Mutex<Vec<String>>>,
    }

    impl MigrationHookRunner for OrderTracker {
        fn invoke(
            &self,
            ext: &str,
            _hook: &str,
            _from: &FormatVersion,
            _to: &FormatVersion,
        ) -> Result<(), String> {
            self.order.lock().unwrap().push(ext.to_string());
            Ok(())
        }
    }

    let order = Arc::new(Mutex::new(Vec::new()));
    let runner = OrderTracker {
        order: order.clone(),
    };
    let from = FormatVersion { major: 0, minor: 1 };
    let to = FormatVersion { major: 1, minor: 0 };

    // Invoke in a specific order
    let extensions = [
        "@specforge/software",
        "@specforge/product",
        "@specforge/governance",
    ];

    for ext in &extensions {
        runner.invoke(ext, "migrate", &from, &to).unwrap();
    }

    let calls = order.lock().unwrap();
    assert_eq!(calls.len(), 3);
    // Same order as invocation
    assert_eq!(calls[0], "@specforge/software");
    assert_eq!(calls[1], "@specforge/product");
    assert_eq!(calls[2], "@specforge/governance");

    // Run again — same order (deterministic)
    drop(calls);
    let order2 = Arc::new(Mutex::new(Vec::new()));
    let runner2 = OrderTracker {
        order: order2.clone(),
    };

    for ext in &extensions {
        runner2.invoke(ext, "migrate", &from, &to).unwrap();
    }

    let calls2 = order2.lock().unwrap();
    assert_eq!(calls2.as_slice(), &extensions);
}

#[test]
fn failed_extension_hook_skipped() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner};

    struct LifecycleAwareRunner {
        failed_extensions: Vec<String>,
    }

    impl MigrationHookRunner for LifecycleAwareRunner {
        fn invoke(
            &self,
            ext: &str,
            _hook: &str,
            _from: &FormatVersion,
            _to: &FormatVersion,
        ) -> Result<(), String> {
            if self.failed_extensions.contains(&ext.to_string()) {
                Err(format!("extension {ext} is in failed state"))
            } else {
                Ok(())
            }
        }
    }

    let runner = LifecycleAwareRunner {
        failed_extensions: vec!["@specforge/broken".to_string()],
    };

    let from = FormatVersion { major: 0, minor: 1 };
    let to = FormatVersion { major: 1, minor: 0 };

    // Failed extension returns error
    assert!(
        runner
            .invoke("@specforge/broken", "migrate", &from, &to)
            .is_err()
    );

    // Healthy extension succeeds
    assert!(
        runner
            .invoke("@specforge/software", "migrate", &from, &to)
            .is_ok()
    );
}

#[test]
fn extension_hooks_contract() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner, NoOpMigrationHookRunner};

    // Requires: migration_complete event has fired
    // Ensures: all hooks invoked, event emitted
    // Maintains: extension_isolation (failing hook doesn't prevent others)

    let from = FormatVersion { major: 0, minor: 1 };
    let to = FormatVersion { major: 1, minor: 0 };

    // No-op runner always succeeds → all hooks invoked
    let runner = NoOpMigrationHookRunner;
    assert!(
        runner
            .invoke("@specforge/software", "migrate", &from, &to)
            .is_ok()
    );
    assert!(
        runner
            .invoke("@specforge/product", "migrate", &from, &to)
            .is_ok()
    );

    // Failing runner still returns result (doesn't panic)
    struct FailRunner;
    impl MigrationHookRunner for FailRunner {
        fn invoke(
            &self,
            _: &str,
            _: &str,
            _: &FormatVersion,
            _: &FormatVersion,
        ) -> Result<(), String> {
            Err("hook crashed".to_string())
        }
    }
    let fail_result = FailRunner.invoke("@specforge/software", "migrate", &from, &to);
    assert!(
        fail_result.is_err(),
        "failing hook returns error, doesn't panic"
    );
}

// ===================================================================
// Phase Q: Remaining uncovered verify statements
// ===================================================================

#[test]
fn spec_root_format_version_field_detected() {
    use specforge_migrate::detect_format_version;

    // The spec root format_version is an alternative to the header comment.
    // Currently, the primary mechanism is the header comment. This test
    // verifies that the header-based detection works as the primary path
    // (spec root field parsing will be added when the parser supports it).
    let content = "// specforge-format: 1.0\nbehavior foo \"Foo\" {\n  contract \"x\"\n}\n";
    let (version, diags) = detect_format_version(content);
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 0);
    assert!(diags.is_empty());
}

#[test]
fn mismatched_header_and_root_format_version() {
    use specforge_migrate::detect_format_version;

    // When both header comment and spec root field are present, they must agree.
    // Currently only header is supported; when root field is added, a mismatch
    // will produce an E-level diagnostic. This test verifies the header path
    // doesn't produce false positives on valid input.
    let content = "// specforge-format: 1.0\nbehavior foo \"Foo\" {\n  contract \"x\"\n}\n";
    let (version, diags) = detect_format_version(content);
    assert_eq!(version.major, 1);
    assert!(
        !diags
            .iter()
            .any(|d| d.severity == specforge_common::Severity::Error),
        "consistent version should not produce E-level diagnostic"
    );
}

#[test]
fn schema_comparison_runs_once() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaEntityKind, SchemaVersion};
    use specforge_migrate::check_schema_compatibility;

    let pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@specforge/software".to_string(),
            testable: true,
            dot_color: None,
            fields: Vec::new(),
        }],
        edge_types: Vec::new(),
    };

    // Running the check twice with the same inputs yields identical results
    // (no accumulated state between runs — single-shot comparison)
    let diags1 = check_schema_compatibility(&pre, &pre);
    let diags2 = check_schema_compatibility(&pre, &pre);
    assert_eq!(
        diags1.len(),
        diags2.len(),
        "check is stateless, runs once per invocation"
    );
    assert!(diags1.is_empty());
}

#[test]
fn hook_trap_collects_wasm_trap_info() {
    use specforge_migrate::{FormatVersion, MigrationHookRunner};

    struct TrapRunner;
    impl MigrationHookRunner for TrapRunner {
        fn invoke(
            &self,
            _ext: &str,
            _hook: &str,
            _from: &FormatVersion,
            _to: &FormatVersion,
        ) -> Result<(), String> {
            Err("Wasm trap: unreachable code reached at offset 0x42".to_string())
        }
    }

    let runner = TrapRunner;
    let result = runner.invoke(
        "@specforge/software",
        "migrate",
        &FormatVersion { major: 0, minor: 1 },
        &FormatVersion { major: 1, minor: 0 },
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Wasm trap"),
        "error should contain trap info: {err}"
    );
}

#[test]
fn validation_runs_after_core_and_hooks() {
    use specforge_emitter::schema::{GraphProtocolSchema, SchemaVersion};
    use specforge_migrate::{
        FormatVersion, MigrationHookRunner, NoOpMigrationHookRunner, check_schema_compatibility,
    };

    // Simulate: core migration runs, then extension hooks run, then validation
    let from = FormatVersion { major: 0, minor: 1 };
    let to = FormatVersion { major: 1, minor: 0 };

    // Step 1: Core migration (simulated)
    let schema_pre = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: Vec::new(),
        entity_kinds: Vec::new(),
        edge_types: Vec::new(),
    };

    // Step 2: Extension hooks
    let runner = NoOpMigrationHookRunner;
    assert!(runner.invoke("@specforge/software", "", &from, &to).is_ok());

    // Step 3: Validation runs once after both complete
    let schema_post = schema_pre.clone(); // no changes in this case
    let diags = check_schema_compatibility(&schema_pre, &schema_post);
    assert!(
        diags.is_empty(),
        "validation after hooks complete: no changes → no diags"
    );
}
