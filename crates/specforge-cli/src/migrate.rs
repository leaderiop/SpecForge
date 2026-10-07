use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
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
        print_rollback(
            &summary,
            &migrate::restored(&summary).names_under(path),
            format,
        );
        return Exit::of_verdict(summary.failed_count == 0).code();
    }

    let target = match migrate::parse_target(target_version) {
        Ok(target) => target,
        Err(error) => return Refusal::of(format).report(&error),
    };

    // The shared migration: migrate, run the extensions' hooks, then check
    // the graph kept its structure, rolling back when it didn't.
    let runtime = specforge_component::project_runtime(path);
    let request = Request {
        root: path,
        target,
        dry_run,
        no_backup,
    };
    let outcome = migrate::run(&request, Some(&runtime));
    // Each migrated file and each backup (none for a dry run).
    let written = (!dry_run).then(|| outcome.writes.names_under(path));
    print_migration(&outcome.summary, written.as_deref(), format, dry_run);

    if outcome.summary.failed_count != 0 {
        return Exit::of_verdict(outcome.ok()).code();
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
    }

    Exit::of_verdict(outcome.ok()).code()
}

/// The JSON of `document` with `files_written`, when given.
fn with_files_written(document: impl serde::Serialize, files_written: Option<&[String]>) -> String {
    let mut json = serde_json::to_value(document).unwrap_or_default();
    if let (Some(files), Some(object)) = (files_written, json.as_object_mut()) {
        object.insert("files_written".to_string(), serde_json::json!(files));
    }
    serde_json::to_string_pretty(&json).unwrap_or_default()
}

/// The restore; its JSON lists each file restored in `files_written`.
fn print_rollback(summary: &RollbackSummary, files_written: &[String], format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", with_files_written(summary, Some(files_written)));
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

/// The migration; its JSON lists each migrated file and each backup in
/// `files_written` (absent from a dry run).
fn print_migration(
    summary: &MigrationSummary,
    files_written: Option<&[String]>,
    format: OutputFormat,
    dry_run: bool,
) {
    match format {
        OutputFormat::Json => {
            println!("{}", with_files_written(summary, files_written));
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
