use serde_json::json;
use specforge_registry::{
    HttpRegistryClient, RegistryConfig,
    search_registries, parse_registries_from_config,
};
use std::path::Path;

pub fn run(query: &str, path: &Path, format: &str) -> i32 {
    let config_path = path.join("specforge.json");
    let registries = load_registries(&config_path);

    let client = HttpRegistryClient::new();
    let (results, diagnostics) = search_registries(query, &registries, &client);

    match format {
        "json" => {
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
        _ => {
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

fn load_registries(config_path: &Path) -> Vec<RegistryConfig> {
    if !config_path.exists() {
        return vec![default_registry()];
    }

    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return vec![default_registry()],
    };

    let (registries, _) = parse_registries_from_config(&content);
    if registries.is_empty() {
        vec![default_registry()]
    } else {
        registries
    }
}

fn default_registry() -> RegistryConfig {
    RegistryConfig {
        alias: "default".to_string(),
        url: "https://registry.specforge.dev/v1".to_string(),
        scope_filter: None,
        default_registry: true,
    }
}
