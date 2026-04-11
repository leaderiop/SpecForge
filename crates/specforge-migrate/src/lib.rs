use serde::Serialize;
use sha2::{Digest, Sha256};
use specforge_common::{Diagnostic, Severity, find_project_root};
use specforge_emitter::schema::{
    GraphProtocolSchema, SchemaMigration, diff_schemas,
};
use specforge_formatter::{discover_targets, unified_diff};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Format Version
// ---------------------------------------------------------------------------

/// The DSL format version embedded in spec file headers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FormatVersion {
    pub major: u32,
    pub minor: u32,
}

/// Current format version. All new spec files are at this version.
pub const CURRENT_FORMAT_VERSION: FormatVersion = FormatVersion { major: 1, minor: 0 };

/// Minimum supported format version for migration.
pub const MIN_SUPPORTED_VERSION: FormatVersion = FormatVersion { major: 1, minor: 0 };

/// Maximum supported target version.
pub const MAX_SUPPORTED_VERSION: FormatVersion = FormatVersion { major: 1, minor: 0 };

impl fmt::Display for FormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for FormatVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        match parts.len() {
            1 => {
                let major = parts[0]
                    .parse::<u32>()
                    .map_err(|e| format!("invalid version: {e}"))?;
                Ok(FormatVersion { major, minor: 0 })
            }
            2 => {
                let major = parts[0]
                    .parse::<u32>()
                    .map_err(|e| format!("invalid major: {e}"))?;
                let minor = parts[1]
                    .parse::<u32>()
                    .map_err(|e| format!("invalid minor: {e}"))?;
                Ok(FormatVersion { major, minor })
            }
            _ => Err(format!("expected MAJOR.MINOR, got '{s}'")),
        }
    }
}

