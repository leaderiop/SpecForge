use serde_json::json;
use specforge_registry::{
    HttpRegistryClient, RegistryConfig,
    resolve_from_registry, resolve_version, verify_registry_integrity,
    parse_registries_from_config,
};
use specforge_wasm::{
    install_extension, install_from_local,
    read_lock_file, write_lock_file,
    parse_extension_specifier,
};
use std::path::Path;

pub fn run(specifier: &str, path: &Path, format: &str) -> i32 {
    let parsed = match parse_extension_specifier(specifier) {
        Ok(p) => p,
        Err(diag) => {
            match format {
                "json" => {
                    let output = json!({
                        "error": diag.message,
                        "code": diag.code,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                _ => {
                    eprintln!("error: {}", diag.message);
                    if let Some(suggestion) = &diag.suggestion {
                        eprintln!("  hint: {}", suggestion);
                    }
                }
            }
            return 1;
        }
    };

    match &parsed {
        specforge_wasm::ExtensionSpecifier::Local { path: local_path } => {
            install_local(local_path, path, format)
        }
        specforge_wasm::ExtensionSpecifier::Registry { name, version } => {
            install_from_registry(name, version, path, format)
        }
        specforge_wasm::ExtensionSpecifier::Git { url, .. } => {
            match format {
                "json" => {
                    let output = json!({
                        "error": format!("git source '{}' not yet supported", url),
                        "code": "E-ADD-001",
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                _ => eprintln!("error: git source not yet supported: {}", url),
            }
            1
        }
    }
}

fn install_from_registry(name: &str, version: &str, project_path: &Path, format: &str) -> i32 {
    let config_path = project_path.join("specforge.json");
    let registries = load_registries(&config_path);

    if registries.is_empty() {
        let msg = "no registries configured. Add a \"registries\" section to specforge.json or set a default registry.";
        match format {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&json!({"error": msg, "code": "R-OPS-001"})).unwrap());
            }
            _ => eprintln!("error: {}", msg),
        }
        return 1;
    }

    let client = HttpRegistryClient::new();

    // Resolve version range to a specific version
    let resolved_version = if version == "latest" || version.starts_with('^') || version.starts_with('~') || version.starts_with('>') || version == "*" {
        let registry = registries.first().unwrap();
        match resolve_version(name, version, &client, registry) {
            Ok(v) => v,
            Err(diag) => {
                print_error(format, &diag.message, &diag.code);
                return 1;
            }
        }
    } else {
        version.to_string()
    };

    // Fetch package metadata
    let specifier = format!("{}@{}", name, resolved_version);
    let response = match resolve_from_registry(&specifier, &registries, &client) {
        Ok(r) => r,
        Err(diag) => {
            print_error(format, &diag.message, &diag.code);
            return 1;
        }
    };

    // Download wasm binary
    let wasm_bytes = match client.download_wasm(&response.wasm_url) {
        Ok(bytes) => bytes,
        Err(e) => {
            let diag = e.to_diagnostic();
            print_error(format, &diag.message, &diag.code);
            return 1;
        }
    };

    // Verify integrity
    if let Err(diag) = verify_registry_integrity(&wasm_bytes, &response.sha256) {
        print_error(format, &diag.message, &diag.code);
        return 1;
    }

    // Install
    let extensions_dir = project_path.join(".specforge").join("extensions");
    let cache_dir = project_path.join(".specforge").join("cache");
    let lock_path = project_path.join("specforge.lock");

    let mut lock = read_lock_file(&lock_path).unwrap_or_default();

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
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                print_error(format, &diag.message, &diag.code);
                return 1;
            }

            update_specforge_json(&config_path, &response.name, &resolved_version);

            match format {
                "json" => {
                    let output = json!({
                        "action": "add",
                        "name": result.name,
                        "version": result.version,
                        "sha256": result.wasm_hash,
                        "cached": result.cached,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                _ => {
                    println!("installed {} v{}", result.name, result.version);
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

fn install_local(local_path: &Path, project_path: &Path, format: &str) -> i32 {
    if !local_path.exists() {
        print_error(format, &format!("file not found: {}", local_path.display()), "E028");
        return 1;
    }

    let name = local_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let extensions_dir = project_path.join(".specforge").join("extensions");
    let cache_dir = project_path.join(".specforge").join("cache");
    let lock_path = project_path.join("specforge.lock");

    let mut lock = read_lock_file(&lock_path).unwrap_or_default();

    match install_from_local(
        name,
        "local",
        local_path,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
    ) {
        Ok(result) => {
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                print_error(format, &diag.message, &diag.code);
                return 1;
            }

            match format {
                "json" => {
                    let output = json!({
                        "action": "add",
                        "name": result.name,
                        "version": "local",
                        "sha256": result.wasm_hash,
                        "source": "local",
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                _ => {
                    println!("installed {} from local path", result.name);
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

    let (registries, _diags) = parse_registries_from_config(&content);
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

fn update_specforge_json(config_path: &Path, name: &str, version: &str) {
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return,
    };

    let extensions = json
        .as_object_mut()
        .and_then(|obj| obj.entry("extensions").or_insert_with(|| json!([])).as_array_mut());

    if let Some(exts) = extensions {
        let entry = format!("{}@{}", name, version);
        if !exts.iter().any(|e| {
            e.as_str()
                .is_some_and(|s| s.starts_with(name))
        }) {
            exts.push(json!(entry));
        }
    }

    if let Ok(pretty) = serde_json::to_string_pretty(&json) {
        let _ = std::fs::write(config_path, pretty);
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
