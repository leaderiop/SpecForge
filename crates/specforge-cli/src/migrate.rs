use crate::OutputFormat;
use specforge_migrate::{MigrationStatus, MigrationSummary, RollbackSummary};
use specforge_ops::migrate::{self, Request};
use std::path::Path;

pub fn run(
    path: &Path,
    dry_run: bool,
    no_backup: bool,
    rollback: bool,
    target_version: Option<&str>,
    format: OutputFormat,
) -> i32 {
    // Handle rollback mode
    if rollback {
        let summary = migrate::rollback(path);
        print_rollback(&summary, format);
        return if summary.failed_count > 0 { 1 } else { 0 };
    }

    let target = match migrate::parse_target(target_version) {
        Ok(target) => target,
        Err(error) => {
            eprintln!("{}: {}", error.code, error.message);
            if let Some(suggestion) = &error.suggestion {
                eprintln!("  help: {suggestion}");
            }
            return 1;
        }
    };

    // The shared migration: migrate, run the extensions' hooks, then check
    // the graph kept its structure, rolling back when it didn't.
    let runtime = crate::pipeline::build_runtime(path);
    let request = Request {
        root: path,
        target,
        dry_run,
        no_backup,
    };
    let outcome = migrate::run(&request, Some(&runtime));
    print_migration(&outcome.summary, format, dry_run);

    if outcome.summary.failed_count > 0 {
        return 1;
    }
    for failure in &outcome.hook_failures {
        eprintln!("migration hook failure: {failure}");
    }
    // Graph Protocol compatibility: breaking schema changes warn W053.
    for warning in &outcome.schema_warnings {
        eprintln!("warning[{}]: {}", warning.code, warning.message);
    }
    for d in &outcome.structural_differences {
        eprintln!("{}: {}", d.code, d.message);
    }
    if outcome.rollback.is_some() {
        if outcome.hook_failures.is_empty() {
            eprintln!("migration changed the graph structure; files restored from backups");
        } else {
            eprintln!("files restored from backups");
        }
        return 1;
    }

    0
}

fn print_rollback(summary: &RollbackSummary, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(summary).unwrap_or_default();
            println!("{json}");
        }
        OutputFormat::Human => {
            for w in &summary.warnings {
                eprintln!("warning: {w}");
            }
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

fn print_migration(summary: &MigrationSummary, format: OutputFormat, dry_run: bool) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(summary).unwrap_or_default();
            println!("{json}");
        }
        OutputFormat::Human => {
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
