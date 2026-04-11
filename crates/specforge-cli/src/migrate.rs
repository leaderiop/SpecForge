use specforge_migrate::{
    FormatVersion, MigrationStatus, MigrationSummary, RollbackSummary,
    CURRENT_FORMAT_VERSION, MAX_SUPPORTED_VERSION,
    migrate_project, run_rollback,
};
use std::path::Path;
use std::str::FromStr;

pub fn run(
    path: &Path,
    dry_run: bool,
    no_backup: bool,
    rollback: bool,
    target_version: Option<&str>,
    format: &str,
) -> i32 {
    // Handle rollback mode
    if rollback {
        let summary = run_rollback(path);
        print_rollback(&summary, format);
        return if summary.failed_count > 0 { 1 } else { 0 };
    }

    // Parse and validate target version
    let target = match target_version {
        Some(v) => match FormatVersion::from_str(v) {
            Ok(ver) => {
                if ver > MAX_SUPPORTED_VERSION {
                    eprintln!(
                        "E015: unsupported target version {ver} (max supported: {MAX_SUPPORTED_VERSION})"
                    );
                    return 1;
                }
                ver
            }
            Err(e) => {
                eprintln!("E015: invalid target version '{v}': {e}");
                return 1;
            }
        },
        None => CURRENT_FORMAT_VERSION,
    };

    let summary = migrate_project(path, &target, dry_run, no_backup);
    print_migration(&summary, format, dry_run);

    if summary.failed_count > 0 { 1 } else { 0 }
}

fn print_rollback(summary: &RollbackSummary, format: &str) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(summary).unwrap_or_default();
            println!("{json}");
        }
        _ => {
            for r in &summary.results {
                match r.status {
                    MigrationStatus::Restored => eprintln!("  restored: {}", r.file_path),
                    MigrationStatus::Skipped => {}
                    MigrationStatus::Failed => {
                        eprintln!(
                            "  failed: {} ({})",
                            r.file_path,
                            r.error.as_deref().unwrap_or("unknown")
                        );
                    }
                    _ => {}
                }
            }
            eprintln!(
                "{} restored, {} skipped, {} failed",
                summary.restored_count, summary.skipped_count, summary.failed_count
            );
        }
    }
}

fn print_migration(summary: &MigrationSummary, format: &str, dry_run: bool) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(summary).unwrap_or_default();
            println!("{json}");
        }
        _ => {
            if dry_run {
                for d in &summary.diffs {
                    println!("{}", d.unified_text);
                }
            }

            for d in &summary.diagnostics {
                eprintln!("{}: {}", d.code, d.message);
            }

            eprintln!(
                "{} migrated, {} skipped, {} failed",
                summary.migrated_count, summary.skipped_count, summary.failed_count
            );
        }
    }
}
