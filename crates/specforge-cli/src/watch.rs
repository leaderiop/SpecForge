//! `specforge watch` — incremental rebuild loop over the shared pipeline.
//!
//! Cold-builds the project through the standard compile pipeline, then drives
//! [`IncrementalPipeline`] (the same core the LSP uses) from file-watcher
//! events. Rebuilds run through `build_graph_with_config` with the extension
//! registries, so watch diagnostics match `specforge check` byte for byte.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use specforge_common::Severity;
use specforge_graph::{GraphConfig, build_graph_with_config};
use specforge_watch::{ImportDag, IncrementalPipeline, SpecWatcher};

pub fn run(path: &Path, json: bool, verify_incremental: bool) -> i32 {
    // 1. Cold build via the standard compile pipeline (extensions, registries).
    let (mut ctx, mut runtime, mut pipeline) = cold_build(path);
    pipeline.set_verify_incremental(verify_incremental);
    let spec_root: PathBuf =
        std::fs::canonicalize(&ctx.spec_root).unwrap_or_else(|_| ctx.spec_root.clone());

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

    let file_count = ctx.resolved.files.len();
    let diags = full_diagnostics(&pipeline, &ctx, &runtime);
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "event": "ready",
                "spec_root": spec_root.to_string_lossy(),
                "files": file_count,
                "nodes": pipeline.graph().node_count(),
                "edges": pipeline.graph().edge_count(),
                "errors": errors,
                "warnings": warnings,
            })
        );
    } else {
        println!(
            "specforge watch: {} ({} files, {} nodes, {} edges, {} errors, {} warnings)",
            spec_root.display(),
            file_count,
            pipeline.graph().node_count(),
            pipeline.graph().edge_count(),
            errors,
            warnings
        );
        println!("watching for changes (Ctrl-C to stop)");
    }

    // 4. Watch loop: debounced batches from the watcher drive incremental
    //    rebuilds. Tree-sitter trees are retained across rebuilds, so
    //    unchanged subtrees are not re-parsed.
    // Merge both channels: spec-only batches take the incremental path,
    // config/plugin batches force a cold rebuild.
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
        // Extension environment changed: full cold rebuild with a fresh
        // runtime (new/replaced/uninstalled plugins and config). Spec-only
        // batches stay on the incremental path.
        let config_or_plugin = batch
            .iter()
            .any(|e| !matches!(e.kind, specforge_watch::WatchEventKind::Spec));
        if config_or_plugin {
            if debug {
                eprintln!("[watch] reload branch entered");
            }
            let (new_ctx, new_runtime, new_pipeline) = cold_build(path);
            if debug {
                eprintln!("[watch] reload cold_build done");
            }
            pipeline = new_pipeline;
            pipeline.set_verify_incremental(verify_incremental);
            runtime = new_runtime;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "event": "extensions_reloaded",
                        "extensions": new_ctx.manifests.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
                        "files": new_ctx.resolved.files.len(),
                        "nodes": new_ctx.graph.node_count(),
                        "errors": new_ctx.diagnostics.iter().filter(|d| d.severity == Severity::Error).count(),
                    })
                );
            } else {
                println!(
                    "[reload] extension environment changed: {} extension(s), {} file(s)",
                    new_ctx.manifests.len(),
                    new_ctx.resolved.files.len()
                );
            }
            ctx = new_ctx;
            continue;
        }

        let specs: Vec<String> = batch
            .iter()
            .filter(|e| matches!(e.kind, specforge_watch::WatchEventKind::Spec))
            .map(|e| e.path.clone())
            .collect();
        let result = pipeline.rebuild(&specs, |f: &str| {
            std::fs::read_to_string(spec_root.join(f)).ok()
        });

        let diagnostics = full_diagnostics(&pipeline, &ctx, &runtime);
        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();

        // C9-07: write the freshness marker so a running MCP server (or any
        // agent polling the snapshot) can detect the newer graph.
        // The marker lives under the spec root's parent (the project root
        // when it matches); walking up is avoided — the spec root parent is
        // where .specforge/ and specforge.json live in standard layouts.
        let marker_dir = spec_root.parent().unwrap_or(&spec_root).join(".specforge");
        let _ = std::fs::create_dir_all(&marker_dir);
        let marker_tmp = marker_dir.join("graph.json.tmp");
        let marker = marker_dir.join("graph.json");
        let marker_doc = serde_json::json!({
            "updated_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            "nodes": pipeline.graph().node_count(),
            "edges": pipeline.graph().edge_count(),
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

/// Cold-build the project: full compile pipeline + seeded incremental
/// pipeline. Used at startup AND whenever the extension environment changes
/// (specforge.json / .wasm edits) — hardening-plan H3 / R-5.
fn cold_build(
    path: &Path,
) -> (
    crate::pipeline::CompilationContext,
    specforge_component::ComponentRuntime,
    IncrementalPipeline,
) {
    let (ctx, runtime) = crate::pipeline::compile_with_runtime(path);
    let known_extension_keywords: HashMap<String, String> = ctx
        .manifests
        .iter()
        .flat_map(|m| {
            m.entity_kinds
                .iter()
                .map(move |k| (k.keyword.clone(), m.name.clone()))
        })
        .collect();
    let body_parser_kinds: HashSet<String> = ctx
        .manifests
        .iter()
        .flat_map(|m| m.entity_kinds.iter())
        .filter(|k| k.has_body_parser)
        .map(|k| k.keyword.clone())
        .collect();
    let suppressed_parse_error_ranges: Vec<(String, usize, usize)> = ctx
        .resolved
        .files
        .iter()
        .flat_map(|f| f.spec_file.entities.iter())
        .filter(|e| body_parser_kinds.contains(e.kind.raw.as_str()))
        .map(|e| {
            (
                e.span.file.as_str().to_string(),
                e.span.start_line,
                e.span.end_line,
            )
        })
        .collect();
    let single_reference_fields: std::collections::HashSet<(String, String)> = ctx
        .field_registry
        .iter()
        .filter(|(_, _, entry)| {
            entry.field_type == specforge_registry::ManifestFieldType::Reference
        })
        .map(|(kind, field, _)| (kind.to_string(), field.to_string()))
        .collect();
    let graph_config = GraphConfig {
        installed_keywords: ctx.kind_registry.keywords().cloned().collect(),
        known_provider_schemes: HashSet::new(),
        known_extension_keywords,
        bidirectional_pairs: ctx.field_registry.bidirectional_pairs(),
        suppressed_parse_error_ranges,
        single_reference_fields,
        absent_reference_targets: ctx
            .field_registry
            .absent_reference_targets(&ctx.kind_registry),
    };
    let mut dag = ImportDag::new();
    for f in &ctx.resolved.files {
        let imports: Vec<String> = f
            .spec_file
            .imports
            .iter()
            .map(|i| i.path.to_string())
            .collect();
        dag.set_imports_resolved(&f.path, imports);
    }
    let spec_files: Vec<(String, specforge_parser::SpecFile)> = ctx
        .resolved
        .files
        .iter()
        .map(|f| (f.path.clone(), f.spec_file.clone()))
        .collect();
    let all_specs: Vec<specforge_parser::SpecFile> =
        spec_files.iter().map(|(_, sf)| sf.clone()).collect();
    let (graph, build_diagnostics) = build_graph_with_config(&all_specs, &graph_config);
    let pipeline = IncrementalPipeline::from_cold_build(
        spec_files,
        graph,
        dag,
        build_diagnostics,
        graph_config,
    );
    (ctx, runtime, pipeline)
}

/// What `specforge check` reports for the pipeline's graph: its parse and
/// resolution layer plus the registry and extension-rule checks.
fn full_diagnostics(
    pipeline: &IncrementalPipeline,
    ctx: &crate::pipeline::CompilationContext,
    runtime: &specforge_component::ComponentRuntime,
) -> Vec<specforge_common::Diagnostic> {
    let mut diagnostics = pipeline.diagnostics().to_vec();
    diagnostics.extend(specforge_emitter::compile::check_graph(
        pipeline.graph(),
        &specforge_emitter::compile::GraphChecks {
            spec_root: &ctx.spec_root,
            kind_registry: &ctx.kind_registry,
            field_registry: &ctx.field_registry,
            rules: &ctx.extension_rules,
            runtime: Some(runtime),
        },
    ));
    diagnostics
}
