use serde::Serialize;
use sha2::{Digest, Sha256};
use specforge_common::shape::Shape;
use specforge_common::{Diagnostic, Severity, codes, load_project_config, project_root_of};
use specforge_emitter::schema::{GraphProtocolSchema, SchemaMigration, diff_schemas};
use specforge_formatter::unified_diff;
use specforge_parser::{FORMAT_HEADER_PREFIX, FormatVersion, detect_format_version};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Migration Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Shape)]
pub struct MigrationResult {
    pub file_path: String,
    pub status: MigrationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_version: Option<FormatVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_version: Option<FormatVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Migrated,
    Skipped,
    Failed,
    Restored,
}

#[derive(Debug, Clone, Serialize, Shape)]
pub struct MigrationBackup {
    pub original_path: String,
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize, Shape)]
pub struct MigrationDiff {
    pub file_path: String,
    pub before_hash: String,
    pub after_hash: String,
    pub unified_text: String,
}

#[derive(Debug, Clone, Serialize, Shape)]
pub struct MigrationSummary {
    pub migrated_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub target_version: FormatVersion,
    pub results: Vec<MigrationResult>,
    pub backups: Vec<MigrationBackup>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diffs: Vec<MigrationDiff>,
}

#[derive(Debug, Clone, Serialize, Shape)]
pub struct RollbackSummary {
    pub restored_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub results: Vec<MigrationResult>,
    /// One warning per file skipped because its `.bak` backup is missing.
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Transform Functions (pure)
// ---------------------------------------------------------------------------

/// Update the format version header in a spec file's content.
/// If no header exists, prepend one. If a header exists, replace it.
fn set_format_version_header(content: &str, version: &FormatVersion) -> String {
    let new_header = format!("{FORMAT_HEADER_PREFIX}{version}");

    if let Some(line) = content
        .lines()
        .next()
        .filter(|l| l.starts_with(FORMAT_HEADER_PREFIX))
    {
        let rest = &content[line.len()..];
        return format!("{new_header}{rest}");
    }

    // Prepend header
    format!("{new_header}\n{content}")
}

/// Transform content to version `to`.
/// Currently v1 is the only version, so this just ensures the header is set.
fn transform_content(content: &str, to: &FormatVersion) -> String {
    set_format_version_header(content, to)
}

// ---------------------------------------------------------------------------
// SHA256 Hashing
// ---------------------------------------------------------------------------

fn sha256_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Schema and Graph Comparison
// ---------------------------------------------------------------------------

/// Compare pre/post migration schemas and return breaking change diagnostics.
pub fn check_schema_compatibility(
    pre: &GraphProtocolSchema,
    post: &GraphProtocolSchema,
) -> Vec<Diagnostic> {
    let migration: SchemaMigration = diff_schemas(pre, post);
    let mut diagnostics = Vec::new();

    for change in &migration.changes {
        if change.is_breaking() {
            diagnostics.push(
                Diagnostic::new(
                    codes::W053,
                    format!("breaking schema change after migration: {change:?}"),
                )
                .with_suggestion(
                    "Review the migration to ensure backward compatibility.".to_string(),
                ),
            );
        }
    }

    diagnostics
}

/// Compare two graphs for structural equivalence (entity IDs, edges, field values).
/// Returns diagnostics for any differences found, excluding source spans.
pub fn compare_graphs(
    pre: &specforge_graph::Graph,
    post: &specforge_graph::Graph,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let pre_nodes = pre.nodes();
    let post_nodes = post.nodes();

    // Check entity IDs
    let pre_ids: std::collections::BTreeSet<&str> =
        pre_nodes.iter().map(|n| n.id.raw.as_str()).collect();
    let post_ids: std::collections::BTreeSet<&str> =
        post_nodes.iter().map(|n| n.id.raw.as_str()).collect();

    for id in pre_ids.difference(&post_ids) {
        diagnostics.push(Diagnostic::new(
            codes::W054,
            format!("entity '{id}' present before migration but missing after"),
        ));
    }

    for id in post_ids.difference(&pre_ids) {
        diagnostics.push(Diagnostic::new(
            codes::W054,
            format!("entity '{id}' appeared after migration but was not present before"),
        ));
    }

    // Check edges
    let pre_edges: std::collections::BTreeSet<(&str, &str, &str)> = pre
        .edges()
        .iter()
        .map(|e| (e.source.as_str(), e.target.as_str(), e.label.as_str()))
        .collect();
    let post_edges: std::collections::BTreeSet<(&str, &str, &str)> = post
        .edges()
        .iter()
        .map(|e| (e.source.as_str(), e.target.as_str(), e.label.as_str()))
        .collect();

    for edge in pre_edges.difference(&post_edges) {
        diagnostics.push(Diagnostic::new(
            codes::W054,
            format!(
                "edge {}-[{}]->{} present before migration but missing after",
                edge.0, edge.2, edge.1
            ),
        ));
    }

    for edge in post_edges.difference(&pre_edges) {
        diagnostics.push(Diagnostic::new(
            codes::W054,
            format!(
                "edge {}-[{}]->{} appeared after migration but was not present before",
                edge.0, edge.2, edge.1
            ),
        ));
    }

    // Field values of entities present on both sides, compared as the
    // Graph Protocol exports them, without source positions.
    let post_by_id: std::collections::HashMap<&str, _> =
        post_nodes.iter().map(|n| (n.id.raw.as_str(), n)).collect();
    for pre_node in pre_nodes.iter() {
        let Some(post_node) = post_by_id.get(pre_node.id.raw.as_str()) else {
            continue;
        };
        let before = comparable_fields(&pre_node.fields);
        let after = comparable_fields(&post_node.fields);
        let keys: std::collections::BTreeSet<&String> = before.keys().chain(after.keys()).collect();
        for key in keys {
            if before.get(key) != after.get(key) {
                diagnostics.push(Diagnostic::new(
                    codes::W054,
                    format!(
                        "field '{key}' of entity '{}' changed during migration",
                        pre_node.id.raw.as_str()
                    ),
                ));
            }
        }
    }

    diagnostics
}

/// An entity's fields as exported JSON, with every source span removed:
/// a format migration shifts positions without changing structure.
fn comparable_fields(
    fields: &specforge_graph::FieldMap,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    fn strip_spans(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("span");
                map.values_mut().for_each(strip_spans);
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(strip_spans),
            _ => {}
        }
    }
    let mut map = specforge_emitter::field_map_to_json(fields);
    map.values_mut().for_each(strip_spans);
    map
}

