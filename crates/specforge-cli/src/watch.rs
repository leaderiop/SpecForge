//! `specforge watch` — incremental rebuild loop over a [`ProjectSession`].
//!
//! The session is seeded by one cold build and kept current from
//! file-watcher events; every event reports its diagnostics, the set
//! `specforge check` reports for the same sources.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use specforge_common::{Diagnostic, Severity};
use specforge_project::{ProjectSession, SourceChange};
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
    let (tx, rx) = mpsc::channel::<Vec<specforge_watch::WatchEvent>>();
    let watcher = match SpecWatcher::new(&spec_root, tx, specforge_watch::DEFAULT_DEBOUNCE_WINDOW) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    // Spec-root watch cannot see specforge.json (it lives in the project
    // root), so extension config/plugin artifacts get their own watcher
    // scoped to those kinds (hardening-plan H3 / R-5).
    let (env_tx, env_rx) = mpsc::channel::<Vec<specforge_watch::WatchEvent>>();
    let env_root: PathBuf = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let env_watcher = match SpecWatcher::new_filtered(
        &env_root,
        env_tx,
        &[
            specforge_watch::WatchEventKind::Config,
            specforge_watch::WatchEventKind::Plugin,
        ],
        specforge_watch::DEFAULT_DEBOUNCE_WINDOW,
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // A running MCP server compares this marker with its own compile.
    write_freshness_marker(&session);
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
                "diagnostics": diagnostics,
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

    // Watch loop: debounced batches from the watcher drive incremental
    // rebuilds. Tree-sitter trees are retained across rebuilds, so
    // unchanged subtrees are not re-parsed.
    // Merge both channels: spec-only batches take the incremental path,
    // config/plugin batches reload the environment.
    let debug = std::env::var("SPECFORGE_WATCH_DEBUG").is_ok();
    let (merge_tx, rx2) = mpsc::channel::<Vec<specforge_watch::WatchEvent>>();
    {
        let tx_spec = merge_tx.clone();
        std::thread::spawn(move || {
            for batch in rx {
                if debug {
                    eprintln!("[spec-watcher] forwarding {} events", batch.len());
                }
                if tx_spec.send(batch).is_err() {
                    break;
                }
            }
            if debug {
                eprintln!("[spec-watcher] channel closed");
            }
        });
        std::thread::spawn(move || {
            for batch in env_rx {
                if debug {
                    eprintln!("[env-watcher] forwarding {} events", batch.len());
                }
                if merge_tx.send(batch).is_err() {
                    break;
                }
            }
            if debug {
                eprintln!("[env-watcher] channel closed");
            }
        });
    }

    for batch in rx2 {
        // Extension environment changed: reload it, with a fresh runtime
        // (new/replaced/uninstalled plugins and config). Spec-only batches
        // stay on the incremental path.
        let config_or_plugin = batch
            .iter()
            .any(|e| !matches!(e.kind, specforge_watch::WatchEventKind::Spec));
        if config_or_plugin {
            if debug {
                eprintln!("[watch] reload branch entered");
            }
            let update = session.reload_environment();
            write_freshness_marker(&session);
            let (errors, warnings) = counts(&update.diagnostics);
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
                        "diagnostics": update.diagnostics,
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
            continue;
        }

        let specs: Vec<String> = batch
            .iter()
            .filter(|e| matches!(e.kind, specforge_watch::WatchEventKind::Spec))
            .map(|e| e.path.clone())
            .collect();
        let result = session.update(SourceChange::Disk(&specs));
        write_freshness_marker(&session);
        let (errors, warnings) = counts(&result.diagnostics);

        if json {
            println!(
                "{}",
                serde_json::json!({
                    "event": "rebuilt",
                    "changed": batch.iter().map(|e| e.path.clone()).collect::<Vec<_>>(),
                    "rebuilt_files": result.rebuilt_files,
                    "added_nodes": result.delta.added_nodes.len(),
                    "removed_nodes": result.delta.removed_nodes.len(),
                    "modified_nodes": result.delta.modified_nodes.len(),
                    "added_edges": result.delta.added_edges.len(),
                    "removed_edges": result.delta.removed_edges.len(),
                    "errors": errors,
                    "warnings": warnings,
                    "diagnostics": result.diagnostics,
                    "changed_diagnostic_files": result.changed_diagnostic_files,
                    "verification_failed": matches!(result.verification, Some(Err(_))),
                    "verification": match &result.verification {
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
                "[{stamp}] rebuilt {} file(s): +{} -{} ~{} nodes, +{} -{} edges | {} errors, {} warnings",
                result.rebuilt_files.len(),
                result.delta.added_nodes.len(),
                result.delta.removed_nodes.len(),
                result.delta.modified_nodes.len(),
                result.delta.added_edges.len(),
                result.delta.removed_edges.len(),
                errors,
                warnings
            );
            if let Some(Err(msg)) = &result.verification {
                eprintln!("verification FAILED: {msg}");
            }
        }
        let _ = (&watcher, &env_watcher); // keep both watchers alive
    }

    0
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

/// C9-07: write `.specforge/graph.json` in the project root, where a
/// running MCP server (or any agent polling the snapshot) looks for a
/// newer graph than the one it compiled. Written at startup, after every
/// rebuild and after every environment reload.
fn write_freshness_marker(session: &ProjectSession) {
    let marker_dir = session.environment().root.join(".specforge");
    let _ = std::fs::create_dir_all(&marker_dir);
    let marker_tmp = marker_dir.join("graph.json.tmp");
    let marker = marker_dir.join("graph.json");
    let marker_doc = serde_json::json!({
        "updated_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        "nodes": session.graph().node_count(),
        "edges": session.graph().edge_count(),
    });
    if let Ok(mut f) = std::fs::File::create(&marker_tmp) {
        use std::io::Write;
        let _ = f.write_all(
            serde_json::to_string(&marker_doc)
                .expect("marker serialization cannot fail")
                .as_bytes(),
        );
        let _ = f.sync_all();
        let _ = std::fs::rename(&marker_tmp, &marker);
    }
}
