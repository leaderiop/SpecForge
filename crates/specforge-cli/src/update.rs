use crate::OutputFormat;
use serde_json::json;
use specforge_registry::{
    HttpRegistryClient, resolve_from_registry, resolve_version, verify_registry_integrity,
};
use specforge_wasm::{install_extension, read_lock_file, write_lock_file};
use std::path::Path;

pub fn run(
    name: Option<&str>,
    path: &Path,
    format: OutputFormat,
    allow_unsigned: bool,
    assume_yes: bool,
) -> i32 {
    let lock_path = path.join("specforge.lock");
    let mut lock = match read_lock_file(&lock_path) {
        Ok(l) => l,
        Err(_) => {
            print_error(
                format,
                "no lock file found. Run `specforge add` first.",
                "E033",
            );
            return 1;
        }
    };

    let entries_to_update: Vec<_> = if let Some(n) = name {
        lock.entries
            .iter()
            .filter(|e| e.name == n)
            .cloned()
            .collect()
    } else {
        lock.entries.clone()
    };

    if entries_to_update.is_empty() {
        match format {
            OutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&json!({"updated": []})).unwrap()
            ),
            OutputFormat::Human => println!("no extensions to update"),
        }
        return 0;
    }

    // Only registry installs are updated, and only from a configured
    // registry: with none, fail before any network call (ADR 0004 N1).
    let registries = if entries_to_update.iter().any(|e| e.source == "registry") {
        match specforge_ops::registry::configured(path, "update") {
            Ok(registries) => registries,
            Err(error) => {
                format.print_op_error(&error);
                return 1;
            }
        }
    } else {
        Vec::new()
    };
    let client = HttpRegistryClient::new();

    let extensions_dir = path.join(".specforge").join("extensions");
    let mut updated = Vec::new();

    for entry in &entries_to_update {
        if entry.source != "registry" {
            continue;
        }

        let registry =
            match specforge_registry::find_registry_for_specifier(&entry.name, &registries)
                .or_else(|| registries.first())
            {
                Some(r) => r,
                None => continue,
            };

        // Resolve latest version
        let latest = match resolve_version(&entry.name, "*", &client, registry) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if latest == entry.version {
            continue;
        }

        // Fetch and install the newer version
        let specifier = format!("{}@{}", entry.name, latest);
        let response = match resolve_from_registry(&specifier, &registries, &client) {
            Ok(r) => r,
            Err(diag) => {
                eprintln!(
                    "warning: failed to resolve {}: {}",
                    entry.name, diag.message
                );
                continue;
            }
        };

        let wasm_bytes = match client.download_wasm(&response.wasm_url) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "warning: failed to download {}: {}",
                    entry.name,
                    e.to_diagnostic().message
                );
                continue;
            }
        };

        if verify_registry_integrity(&wasm_bytes, &response.sha256).is_err() {
            eprintln!(
                "warning: integrity check failed for {}, skipping",
                entry.name
            );
            continue;
        }

        // Publisher signature + TOFU: a refused package skips the update.
        let trust = match crate::trust_flow::check_and_pin(
            &response.name,
            &response,
            &wasm_bytes,
            allow_unsigned,
            assume_yes,
            format.as_str(),
            None,
        ) {
            Ok(t) => t,
            Err(diag) => {
                eprintln!(
                    "warning: trust check failed for {}: {} (code {}), skipping",
                    response.name, diag.message, diag.code
                );
                continue;
            }
        };

        // Preserve the peers recorded at the original install.
        let peers = lock
            .entries
            .iter()
            .find(|e| e.name == response.name)
            .map(|e| e.peer_dependencies.clone())
            .unwrap_or_default();
        match install_extension(
            &response.name,
            &response.version,
            &wasm_bytes,
            &response.sha256,
            &extensions_dir,
            &mut lock,
            trust.key_id.as_deref(),
            peers,
        ) {
            Ok(result) => {
                updated.push(json!({
                    "name": result.name,
                    "from": entry.version,
                    "to": result.version,
                }));
            }
            Err(diag) => {
                eprintln!(
                    "warning: failed to install {}: {}",
                    entry.name, diag.message
                );
            }
        }
    }

    if let Err(diag) = write_lock_file(&lock, &lock_path) {
        print_error(format, &diag.message, &diag.code);
        return 1;
    }

    match format {
        OutputFormat::Json => {
            let output = json!({"updated": updated});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            if updated.is_empty() {
                println!("all extensions are up to date");
            } else {
                println!("updated {} extension(s):", updated.len());
                for u in &updated {
                    println!(
                        "  {} {} -> {}",
                        u["name"].as_str().unwrap_or(""),
                        u["from"].as_str().unwrap_or(""),
                        u["to"].as_str().unwrap_or("")
                    );
                }
            }
        }
    }

    0
}

fn print_error(format: OutputFormat, message: &str, code: &str) {
    match format {
        OutputFormat::Json => {
            let output = json!({"error": message, "code": code});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => eprintln!("error[{}]: {}", code, message),
    }
}
