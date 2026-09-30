use crate::OutputFormat;
use serde_json::json;
use std::path::Path;

pub fn run(path: &Path, format: OutputFormat) -> i32 {
    let config_path = path.join("specforge.json");

    // Read specforge.json
    let config: serde_json::Value = match std::fs::read_to_string(&config_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => {
                match format {
                    OutputFormat::Json => {
                        let output = json!({ "providers": [], "count": 0 });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&output).expect("serialize JSON output")
                        );
                    }
                    OutputFormat::Human => {
                        eprintln!("warning: specforge.json is not valid JSON");
                        println!("No providers configured.");
                    }
                }
                return 0;
            }
        },
        Err(_) => {
            match format {
                OutputFormat::Json => {
                    let output = json!({ "providers": [], "count": 0 });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output).expect("serialize JSON output")
                    );
                }
                OutputFormat::Human => {
                    println!("No providers configured (no specforge.json found).");
                }
            }
            return 0;
        }
    };

    // Extract providers array
    let providers = match config.get("providers").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => Vec::new(),
    };

    match format {
        OutputFormat::Json => {
            let output = json!({
                "providers": providers,
                "count": providers.len(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            if providers.is_empty() {
                println!("No providers configured.");
            } else {
                println!("Configured providers:");
                println!();
                for provider in &providers {
                    let alias = provider
                        .get("alias")
                        .and_then(|v| v.as_str())
                        .unwrap_or("<unnamed>");
                    let extension = provider
                        .get("extension")
                        .and_then(|v| v.as_str())
                        .unwrap_or("<unknown>");
                    let list = |key: &str| -> Vec<&str> {
                        provider
                            .get(key)
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|s| s.as_str()).collect())
                            .unwrap_or_default()
                    };
                    let schemes = list("schemes");
                    let kinds = list("kinds");

                    println!("  {} (extension: {})", alias, extension);
                    if !schemes.is_empty() {
                        println!("    schemes: {}", schemes.join(", "));
                    }
                    if !kinds.is_empty() {
                        println!("    kinds: {}", kinds.join(", "));
                    }
                }
                println!();
                println!("{} provider(s) configured.", providers.len());
            }
        }
    }

    0
}
