//! `specforge watch` — incremental rebuild loop over a [`ProjectSession`].
//!
//! The session is seeded by one cold build and kept current from
//! file-watcher events; every event reports its diagnostics, the set
//! `specforge check` reports for the same sources. The watchers only say
//! which paths changed: what a path is to the project, and what its change
//! does, is the session's (`classify_project_changes`).

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use specforge_ops::check::Counts;
use specforge_project::{Changes, InputRole, ProjectSession, Update, UpdateKind, WatchRoot};
use specforge_watch::SpecWatcher;

pub fn run(path: &Path, json: bool, verify_incremental: bool) -> i32 {
    // A debug build checks every rebuild (ProjectSession); a release build
    // only when asked (the check costs a cold rebuild per change).
    let mut session = ProjectSession::open(path);
    if verify_incremental {
        session.set_verify_incremental(true);
    }

    // Start watching before announcing readiness: a client that writes on
    // seeing "ready" must never race a watcher that does not exist yet.
    // One watcher per directory the session is built from.
    let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
    let mut roots = session.inputs().watch_roots();
    let mut watchers = match arm(path, &roots, &tx) {
        Ok(watchers) => watchers,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // What was written between the open's stamps and the watchers is applied
    // before `ready` (ADR 0030): the session follows the disk it saw when
    // the watchers were armed.
    while session.ensure_fresh().is_some() {
        let now = session.inputs().watch_roots();
        if now == roots {
            break;
        }
        match arm(path, &now, &tx) {
            Ok(rearmed) => {
                watchers = rearmed;
                roots = now;
            }
            Err(e) => {
                eprintln!("warning: {e}");
                break;
            }
        }
    }

    let spec_root: PathBuf = std::fs::canonicalize(&session.environment().spec_root)
        .unwrap_or_else(|_| session.environment().spec_root.clone());
    let diagnostics = session.diagnostics();
    let Counts {
        errors, warnings, ..
    } = Counts::of(&diagnostics);
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
        let roles: Vec<InputRole> = batch
            .iter()
            .map(|path| session.inputs().classify(path))
            .collect();
        if debug {
            eprintln!("[watch] batch: {batch:?} -> {roles:?}");
        }
        let changed = changed_labels(&session, &batch, &roles);
        let Some(update) = session.apply(&Changes::from_roles(roles)) else {
            continue;
        };
        report(&session, &update, &changed, json);
        // An edit that names a file the checks read, or a reload that moved
        // the spec root or loaded modules from elsewhere: follow the
        // session's inputs, then catch up on what was written while no
        // watcher covered it (ADR 0030).
        let mut inputs_changed = update.inputs_changed;
        while inputs_changed {
            let now = session.inputs().watch_roots();
            if now == roots {
                break;
            }
            match arm(path, &now, &tx) {
                Ok(rearmed) => {
                    watchers = rearmed;
                    roots = now;
                }
                Err(e) => {
                    eprintln!("warning: {e}");
                    break;
                }
            }
            inputs_changed = match session.ensure_fresh() {
                Some(update) => {
                    report(&session, &update, &update.rebuilt_files, json);
                    update.inputs_changed
                }
                None => false,
            };
        }
    }
    drop(watchers);

    0
}

/// One watcher per directory in `roots`, each sending its batches to `tx`.
fn arm(
    path: &Path,
    roots: &[WatchRoot],
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
        .map(|root| {
            let watch = if root.recursive {
                SpecWatcher::new
            } else {
                SpecWatcher::shallow
            };
            watch(
                &root.dir,
                tx.clone(),
                specforge_watch::DEFAULT_DEBOUNCE_WINDOW,
            )
        })
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
    let Counts {
        errors, warnings, ..
    } = Counts::of(&update.diagnostics);
    if update.kind == UpdateKind::Environment {
        let extensions: Vec<&str> = session
            .environment()
            .registries
            .declarations()
            .iter()
            .map(|d| d.name())
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