// ---------------------------------------------------------------------------
// Core Migration Logic
// ---------------------------------------------------------------------------

/// `path` from `root`, `/`-separated; `path` itself when it is not under `root`.
fn diff_label(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Run migration on a single file of the project at `root`. Returns the result,
/// the backup it made and the diff, which labels the file with its path from
/// `root` so that `patch -p1` applies it there.
fn migrate_file(
    path: &Path,
    root: &Path,
    target_version: &FormatVersion,
    dry_run: bool,
    no_backup: bool,
) -> (
    MigrationResult,
    Option<MigrationBackup>,
    Option<MigrationDiff>,
) {
    let path_str = path.display().to_string();
    let label = diff_label(path, root);

    // Read file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return (
                MigrationResult {
                    file_path: path_str,
                    status: MigrationStatus::Failed,
                    from_version: None,
                    to_version: None,
                    error: Some(format!("failed to read file: {e}")),
                },
                None,
                None,
            );
        }
    };

    // Detect version; a header this build can't read fails the file.
    let (detected_version, diags) = detect_format_version(&content, &path_str);
    if let Some(error) = diags.iter().find(|d| d.severity == Severity::Error) {
        let guidance = error
            .suggestion
            .as_deref()
            .map(|s| format!(" — {s}"))
            .unwrap_or_default();
        return (
            MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Failed,
                from_version: Some(detected_version),
                to_version: None,
                error: Some(format!("{}: {}{guidance}", error.code, error.message)),
            },
            None,
            None,
        );
    }

    // Skip if already at target version
    if detected_version >= *target_version {
        return (
            MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Skipped,
                from_version: Some(detected_version.clone()),
                to_version: Some(detected_version),
                error: None,
            },
            None,
            None,
        );
    }

    // Transform
    let transformed = transform_content(&content, target_version);

    // Build diff
    let diff = if content != transformed {
        let diff_text = unified_diff(&format!("a/{label}"), &content, &transformed);
        // unified_diff uses the same path for both --- and +++.
        // We need +++ to use b/ prefix per POSIX convention.
        let unified_text = diff_text
            .diff_text
            .replace(&format!("+++ a/{label}"), &format!("+++ b/{label}"));
        Some(MigrationDiff {
            file_path: path_str.clone(),
            before_hash: sha256_hash(&content),
            after_hash: sha256_hash(&transformed),
            unified_text,
        })
    } else {
        None
    };

    if dry_run {
        return (
            MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Migrated,
                from_version: Some(detected_version),
                to_version: Some(target_version.clone()),
                error: None,
            },
            None,
            diff,
        );
    }

    // Create backup
    let backup = if !no_backup {
        let backup_path = path.with_extension("spec.bak");
        if let Err(e) = std::fs::copy(path, &backup_path) {
            return (
                MigrationResult {
                    file_path: path_str,
                    status: MigrationStatus::Failed,
                    from_version: Some(detected_version),
                    to_version: None,
                    error: Some(format!("failed to create backup: {e}")),
                },
                None,
                None,
            );
        }
        Some(MigrationBackup {
            original_path: path_str.clone(),
            backup_path: backup_path.display().to_string(),
        })
    } else {
        None
    };

    // Atomic write: temp + rename
    let tmp_path = path.with_extension("spec.tmp");
    if let Err(e) = std::fs::write(&tmp_path, &transformed) {
        return (
            MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Failed,
                from_version: Some(detected_version),
                to_version: None,
                error: Some(format!("failed to write temp file: {e}")),
            },
            backup,
            None,
        );
    }

    if let Err(e) = std::fs::rename(&tmp_path, path) {
        // Clean up temp file on rename failure
        let _ = std::fs::remove_file(&tmp_path);
        return (
            MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Failed,
                from_version: Some(detected_version),
                to_version: None,
                error: Some(format!("failed to rename temp file: {e}")),
            },
            backup,
            None,
        );
    }

    (
        MigrationResult {
            file_path: path_str,
            status: MigrationStatus::Migrated,
            from_version: Some(detected_version),
            to_version: Some(target_version.clone()),
            error: None,
        },
        backup,
        diff,
    )
}

