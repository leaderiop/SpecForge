use serde_json::json;
use specforge_registry::{
    HttpRegistryClient, ManifestV2, RegistryConfig,
    publish_to_registry, parse_registries_from_config,
    find_registry_for_specifier,
};
use std::path::Path;

pub fn run(path: &Path, format: &str) -> i32 {
    // Load manifest
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        print_error(format, "no manifest.json found in current directory", "E-PUB-001");
        return 1;
    }

    let manifest_content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            print_error(format, &format!("failed to read manifest.json: {}", e), "E-PUB-001");
            return 1;
        }
    };

    let manifest: ManifestV2 = match serde_json::from_str(&manifest_content) {
        Ok(m) => m,
        Err(e) => {
            print_error(format, &format!("invalid manifest.json: {}", e), "E-PUB-002");
            return 1;
        }
    };

    // Load wasm binary
    let wasm_path = path.join(&manifest.wasm_path);
    if !wasm_path.exists() {
        print_error(
            format,
            &format!("wasm binary not found at '{}'", wasm_path.display()),
            "E-PUB-003",
        );
        return 1;
    }

    let wasm_bytes = match std::fs::read(&wasm_path) {
        Ok(b) => b,
        Err(e) => {
            print_error(format, &format!("failed to read wasm binary: {}", e), "E-PUB-003");
            return 1;
        }
    };

    // Load registry config
    let project_config_path = path.join("specforge.json");
    let registries = load_registries(&project_config_path);

    let registry = match find_registry_for_specifier(&manifest.name, &registries) {
        Some(r) => r,
        None => {
            print_error(format, "no registry configured for this package scope", "R-OPS-001");
            return 1;
        }
    };

    // Publish
    let client = HttpRegistryClient::new();
    match publish_to_registry(&wasm_bytes, &manifest, registry, &client, false) {
        Ok(url) => {
            match format {
                "json" => {
                    let output = json!({
                        "action": "publish",
                        "name": manifest.name,
                        "version": manifest.version,
                        "url": url,
                        "size_bytes": wasm_bytes.len(),
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                _ => {
                    println!("published {} v{}", manifest.name, manifest.version);
                    println!("  url: {}", url);
                    println!("  size: {} bytes", wasm_bytes.len());
                }
            }
            0
        }
        Err(diag) => {
            print_error(format, &diag.message, &diag.code);
            1
        }
    }
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
