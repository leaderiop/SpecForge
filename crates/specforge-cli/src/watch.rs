//! `specforge watch` — incremental rebuild loop over a [`ProjectSession`].
//!
//! The session is seeded by one cold build and kept current from
//! file-watcher events; every event reports its diagnostics, the set
//! `specforge check` reports for the same sources. The watchers only say
//! which paths changed: what a path is to the project, and what its change
//! does, is the session's (`classify_project_changes`).

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use specforge_common::{Diagnostic, Severity};
use specforge_project::{Changes, InputRole, ProjectSession, Update, UpdateKind};
use specforge_watch::SpecWatcher;

pub fn run(path: &Path, json: bool, verify_incremental: bool) -> i32 {
    // A debug build of the compiler checks every rebuild; a release build
    // only when asked (the check costs a cold rebuild per change).
    let verify_incremental = verify_incremental || cfg!(debug_assertions);
    let mut session = ProjectSession::open(path);
    session.set_verify_incremental(verify_incremental);
    let spec_root: PathBuf = std::fs::canonicalize(&session.environment().spec_root)
        .unwrap_or_else(|_| session.environment().spec_root.clone());

    // Start watching before announcing readiness: a client that writes on
    // seeing "ready" must never race a watcher that does not exist yet.
    // One watcher per directory the session is built from.
    let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
    let mut roots = session.watch_roots();
    let mut watchers = match arm(path, &roots, &tx) {
        Ok(watchers) => watchers,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let diagnostics = session.diagnostics();
    let (errors, warnings) = counts(&diagnostics);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "event": "ready",
                "spec_root": spec_root.to_string_lossy(),
                "files": session.file_count(),
                "nodes": session.graph().node_count(),
                "edges": session.graph().edge_count(),
                "errors": errors,
                "warnings": warnings,
                "diagnostics": specforge_common::diagnostics_json(&diagnostics),
            })
        );
    } else {
        println!(
            "specforge watch: {} ({} files, {} nodes, {} edges, {} errors, {} warnings)",
            spec_root.display(),
            session.file_count(),
            session.graph().node_count(),
            session.graph().edge_count(),
            errors,
            warnings
        );
        println!("watching for changes (Ctrl-C to stop)");
    }

    // Watch loop: debounced batches of changed paths. The session says
    // what they are and applies them: sources take the incremental path
    // (tree-sitter trees are retained, so unchanged subtrees are not
    // re-parsed), an environment input reloads the environment, a check
    // input re-runs the checks; anything else changes nothing.
    let debug = std::env::var("SPECFORGE_WATCH_DEBUG").is_ok();
    for batch in &rx {
        let roles = session.classify_all(batch.iter().map(PathBuf::as_path));
        if debug {
            eprintln!("[watch] batch: {batch:?} -> {roles:?}");
        }
        let changed = changed_labels(&session, &batch, &roles);
        let Some(update) = session.apply(&Changes::from_roles(roles)) else {
            continue;
        };
        report(&session, &update, &changed, json);
        if update.kind != UpdateKind::Environment {
            continue;
        }
        // The reload may have moved the spec root or loaded modules from
        // elsewhere: follow the session's inputs, then catch up on what was
        // written while no watcher covered it.
        let now = session.watch_roots();
        if now == roots {
            continue;
        }
        match arm(path, &now, &tx) {
            Ok(rearmed) => {
                watchers = rearmed;
                roots = now;
            }
            Err(e) => eprintln!("warning: {e}"),
        }
        if let Some(update) = session.ensure_fresh() {
            report(&session, &update, &update.rebuilt_files, json);
        }
    }
    drop(watchers);

    0
}

/// One watcher per directory in `roots`, each sending its batches to `tx`.
fn arm(
    path: &Path,
    roots: &[PathBuf],
    tx: &mpsc::Sender<Vec<PathBuf>>,
) -> Result<Vec<SpecWatcher>, String> {
    if roots.is_empty() {
        return Err(format!(
            "failed to watch directory: {} does not exist",
            path.display()
        ));
    }
    roots
        .iter()
        .map(|root| SpecWatcher::new(root, tx.clone(), specforge_watch::DEFAULT_DEBOUNCE_WINDOW))
        .collect()
}

