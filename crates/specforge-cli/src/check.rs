use specforge_project::DiagnosticPolicy;
use specforge_validator::{diagnostic_summary_detailed, render_diagnostics_colored};
use std::collections::HashMap;
use std::path::Path;

use crate::OutputFormat;
use crate::pipeline;

pub fn run(path: &Path, strict: bool, format: OutputFormat, lint_profiles: &[String]) -> i32 {
    let ctx = pipeline::compile(path);

    // --lint profiles add their diagnostics, --strict promotes warnings.
    let policy = DiagnosticPolicy {
        strict,
        lint_profiles: lint_profiles.to_vec(),
    };
    let all_diagnostics = policy.apply(path, ctx.diagnostics);

    // Output
    match format {
        OutputFormat::Json => {
            let entries = specforge_emitter::diagnostics_json(&all_diagnostics);
            let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Human => {
            let color = crate::color::stderr();
            if !all_diagnostics.is_empty() {
                let sources = build_source_map(&ctx.spec_root, &ctx.resolved.files);
                let rendered = render_diagnostics_colored(&all_diagnostics, &sources, color);
                eprint!("{}", rendered);
            }
            eprintln!("{}", diagnostic_summary_detailed(&all_diagnostics, color));
        }
    }

    // Strict already promoted warnings: errors alone decide.
    specforge_emitter::compute_exit_code(&all_diagnostics)
}

pub(crate) fn build_source_map(
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
