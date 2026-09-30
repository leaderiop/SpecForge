use crate::OutputFormat;
use serde_json::json;
use specforge_ops::extension;
use std::path::Path;

/// `specforge providers`: the providers `specforge.json` configures, in
/// declaration order, each with the status the scheme registry built from
/// the loaded extensions gives it.
pub fn run(path: &Path, format: OutputFormat) -> i32 {
    let ctx = crate::pipeline::compile(path);
    let (providers, diagnostics) = extension::providers(path, &ctx.manifests);

    match format {
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = providers
                .iter()
                .map(|p| {
                    json!({
                        "scheme": p.scheme,
                        "alias": p.alias,
                        "extension": p.extension,
                        "status": p.status.as_str(),
                    })
                })
                .collect();
            let output = json!({
                "providers": items,
                "count": items.len(),
                "diagnostics": diagnostics,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            for diagnostic in &diagnostics {
                let severity = match diagnostic.severity {
                    specforge_common::Severity::Error => "error",
                    specforge_common::Severity::Warning => "warning",
                    specforge_common::Severity::Info => "info",
                };
                eprintln!("{severity}[{}]: {}", diagnostic.code, diagnostic.message);
            }
            if providers.is_empty() {
                println!("No providers configured.");
                return 0;
            }
            println!("Configured providers:");
            println!();
            for provider in &providers {
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

    0
}