/// How the changed paths of a batch are named in its event: a source by
/// its key under the spec root, any other input by its path under the
/// project root (absolute outside it). Paths that change nothing are not
/// named.
fn changed_labels(session: &ProjectSession, batch: &[PathBuf], roles: &[InputRole]) -> Vec<String> {
    let root = std::fs::canonicalize(&session.environment().root)
        .unwrap_or_else(|_| session.environment().root.clone());
    let mut labels: Vec<String> = batch
        .iter()
        .zip(roles)
        .filter_map(|(path, role)| match role {
            InputRole::Unrelated => None,
            InputRole::Source(key) => Some(key.clone()),
            InputRole::Environment | InputRole::CheckInput => Some(
                path.strip_prefix(&root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .into_owned(),
            ),
        })
        .collect();
    labels.sort();
    labels.dedup();
    labels
}

/// Print what `update` did: `rebuilt` for sources, `rechecked` for check
/// inputs (with the same fields), `extensions_reloaded` for the
/// environment.
fn report(session: &ProjectSession, update: &Update, changed: &[String], json: bool) {
    let (errors, warnings) = counts(&update.diagnostics);
    if update.kind == UpdateKind::Environment {
        let extensions: Vec<&str> = session
            .environment()
            .registries
            .manifests
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "event": "extensions_reloaded",
                    "extensions": extensions,
                    "files": session.file_count(),
                    "nodes": session.graph().node_count(),
                    "errors": errors,
                    "warnings": warnings,
                    "diagnostics": specforge_common::diagnostics_json(&update.diagnostics),
                })
            );
        } else {
            println!(
                "[reload] extension environment changed: {} extension(s), {} file(s) | {} errors, {} warnings",
                extensions.len(),
                session.file_count(),
                errors,
                warnings
            );
        }
        return;
    }

    let event = match update.kind {
        UpdateKind::Checks => "rechecked",
        _ => "rebuilt",
    };
    if json {
        println!(
            "{}",
            serde_json::json!({
                "event": event,
                "changed": changed,
                "rebuilt_files": update.rebuilt_files,
                "added_nodes": update.delta.added_nodes.len(),
                "removed_nodes": update.delta.removed_nodes.len(),
                "modified_nodes": update.delta.modified_nodes.len(),
                "added_edges": update.delta.added_edges.len(),
                "removed_edges": update.delta.removed_edges.len(),
                "errors": errors,
                "warnings": warnings,
                "diagnostics": specforge_common::diagnostics_json(&update.diagnostics),
                "changed_diagnostic_files": update.changed_diagnostic_files,
                "verification_failed": matches!(update.verification, Some(Err(_))),
                "verification": match &update.verification {
                    None => serde_json::Value::Null,
                    Some(Ok(())) => "passed".into(),
                    Some(Err(msg)) => msg.clone().into(),
                },
            })
        );
    } else {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        println!(
            "[{stamp}] {event} {} file(s): +{} -{} ~{} nodes, +{} -{} edges | {} errors, {} warnings",
            update.rebuilt_files.len(),
            update.delta.added_nodes.len(),
            update.delta.removed_nodes.len(),
            update.delta.modified_nodes.len(),
            update.delta.added_edges.len(),
            update.delta.removed_edges.len(),
            errors,
            warnings
        );
        if let Some(Err(msg)) = &update.verification {
            eprintln!("verification FAILED: {msg}");
        }
    }
}

/// (errors, warnings) among `diagnostics`.
fn counts(diagnostics: &[Diagnostic]) -> (usize, usize) {
    let count = |severity: Severity| {
        diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    };
    (count(Severity::Error), count(Severity::Warning))
}