/// Run rollback: restore `.spec.bak` files to their originals.
pub fn run_rollback(path: &Path) -> RollbackSummary {
    // The project's sources, then their .bak counterparts.
    let targets = project_sources(&project_root_of(path));

    let mut results = Vec::new();
    let mut restored = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut warnings = Vec::new();

    for target in &targets {
        let bak_path = target.with_extension("spec.bak");
        let path_str = target.display().to_string();

        if !bak_path.exists() {
            skipped += 1;
            warnings.push(format!(
                "no backup {} for {path_str}; skipped",
                bak_path.display()
            ));
            results.push(MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Skipped,
                from_version: None,
                to_version: None,
                error: None,
            });
            continue;
        }

        // Atomic restore: read backup, write to temp, rename
        let backup_content = match std::fs::read_to_string(&bak_path) {
            Ok(c) => c,
            Err(e) => {
                failed += 1;
                results.push(MigrationResult {
                    file_path: path_str,
                    status: MigrationStatus::Failed,
                    from_version: None,
                    to_version: None,
                    error: Some(format!("failed to read backup: {e}")),
                });
                continue;
            }
        };

        let tmp_path = target.with_extension("spec.restore.tmp");
        if let Err(e) = std::fs::write(&tmp_path, &backup_content) {
            failed += 1;
            results.push(MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Failed,
                from_version: None,
                to_version: None,
                error: Some(format!("failed to write restore temp: {e}")),
            });
            continue;
        }

        if let Err(e) = std::fs::rename(&tmp_path, target) {
            let _ = std::fs::remove_file(&tmp_path);
            failed += 1;
            results.push(MigrationResult {
                file_path: path_str,
                status: MigrationStatus::Failed,
                from_version: None,
                to_version: None,
                error: Some(format!("failed to rename restore: {e}")),
            });
            continue;
        }

        restored += 1;
        results.push(MigrationResult {
            file_path: path_str,
            status: MigrationStatus::Restored,
            from_version: None,
            to_version: None,
            error: None,
        });
    }

    RollbackSummary {
        restored_count: restored,
        skipped_count: skipped,
        failed_count: failed,
        results,
        warnings,
    }
}

/// The sources of the project at `project_root`: the
/// files a compile reads, under `spec_root` without what `exclude` leaves
/// out (ADR 0021 D3).
fn project_sources(project_root: &Path) -> Vec<PathBuf> {
    load_project_config(project_root).spec_files(project_root)
}

/// Migrate every source of the project `path` is in.
pub fn migrate_project(
    path: &Path,
    target_version: &FormatVersion,
    dry_run: bool,
    no_backup: bool,
) -> MigrationSummary {
    let project_root = project_root_of(path);
    let targets = project_sources(&project_root);

    let mut results = Vec::new();
    let mut backups = Vec::new();
    let mut diffs = Vec::new();
    let mut migrated = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for target_path in &targets {
        let (result, backup, diff) = migrate_file(
            target_path,
            &project_root,
            target_version,
            dry_run,
            no_backup,
        );

        match result.status {
            MigrationStatus::Migrated => migrated += 1,
            MigrationStatus::Skipped => skipped += 1,
            MigrationStatus::Failed => failed += 1,
            _ => {}
        }

        if let Some(d) = diff {
            diffs.push(d);
        }
        if let Some(b) = backup {
            backups.push(b);
        }
        results.push(result);
    }

    MigrationSummary {
        migrated_count: migrated,
        skipped_count: skipped,
        failed_count: failed,
        target_version: target_version.clone(),
        results,
        backups,
        diffs,
    }
}