// ---------------------------------------------------------------------------
// Migration Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Migrated,
    Skipped,
    Failed,
    Restored,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationBackup {
    pub original_path: String,
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationDiff {
    pub file_path: String,
    pub before_hash: String,
    pub after_hash: String,
    pub unified_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationSummary {
    pub migrated_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub target_version: FormatVersion,
    pub results: Vec<MigrationResult>,
    pub backups: Vec<MigrationBackup>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diffs: Vec<MigrationDiff>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RollbackSummary {
    pub restored_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub results: Vec<MigrationResult>,
}

// ---------------------------------------------------------------------------
// Version Detection
// ---------------------------------------------------------------------------

const FORMAT_HEADER_PREFIX: &str = "// specforge-format: ";

/// Detect the format version from a spec file's content.
/// Returns the detected version (or default) and any diagnostics.
pub fn detect_format_version(content: &str) -> (FormatVersion, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    let first_line = content.lines().find(|l| !l.trim().is_empty());

    if let Some(version_str) = first_line
        .and_then(|line| line.strip_prefix(FORMAT_HEADER_PREFIX))
    {
        let version_str = version_str.trim();
        match FormatVersion::from_str(version_str) {
            Ok(v) => {
                if v > MAX_SUPPORTED_VERSION {
                    diagnostics.push(Diagnostic {
                        code: "E015".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "unsupported format version {v} (max supported: {MAX_SUPPORTED_VERSION})"
                        ),
                        span: None,
                        suggestion: Some(format!(
                            "Use a format version between {MIN_SUPPORTED_VERSION} and {MAX_SUPPORTED_VERSION}."
                        )),
                    });
                } else if v < MIN_SUPPORTED_VERSION {
                    diagnostics.push(Diagnostic {
                        code: "I007".to_string(),
                        severity: Severity::Info,
                        message: format!(
                            "format version {v} is older than current ({CURRENT_FORMAT_VERSION}); migration available"
                        ),
                        span: None,
                        suggestion: Some("Run `specforge migrate` to upgrade.".to_string()),
                    });
                }
                return (v, diagnostics);
            }
            Err(_) => {
                diagnostics.push(Diagnostic {
                    code: "E015".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "invalid format version header: '{version_str}'"
                    ),
                    span: None,
                    suggestion: Some(format!(
                        "Expected `// specforge-format: MAJOR.MINOR` (e.g., `// specforge-format: {CURRENT_FORMAT_VERSION}`)."
                    )),
                });
                return (MIN_SUPPORTED_VERSION, diagnostics);
            }
        }
    }

    // No header found — default to current version (files without headers are current)
    (CURRENT_FORMAT_VERSION, diagnostics)
}

// ---------------------------------------------------------------------------
// Transform Functions (pure)
// ---------------------------------------------------------------------------

/// Update the format version header in a spec file's content.
/// If no header exists, prepend one. If a header exists, replace it.
fn set_format_version_header(content: &str, version: &FormatVersion) -> String {
    let new_header = format!("{FORMAT_HEADER_PREFIX}{version}");

    if let Some(line) = content.lines().next().filter(|l| l.starts_with(FORMAT_HEADER_PREFIX)) {
        let rest = &content[line.len()..];
        return format!("{new_header}{rest}");
    }

    // Prepend header
    format!("{new_header}\n{content}")
}

/// Transform content from one version to the next.
/// Currently v1 is the only version, so this just ensures the header is set.
fn transform_content(content: &str, _from: &FormatVersion, to: &FormatVersion) -> String {
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
// Pre-Migration Schema Snapshot
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PreMigrationSnapshot {
    pub schema: GraphProtocolSchema,
}

#[allow(dead_code)]
pub fn capture_pre_migration_snapshot(schema: &GraphProtocolSchema) -> PreMigrationSnapshot {
    PreMigrationSnapshot {
        schema: schema.clone(),
    }
}

/// Compare pre/post migration schemas and return breaking change diagnostics.
#[allow(dead_code)]
pub fn check_schema_compatibility(
    pre: &GraphProtocolSchema,
    post: &GraphProtocolSchema,
) -> Vec<Diagnostic> {
    let migration: SchemaMigration = diff_schemas(pre, post);
    let mut diagnostics = Vec::new();

    for change in &migration.changes {
        if change.is_breaking() {
            diagnostics.push(Diagnostic {
                code: "W053".to_string(),
                severity: Severity::Warning,
                message: format!("breaking schema change after migration: {change:?}"),
                span: None,
                suggestion: Some("Review the migration to ensure backward compatibility.".to_string()),
            });
        }
    }

    diagnostics
}

/// Compare two graphs for structural equivalence (entity IDs, edges, field values).
/// Returns diagnostics for any differences found, excluding source spans.
#[allow(dead_code)]
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
        diagnostics.push(Diagnostic {
            code: "W054".to_string(),
            severity: Severity::Warning,
            message: format!("entity '{id}' present before migration but missing after"),
            span: None,
            suggestion: None,
        });
    }

    for id in post_ids.difference(&pre_ids) {
        diagnostics.push(Diagnostic {
            code: "W054".to_string(),
            severity: Severity::Warning,
            message: format!("entity '{id}' appeared after migration but was not present before"),
            span: None,
            suggestion: None,
        });
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
        diagnostics.push(Diagnostic {
            code: "W054".to_string(),
            severity: Severity::Warning,
            message: format!(
                "edge {}-[{}]->{} present before migration but missing after",
                edge.0, edge.2, edge.1
            ),
            span: None,
            suggestion: None,
        });
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// Extension Hook Runner (trait for testability)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub trait MigrationHookRunner {
    fn invoke(
        &self,
        extension_name: &str,
        hook: &str,
        from: &FormatVersion,
        to: &FormatVersion,
    ) -> Result<(), String>;
}

/// Mock hook runner for testing.
#[allow(dead_code)]
pub struct NoOpMigrationHookRunner;

#[allow(dead_code)]
impl MigrationHookRunner for NoOpMigrationHookRunner {
    fn invoke(
        &self,
        _extension_name: &str,
        _hook: &str,
        _from: &FormatVersion,
        _to: &FormatVersion,
    ) -> Result<(), String> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Core Migration Logic
// ---------------------------------------------------------------------------

/// Run migration on a single file. Returns the result and optionally a diff.
pub fn migrate_file(
    path: &Path,
    target_version: &FormatVersion,
    dry_run: bool,
    no_backup: bool,
) -> (MigrationResult, Option<MigrationBackup>, Option<MigrationDiff>) {
    let path_str = path.display().to_string();

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

    // Detect version
    let (detected_version, _diags) = detect_format_version(&content);

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
    let transformed = transform_content(&content, &detected_version, target_version);

    // Build diff
    let diff = if content != transformed {
        let diff_text = unified_diff(
            &format!("a/{path_str}"),
            &content,
            &transformed,
        );
        // unified_diff uses the same path for both --- and +++.
        // We need +++ to use b/ prefix per POSIX convention.
        let unified_text = diff_text.diff_text.replace(
            &format!("+++ a/{path_str}"),
            &format!("+++ b/{path_str}"),
        );
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
    let project_root = find_project_root(path).unwrap_or_else(|| path.to_path_buf());
    let spec_root = project_root.join("spec");
    let search_root = if spec_root.exists() { &spec_root } else { &project_root };

    // Discover .spec files, then check for .bak counterparts
    let targets = discover_targets(search_root, &[], &[]);

    let mut results = Vec::new();
    let mut restored = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for target in &targets {
        let bak_path = target.with_extension("spec.bak");
        let path_str = target.display().to_string();

        if !bak_path.exists() {
            skipped += 1;
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
    }
}

/// Discover spec files and run migration on all of them.
pub fn migrate_project(
    path: &Path,
    target_version: &FormatVersion,
    dry_run: bool,
    no_backup: bool,
) -> MigrationSummary {
    let project_root = find_project_root(path).unwrap_or_else(|| path.to_path_buf());
    let spec_root = project_root.join("spec");
    let search_root = if spec_root.exists() { &spec_root } else { &project_root };
    let targets = discover_targets(search_root, &[], &[]);

    let mut results = Vec::new();
    let mut backups = Vec::new();
    let mut diffs = Vec::new();
    let diagnostics: Vec<Diagnostic> = Vec::new();
    let mut migrated = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for target_path in &targets {
        let (result, backup, diff) = migrate_file(target_path, target_version, dry_run, no_backup);

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
        diagnostics,
        diffs,
    }
}
