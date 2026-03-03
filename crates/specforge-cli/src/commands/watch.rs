use crate::formatter;
use crate::pipeline;
use clap::Args;
use notify::{EventKind, RecursiveMode, Watcher};
use specforge_common::Severity;
use specforge_parser::{parse, SpecFile};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Args)]
pub struct WatchArgs {
    /// Path to watch. Defaults to current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Treat warnings as errors
    #[arg(long)]
    pub strict: bool,
}

/// Run the watch command. Returns exit code (0 = clean exit, 1 = errors on final build).
pub fn run(args: WatchArgs) -> i32 {
    // Initial cold build
    let mut result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    print_watch_header(&result, args.strict);
    formatter::print_diagnostics(&result.diagnostics, &result.sources);
    print_watch_status(&result, args.strict);

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    if ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .is_err()
    {
        eprintln!("specforge: warning: could not set Ctrl+C handler");
    }

    // Determine the spec root directory
    let spec_root = pipeline::find_spec_root(&args.path);
    let watch_dir = spec_root
        .as_ref()
        .map(|p| p.parent().unwrap_or(args.path.as_path()))
        .unwrap_or(args.path.as_path());

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("specforge: error creating file watcher: {e}");
            return 1;
        }
    };

    if let Err(e) = watcher.watch(watch_dir, RecursiveMode::Recursive) {
        eprintln!("specforge: error watching directory: {e}");
        return 1;
    }

    eprintln!(
        "\nspecforge: watching {} for changes (Ctrl+C to stop)",
        watch_dir.display()
    );

    // Watch loop
    let debounce_duration = Duration::from_millis(50);

    while running.load(Ordering::SeqCst) {
        // Wait for an event (with timeout to check running flag)
        let event = match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(e) => e,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        // Collect the initial event paths
        let mut changed_paths: Vec<PathBuf> = Vec::new();
        collect_spec_paths(&event, &mut changed_paths);

        // Debounce: drain all events within the debounce window
        let debounce_start = Instant::now();
        while debounce_start.elapsed() < debounce_duration {
            match rx.recv_timeout(debounce_duration - debounce_start.elapsed()) {
                Ok(event) => collect_spec_paths(&event, &mut changed_paths),
                Err(_) => break,
            }
        }

        if changed_paths.is_empty() {
            continue;
        }

        // Check running flag again before rebuilding
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let changed_strs: Vec<String> = changed_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // Discover current spec files (handles created/deleted files)
        let current_spec_files = pipeline::discover_spec_files(watch_dir);
        let current_paths: std::collections::HashSet<String> = current_spec_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        result = incremental_rebuild(&result, &changed_strs, watch_dir, |path| {
            if current_paths.contains(path) {
                std::fs::read_to_string(path).ok()
            } else {
                None
            }
        });

        // Remove sources for files that no longer exist
        result.sources.retain(|k, _| current_paths.contains(k));

        // Print results
        eprintln!("\n--- rebuild triggered ---\n");
        print_watch_header(&result, args.strict);
        formatter::print_diagnostics(&result.diagnostics, &result.sources);
        print_watch_status(&result, args.strict);
    }

    eprintln!("\nspecforge: watch stopped");
    exit_code(&result, args.strict)
}

/// Perform an incremental rebuild: re-parse only invalidated files, then re-resolve
/// and re-validate the full set. Returns the updated pipeline result.
///
/// This is the core logic extracted from the watch loop to support testing.
pub fn incremental_rebuild(
    prev: &pipeline::PipelineResult,
    changed_files: &[String],
    spec_root_dir: &std::path::Path,
    read_source: impl Fn(&str) -> Option<String>,
) -> pipeline::PipelineResult {
    // Compute invalidation set
    let mut invalidated_files = std::collections::HashSet::new();
    for changed in changed_files {
        invalidated_files.extend(prev.file_graph.invalidation_set(changed));
        invalidated_files.insert(changed.clone());
    }

    // Keep unchanged files as-is; re-parse only invalidated files
    let mut updated_files: Vec<SpecFile> = prev
        .files
        .iter()
        .filter(|f| !invalidated_files.contains(&f.path))
        .cloned()
        .collect();

    let mut updated_sources = prev.sources.clone();

    for file_path in &invalidated_files {
        if let Some(source) = read_source(file_path) {
            let parsed = parse(&source, file_path);
            updated_sources.insert(file_path.clone(), source);
            updated_files.push(parsed);
        } else {
            // File was deleted
            updated_sources.remove(file_path.as_str());
        }
    }

    // Re-run resolve → build graph → validate on the full file set
    let spec_root_str = spec_root_dir.to_string_lossy().to_string();
    let resolved = specforge_resolver::resolve_with_config(
        updated_files,
        &spec_root_str,
        prev.external_config.clone(),
    );
    let registry = pipeline::build_field_registry(&resolved.config);
    let graph_result = specforge_graph::build_graph(&resolved.files, &registry);
    let validation_bag = specforge_validator::validate(
        &resolved.files,
        &graph_result.graph,
        &resolved.config,
        spec_root_dir,
        &registry,
    );

    let mut all_diagnostics = resolved.diagnostics.sorted();
    all_diagnostics.extend(validation_bag.sorted());
    all_diagnostics.sort();

    pipeline::PipelineResult {
        files: resolved.files,
        graph: graph_result.graph,
        file_index: graph_result.file_index,
        file_graph: resolved.file_graph,
        diagnostics: all_diagnostics,
        config: resolved.config,
        sources: updated_sources,
        field_registry: registry,
        external_config: prev.external_config.clone(),
        wasm_runtime: None,
        kind_registry: specforge_common::KindRegistry::with_builtins(),
    }
}

