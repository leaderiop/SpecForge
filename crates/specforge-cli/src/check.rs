use specforge_common::Severity;
use specforge_common::inference;
use specforge_validator::{render_diagnostics, diagnostic_summary_detailed};
use std::collections::HashMap;
use std::path::Path;

use crate::pipeline;

const DEFAULT_DENSITY_THRESHOLD: f64 = 0.05;

pub fn run(path: &Path, strict: bool, format: &str, lint_profiles: &[String]) -> i32 {
    let ctx = pipeline::compile(path);

    let mut all_diagnostics = ctx.diagnostics;

    if lint_profiles.iter().any(|p| p == "inferred")
        && let Ok(manifest) = inference::load_inference_manifest(path)
    {
        let config = specforge_common::load_project_config(path);
        let density_threshold = config.inference.density_threshold
            .unwrap_or(DEFAULT_DENSITY_THRESHOLD);
        let infer_diags = inference::compute_inference_diagnostics(
            path, &manifest, density_threshold,
        );
        all_diagnostics.extend(infer_diags);
    }

    // Apply --strict: promote warnings to errors
    if strict {
        for diag in &mut all_diagnostics {
            if diag.severity == Severity::Warning {
                diag.severity = Severity::Error;
            }
        }
    }

    // Output
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&all_diagnostics).unwrap_or_default();
            println!("{}", json);
        }
        _ => {
            if !all_diagnostics.is_empty() {
                let sources = build_source_map(&ctx.spec_root, &ctx.resolved.files);
                let rendered = render_diagnostics(&all_diagnostics, &sources);
                eprint!("{}", rendered);
            }
            eprintln!("{}", diagnostic_summary_detailed(&all_diagnostics));
        }
    }

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
