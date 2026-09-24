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

pub fn run(path: &Path, json: bool) -> i32 {
    // 1. Cold build via the standard compile pipeline (extensions, registries).
    let ctx = crate::pipeline::compile(path);
    let spec_root: PathBuf =
        std::fs::canonicalize(&ctx.spec_root).unwrap_or_else(|_| ctx.spec_root.clone());

    // 2. GraphConfig mirroring the emitter's cold build so incremental
    //    rebuilds agree with `specforge check` (I004 keyword hints,
    //    bidirectional edge pairs for cycle detection, installed kinds).
    let known_extension_keywords: HashMap<String, String> = ctx
        .manifests
        .iter()
        .flat_map(|m| {
            m.entity_kinds
                .iter()
                .map(move |k| (k.keyword.clone(), m.name.clone()))
        })
        .collect();
    // Body-parser entity ranges: their E001s are suppressed (extension-owned
    // syntax), matching the emitter's cold build.
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
    };

    // 3. Seed the pipeline: import DAG from resolved files, graph rebuilt
    //    through build_graph_with_config (same config as future rebuilds).
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
    let mut pipeline = IncrementalPipeline::from_cold_build(
        spec_files,
        graph,
        dag,
        build_diagnostics,
        graph_config,
    );

    // Start watching before announcing readiness: a client that writes on
    // seeing "ready" must never race a watcher that does not exist yet.
    let (tx, rx) = mpsc::channel::<Vec<String>>();
    let watcher = match SpecWatcher::new(&spec_root, tx) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let file_count = ctx.resolved.files.len();
    let diags = pipeline.diagnostics();
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
    for batch in rx {
        let result = pipeline.rebuild(&batch, |f: &str| {
            std::fs::read_to_string(spec_root.join(f)).ok()
        });

        let errors = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();

        if json {
            println!(
                "{}",
                serde_json::json!({
                    "event": "rebuilt",
                    "changed": batch,
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
        let _ = &watcher; // keep the watcher alive for the loop's lifetime
    }

    0
}
