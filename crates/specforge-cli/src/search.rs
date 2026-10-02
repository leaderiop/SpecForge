use crate::OutputFormat;
use serde_json::json;
use specforge_registry_client::{HttpRegistryClient, search_registries};
use std::path::Path;

pub fn run(query: &str, path: &Path, format: OutputFormat) -> i32 {
    // No registry configured: fail before any network call (ADR 0004 N1).
    let registries = match specforge_ops::registry::configured(path, "search") {
        Ok(configured) => {
            format.eprint_diagnostics(&configured.diagnostics);
            configured.registries
        }
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };

    let client = HttpRegistryClient::new();
    let (results, diagnostics) = search_registries(query, &registries, &client);

    match format {
        OutputFormat::Json => {
            let output = json!({
                "query": query,
                "results": results.iter().map(|r| json!({
                    "name": r.name,
                    "version": r.version,
                    "description": r.description,
                })).collect::<Vec<_>>(),
                "errors": diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            if results.is_empty() {
                if diagnostics.is_empty() {
                    println!("no extensions found matching '{}'", query);
                } else {
                    for diag in &diagnostics {
                        eprintln!("warning: {}", diag.message);
                    }
                    println!("no extensions found matching '{}'", query);
                }
            } else {
                println!("found {} extension(s) matching '{}':", results.len(), query);
                println!();
                for result in &results {
                    println!("  {} v{}", result.name, result.version);
                    if !result.description.is_empty() {
                        println!("    {}", result.description);
                    }
                }
            }

            for diag in &diagnostics {
                eprintln!("warning: {}", diag.message);
            }
        }
    }

    0
}
