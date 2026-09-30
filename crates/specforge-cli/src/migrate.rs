use crate::OutputFormat;
use specforge_migrate::{
    CURRENT_FORMAT_VERSION, FormatVersion, MAX_SUPPORTED_VERSION, MIN_SUPPORTED_VERSION,
    MigrationStatus, MigrationSummary, RollbackSummary, capture_pre_migration_snapshot,
    check_schema_compatibility, compare_graphs, migrate_project, run_rollback,
};
use std::path::Path;
use std::str::FromStr;

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
        let summary = run_rollback(path);
        print_rollback(&summary, format);
        return if summary.failed_count > 0 { 1 } else { 0 };
    }

    // Criterion 3 (pre-side): capture the compiled graph before any file is
    // touched, so post-migration validation can confirm structural
    // equivalence.
    // The schema snapshot is the before-state the W053 compatibility check
    // compares against.
    let pre = (!dry_run).then(|| {
        let ctx = crate::pipeline::compile(path);
        let snapshot = capture_pre_migration_snapshot(&crate::export::build_schema(&ctx));
        (ctx.graph, snapshot)
    });

    // Parse and validate target version
    let target = match target_version {
        Some(v) => match FormatVersion::from_str(v) {
            Ok(ver) => {
                if ver > MAX_SUPPORTED_VERSION {
                    eprintln!(
                        "E019: unsupported target version {ver} (max supported: {MAX_SUPPORTED_VERSION})"
                    );
                    eprintln!(
                        "  help: Use a format version between {MIN_SUPPORTED_VERSION} and {MAX_SUPPORTED_VERSION}."
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

    if summary.failed_count > 0 {
        return 1;
    }

    // Criterion 5: invoke declared extension migration hooks in
    // topological (dependency) order. Extensions without a hook are
    // skipped silently.
    if !dry_run && let Err(e) = invoke_migration_hooks(path) {
        eprintln!("migration hook failure: {e}");
        run_rollback(path);
        eprintln!("files restored from backups");
        return 1;
    }

    // Criterion 3: post-migration validation - the graph must be
    // structurally equivalent to the pre-migration graph.
    if let Some((pre_graph, snapshot)) = pre {
        let post = crate::pipeline::compile(path);
        // Graph Protocol compatibility: breaking schema changes warn W053.
        let post_schema = crate::export::build_schema(&post);
        for warning in check_schema_compatibility(&snapshot.schema, &post_schema) {
            eprintln!("warning[{}]: {}", warning.code, warning.message);
        }
        let structural = compare_graphs(&pre_graph, &post.graph);
        if !structural.is_empty() {
            run_rollback(path);
            for d in &structural {
                eprintln!("{}: {}", d.code, d.message);
            }
            eprintln!("migration changed the graph structure; files restored from backups");
            return 1;
        }
    }

    0
}

/// Invoke every installed extension's declared migration hook, in
/// topological dependency order. Extensions without a `migration_hook`
/// are skipped.
fn invoke_migration_hooks(path: &Path) -> Result<Vec<String>, String> {
    use specforge_wasm::WasmRuntime;
    use specforge_wasm::runtime::WasmCallResult;

    let config = specforge_common::load_project_config(path);
    let runtime = crate::pipeline::build_runtime(path);
    let mut load_diags = Vec::new();
    let manifests =
        specforge_emitter::compile::load_extensions(&config.extensions, &runtime, &mut load_diags);
    let order = specforge_wasm::topological_sort_extensions(&manifests).map_err(|ds| {
        ds.first()
            .map(|d| d.message.clone())
            .unwrap_or_else(|| "dependency cycle".to_string())
    })?;

    let mut invoked = Vec::new();
    for name in &order {
        let Some(manifest) = manifests.iter().find(|m| &m.name == name) else {
            continue;
        };
        let Some(hook) = &manifest.migration_hook else {
            continue;
        };
        let payload = serde_json::to_vec(&serde_json::json!({}))
            .map_err(|e| format!("hook payload serialization failed: {e}"))?;
        match runtime.call_export(name, hook, &payload) {
            WasmCallResult::Ok(_) => invoked.push(format!("{name}:{hook}")),
            WasmCallResult::Trap(trap) => {
                return Err(format!(
                    "migration hook '{hook}' of {name} did not execute: {}: {}",
                    trap.kind, trap.message
                ));
            }
        }
    }
    Ok(invoked)
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
