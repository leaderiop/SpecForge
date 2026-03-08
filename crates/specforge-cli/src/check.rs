use specforge_common::Severity;
use specforge_graph::build_graph;
use specforge_resolver::resolve_project;
use specforge_validator::{
    diagnostic_summary, render_diagnostics, validate, ValidatorConfig,
};
use std::collections::HashMap;
use std::path::Path;

pub fn run(path: &Path, strict: bool, format: &str) -> i32 {
    // 1. Resolve project
    let resolved = resolve_project(path);

    // 2. Build graph
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (graph, build_diagnostics) = build_graph(&spec_files);

    // 3. Run validation
    let _config = ValidatorConfig {
        spec_root: path.to_path_buf(),
        ..Default::default()
    };
    let validation_diagnostics = validate(&graph);

    // 4. Collect all diagnostics
    let mut all_diagnostics = resolved.diagnostics;
    all_diagnostics.extend(build_diagnostics);
    all_diagnostics.extend(validation_diagnostics);

    // 5. Apply --strict: promote warnings to errors
    if strict {
        for diag in &mut all_diagnostics {
            if diag.severity == Severity::Warning {
                diag.severity = Severity::Error;
            }
        }
    }

    // 6. Output
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&all_diagnostics).unwrap_or_default();
            println!("{}", json);
        }
        _ => {
            if !all_diagnostics.is_empty() {
                // Build source map for rendering
                let sources = build_source_map(path, &resolved.files);
                let rendered = render_diagnostics(&all_diagnostics, &sources);
                eprint!("{}", rendered);
            }
            eprintln!("{}", diagnostic_summary(&all_diagnostics));
        }
    }

    // 7. Exit code
    let has_errors = all_diagnostics.iter().any(|d| d.severity == Severity::Error);
    if has_errors { 1 } else { 0 }
}

fn build_source_map(
    spec_root: &Path,
    files: &[specforge_resolver::ResolvedFile],
) -> HashMap<String, String> {
    let mut sources = HashMap::new();
    for file in files {
        let full_path = spec_root.join(&file.path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            sources.insert(file.path.clone(), content);
        }
    }
    sources
}
