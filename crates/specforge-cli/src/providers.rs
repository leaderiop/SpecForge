use crate::OutputFormat;
use crate::outcome::Exit;
use specforge_ops::extension;
use specforge_ops::view::ProjectView;
use std::path::Path;

/// `specforge providers`: the providers `specforge.json` configures, in
/// declaration order, each with the status the scheme registry built from
/// the loaded extensions gives it.
pub fn run(path: &Path, format: OutputFormat) -> Exit {
    let (project, _runtime) = crate::pipeline::compile_project(path);
    let listing = extension::providers(&ProjectView::of(&project));
    let (providers, diagnostics) = (&listing.providers, &listing.diagnostics);

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&listing.to_json()).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            for diagnostic in diagnostics {
                let severity = match diagnostic.severity {
                    specforge_common::Severity::Error => "error",
                    specforge_common::Severity::Warning => "warning",
                    specforge_common::Severity::Info => "info",
                };
                eprintln!("{severity}[{}]: {}", diagnostic.code, diagnostic.message);
            }
            if providers.is_empty() {
                println!("No providers configured.");
                return Exit::Passed;
            }
            println!("Configured providers:");
            println!();
            for provider in providers {
                println!("  {} (extension: {})", provider.alias, provider.extension);
                println!(
                    "    scheme: {} [{}]",
                    provider.scheme,
                    provider.status.as_str()
                );
            }
            println!();
            println!("{} provider(s) configured.", providers.len());
        }
    }

    Exit::Passed
}
