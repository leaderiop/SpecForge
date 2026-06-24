use serde_json::json;
use specforge_registry::{
    HttpRegistryClient, RegistryConfig,
    resolve_version, resolve_from_registry, verify_registry_integrity,
    parse_registries_from_config,
};
use specforge_wasm::{
    install_extension, read_lock_file, write_lock_file,
};
use std::path::Path;

pub fn run(name: Option<&str>, path: &Path, format: &str) -> i32 {
    let lock_path = path.join("specforge.lock");
    let mut lock = match read_lock_file(&lock_path) {
        Ok(l) => l,
        Err(_) => {
            print_error(format, "no lock file found. Run `specforge add` first.", "E-UPD-001");
            return 1;
        }
    };

    let config_path = path.join("specforge.json");
    let registries = load_registries(&config_path);
    let client = HttpRegistryClient::new();

    let entries_to_update: Vec<_> = if let Some(n) = name {
        lock.entries.iter().filter(|e| e.name == n).cloned().collect()
    } else {
        lock.entries.clone()
    };

    if entries_to_update.is_empty() {
        match format {
            "json" => println!("{}", serde_json::to_string_pretty(&json!({"updated": []})).unwrap()),
            _ => println!("no extensions to update"),
        }
        return 0;
    }

    let extensions_dir = path.join(".specforge").join("extensions");
    let cache_dir = path.join(".specforge").join("cache");
    let mut updated = Vec::new();

    for entry in &entries_to_update {
        if entry.source != "registry" {
            continue;
        }

        let registry = match registries.first() {
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
                eprintln!("warning: failed to resolve {}: {}", entry.name, diag.message);
                continue;
            }
        };

        let wasm_bytes = match client.download_wasm(&response.wasm_url) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: failed to download {}: {}", entry.name, e.to_diagnostic().message);
                continue;
            }
        };

        if verify_registry_integrity(&wasm_bytes, &response.sha256).is_err() {
            eprintln!("warning: integrity check failed for {}, skipping", entry.name);
            continue;
        }

        match install_extension(
            &response.name,
            &response.version,
            &wasm_bytes,
            &response.sha256,
            &extensions_dir,
            &cache_dir,
            &mut lock,
            false,
        ) {
            Ok(result) => {
                updated.push(json!({
                    "name": result.name,
                    "from": entry.version,
                    "to": result.version,
                }));
            }
            Err(diag) => {
                eprintln!("warning: failed to install {}: {}", entry.name, diag.message);
            }
        }
    }

    if let Err(diag) = write_lock_file(&lock, &lock_path) {
        print_error(format, &diag.message, &diag.code);
        return 1;
    }

    match format {
        "json" => {
            let output = json!({"updated": updated});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        _ => {
            if updated.is_empty() {
                println!("all extensions are up to date");
            } else {
                println!("updated {} extension(s):", updated.len());
                for u in &updated {
                    println!("  {} {} -> {}", u["name"].as_str().unwrap_or(""), u["from"].as_str().unwrap_or(""), u["to"].as_str().unwrap_or(""));
                }
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

fn print_error(format: &str, message: &str, code: &str) {
    match format {
        "json" => {
            let output = json!({"error": message, "code": code});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        _ => eprintln!("error[{}]: {}", code, message),
    }
}
