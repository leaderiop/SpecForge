//! `specforge.migrate`: run the migration pipeline (`specforge_ops::migrate`).

use serde_json::{Value, json};

use crate::args::Arguments;
use crate::mutation::{Mutated, Written};
use crate::target::ProjectRef;
use crate::tool::{McpError, ToolOutcome};

/// `specforge.migrate`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Return the diffs without changing any file
    dry_run: bool,
    /// Format version to migrate to, as MAJOR.MINOR (defaults to the current format version)
    target_version: Option<String>,
    /// Skip the .bak backup of each migrated file
    no_backup: bool,
}

pub(crate) fn call(project: &ProjectRef<'_>, args: Args) -> Mutated {
    // The project the call migrates, and the runtime its hooks run in.
    let path = project.root;
    let dry_run = args.dry_run;
    let no_backup = args.no_backup;
    // The format version to migrate to, checked as `specforge migrate
    // --target-version` checks it.
    let target = match specforge_ops::migrate::parse_target(args.target_version.as_deref()) {
        Ok(target) => target,
        Err(error) => return Mutated::refused_after(dry_run, error),
    };

    let runtime = project.runtime;
    // The migration `specforge migrate` runs, hooks and rollback included.
    let request = specforge_ops::migrate::Request {
        root: path,
        target,
        dry_run,
        no_backup,
    };
    let outcome = specforge_ops::migrate::run(&request, Some(runtime.clone()));
    let (from, to) = (outcome.from.to_string(), outcome.to.to_string());
    // The format version lives in each spec file's header: with no file
    // behind the target, the project is current and nothing ran: a
    // migration that wrote nothing (a dry run is a preview).
    let migration = |reply: ToolOutcome, writes: specforge_ops::Writes| match dry_run {
        true => Mutated::preview(reply),
        false => Mutated::wrote(reply, Written::files(writes)),
    };
    if !outcome.pending {
        let current = json!({
            "from_version": from,
            "to_version": to,
            "migrated": false,
            "dry_run": dry_run,
            "changes": [],
            "message": "project is already at the latest format version",
            "ok": outcome.ok(),
        });
        return migration(ToolOutcome::ok(current), outcome.writes);
    }

    let summary = &outcome.summary;
    let post_migration_errors: Vec<Value> = outcome
        .post_errors()
        .map(|d| json!({"code": d.code, "message": d.message}))
        .collect();
    let result = json!({
        "ok": outcome.ok(),
        "from_version": from,
        "to_version": to,
        "migrated": outcome.migrated(),
        "dry_run": dry_run,
        "files_migrated": summary.migrated_count,
        "files_skipped": summary.skipped_count,
        "files_failed": summary.failed_count,
        "results": summary.results,
        "diffs": summary.diffs,
        "hooks_invoked": outcome.hooks_invoked,
        "hook_failures": outcome.hook_failures,
        "schema_warnings": specforge_common::diagnostics_json(&outcome.schema_warnings),
        "structural_differences": specforge_common::diagnostics_json(&outcome.structural_differences),
        "rolled_back": outcome.rollback.is_some(),
        "rollback": outcome.rollback,
        "post_migration_validated": outcome.validated,
        "post_migration_errors": post_migration_errors,
    });
    // A failed run's report rides in `data`, and what it left written (its
    // backups after a rollback, the files migrated before a failure) is
    // reported.
    let reply = match outcome.failure() {
        Some(failure) => McpError::from(failure).with_data(result).into(),
        None => ToolOutcome::ok(result),
    };
    migration(reply, outcome.writes)
}