fn collect_spec_paths(event: &notify::Event, paths: &mut Vec<PathBuf>) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
            for path in &event.paths {
                if path
                    .extension()
                    .is_some_and(|ext| ext == "spec")
                {
                    paths.push(path.clone());
                }
            }
        }
        _ => {}
    }
}

fn print_watch_header(result: &pipeline::PipelineResult, strict: bool) {
    let error_count = result
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .count();
    let warning_count = result
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Warning)
        .count();
    let info_count = result
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Info)
        .count();

    let _ = (error_count, warning_count, info_count, strict);
}

fn print_watch_status(result: &pipeline::PipelineResult, strict: bool) {
    let error_count = result
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .count();
    let warning_count = result
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Warning)
        .count();

    if error_count == 0 && (!strict || warning_count == 0) {
        eprintln!("specforge: compilation successful");
    } else {
        eprintln!("specforge: compilation failed ({error_count} errors, {warning_count} warnings)");
    }
}

fn exit_code(result: &pipeline::PipelineResult, strict: bool) -> i32 {
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity() == Severity::Error);
    let has_warnings = result
        .diagnostics
        .iter()
        .any(|d| d.severity() == Severity::Warning);

    if has_errors || (strict && has_warnings) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_resolver::FileGraph;

    /// Helper: build a PipelineResult from a set of in-memory source files.
    fn build_pipeline(sources: &[(&str, &str)]) -> pipeline::PipelineResult {
        let mut parsed_files = Vec::new();
        let mut source_map = std::collections::HashMap::new();
        for (path, source) in sources {
            let parsed = parse(source, path);
            source_map.insert(path.to_string(), source.to_string());
            parsed_files.push(parsed);
        }

        let resolved = specforge_resolver::resolve(parsed_files, "test");
        let registry = pipeline::build_field_registry(&resolved.config);
        let graph_result = specforge_graph::build_graph(&resolved.files, &registry);
        let validation_bag = specforge_validator::validate(
            &resolved.files,
            &graph_result.graph,
            &resolved.config,
            std::path::Path::new("."),
            &registry,
        );

        let mut all_diagnostics = resolved.diagnostics.sorted();
        all_diagnostics.extend(validation_bag.sorted());
        all_diagnostics.sort();

        pipeline::PipelineResult {
            files: resolved.files,
            graph: graph_result.graph,
            file_index: graph_result.file_index,
            file_graph: resolved.file_graph,
            diagnostics: all_diagnostics,
            config: resolved.config,
            sources: source_map,
            field_registry: registry,
            external_config: None,
            wasm_runtime: None,
            kind_registry: specforge_common::KindRegistry::with_builtins(),
        }
    }

    // --- emit_incremental_diagnostics tests ---

    #[test]
    fn incremental_diagnostics_from_changed_file_are_refreshed() {
        // Initial state: behavior references a nonexistent invariant → E001
        let initial = build_pipeline(&[
            ("specforge.spec", "spec \"test\" { version \"1.0\" }"),
            ("a.spec", "behavior beh_a { invariants [nonexistent_inv] }"),
        ]);
        assert!(
            initial.diagnostics.iter().any(|d| format!("{}", d.code) == "E001"),
            "Initial build should have E001 for dangling reference"
        );

        // Fix the file: remove the dangling reference
        let fixed_source = "behavior beh_a { contract \"ok\" }";
        let result = incremental_rebuild(&initial, &["a.spec".to_string()], std::path::Path::new("."), |path| {
            if path == "a.spec" {
                Some(fixed_source.to_string())
            } else {
                initial.sources.get(path).cloned()
            }
        });

        // E001 should be gone — diagnostics from changed file were refreshed
        let has_e001 = result.diagnostics.iter().any(|d| format!("{}", d.code) == "E001");
        assert!(
            !has_e001,
            "After fixing a.spec, E001 should be gone.\nDiagnostics: {:?}",
            result.diagnostics.iter().map(|d| format!("{}", d.code)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn incremental_diagnostics_from_unchanged_file_are_preserved() {
        // Two files, both with issues:
        // a.spec: orphan behavior (W001)
        // b.spec: orphan behavior (W001) + dangling ref (E001)
        let initial = build_pipeline(&[
            ("specforge.spec", "spec \"test\" { version \"1.0\" }"),
            ("a.spec", "behavior orphan_a { contract \"test\" }"),
            ("b.spec", "behavior orphan_b { invariants [does_not_exist] }"),
        ]);

        // b.spec should have E001 (dangling ref)
        let b_has_e001 = initial.diagnostics.iter().any(|d| {
            format!("{}", d.code) == "E001" && d.span.file.contains("b.spec")
        });
        assert!(b_has_e001, "b.spec should have E001 initially");

        // Change only a.spec (give it a contract but it's still orphan)
        let result = incremental_rebuild(
            &initial,
            &["a.spec".to_string()],
            std::path::Path::new("."),
            |path| {
                if path == "a.spec" {
                    Some("behavior orphan_a { contract \"updated\" }".to_string())
                } else {
                    initial.sources.get(path).cloned()
                }
            },
        );

        // b.spec's E001 should still be present — unchanged file diagnostics preserved
        let b_still_has_e001 = result.diagnostics.iter().any(|d| {
            format!("{}", d.code) == "E001" && d.span.file.contains("b.spec")
        });
        assert!(
            b_still_has_e001,
            "b.spec's E001 should be preserved after changing only a.spec.\nDiagnostics: {:?}",
            result.diagnostics.iter().map(|d| format!("{} ({})", d.code, d.span.file)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn incremental_rebuild_matches_cold_rebuild() {
        // Initial state with a dangling ref
        let initial = build_pipeline(&[
            ("specforge.spec", "spec \"test\" { version \"1.0\" }"),
            ("a.spec", "behavior beh_a { invariants [missing_inv] }"),
            ("b.spec", "behavior beh_b { contract \"ok\" }"),
        ]);

        // Fix a.spec incrementally
        let fixed_a = "behavior beh_a { contract \"fixed\" }";
        let incremental_result = incremental_rebuild(
            &initial,
            &["a.spec".to_string()],
            std::path::Path::new("."),
            |path| {
                if path == "a.spec" {
                    Some(fixed_a.to_string())
                } else {
                    initial.sources.get(path).cloned()
                }
            },
        );

        // Cold rebuild with the same final sources
        let cold_result = build_pipeline(&[
            ("specforge.spec", "spec \"test\" { version \"1.0\" }"),
            ("a.spec", fixed_a),
            ("b.spec", "behavior beh_b { contract \"ok\" }"),
        ]);

        // Diagnostic sets should match
        let mut incremental_codes: Vec<String> = incremental_result
            .diagnostics
            .iter()
            .map(|d| format!("{}:{}", d.code, d.span.file))
            .collect();
        incremental_codes.sort();

        let mut cold_codes: Vec<String> = cold_result
            .diagnostics
            .iter()
            .map(|d| format!("{}:{}", d.code, d.span.file))
            .collect();
        cold_codes.sort();

        assert_eq!(
            incremental_codes, cold_codes,
            "Incremental rebuild should produce identical diagnostics to cold rebuild.\n\
             Incremental: {incremental_codes:?}\nCold: {cold_codes:?}"
        );
    }

    #[test]
    fn collect_spec_paths_filters_non_spec() {
        let event = notify::Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![
                PathBuf::from("a.spec"),
                PathBuf::from("b.txt"),
                PathBuf::from("c.spec"),
            ],
            attrs: Default::default(),
        };
        let mut paths = Vec::new();
        collect_spec_paths(&event, &mut paths);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&PathBuf::from("a.spec")));
        assert!(paths.contains(&PathBuf::from("c.spec")));
    }

    #[test]
    fn collect_spec_paths_ignores_access_events() {
        let event = notify::Event {
            kind: EventKind::Access(notify::event::AccessKind::Read),
            paths: vec![PathBuf::from("a.spec")],
            attrs: Default::default(),
        };
        let mut paths = Vec::new();
        collect_spec_paths(&event, &mut paths);
        assert!(paths.is_empty());
    }

    #[test]
    fn invalidation_set_triggers_reparse() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "c.spec");

        // If c.spec changes, a, b, c should all be invalidated
        let set = fg.invalidation_set("c.spec");
        assert_eq!(set.len(), 3);
        assert!(set.contains("a.spec"));
        assert!(set.contains("b.spec"));
        assert!(set.contains("c.spec"));
    }

    #[test]
    fn non_spec_file_ignored() {
        let event = notify::Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![
                PathBuf::from("readme.txt"),
                PathBuf::from("notes.md"),
            ],
            attrs: Default::default(),
        };
        let mut paths = Vec::new();
        collect_spec_paths(&event, &mut paths);
        assert!(paths.is_empty());
    }

    #[test]
    fn file_creation_detected() {
        let event = notify::Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![PathBuf::from("new.spec")],
            attrs: Default::default(),
        };
        let mut paths = Vec::new();
        collect_spec_paths(&event, &mut paths);
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn file_deletion_detected() {
        let event = notify::Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![PathBuf::from("old.spec")],
            attrs: Default::default(),
        };
        let mut paths = Vec::new();
        collect_spec_paths(&event, &mut paths);
        assert_eq!(paths.len(), 1);
    }
}
