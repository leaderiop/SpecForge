use crate::OutputFormat;
use serde_json::json;
use specforge_common::{Diagnostic, Severity};
use specforge_registry::{
    HttpRegistryClient, RegistryConfig, parse_registries_from_config, resolve_from_registry,
    resolve_version, verify_registry_integrity,
};
use specforge_wasm::{
    collect_peer_requirers, install_extension, install_from_local, parse_extension_specifier,
    read_lock_file, write_lock_file,
};
use std::path::Path;

pub fn run(
    specifier: &str,
    path: &Path,
    format: OutputFormat,
    allow_unsigned: bool,
    assume_yes: bool,
) -> i32 {
    if let Some(name) = crate::builtins::builtin_name(specifier) {
        return enable_builtin(name, path, format);
    }

    let parsed = match parse_extension_specifier(specifier) {
        Ok(p) => p,
        Err(diag) => {
            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "error": diag.message,
                        "code": diag.code,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
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
            install_from_registry(name, version, path, format, allow_unsigned, assume_yes)
        }
        specforge_wasm::ExtensionSpecifier::Git { url, .. } => {
            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "error": format!("git source '{}' not yet supported", url),
                        "code": "E-ADD-001",
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => eprintln!("error: git source not yet supported: {}", url),
            }
            1
        }
    }
}

/// Builtins are embedded in the binary: enabling one only edits specforge.json.
fn enable_builtin(name: &str, project_path: &Path, format: OutputFormat) -> i32 {
    // Required builtin peers come first, so they're enabled before the
    // extension that builds on them. An already-enabled extension is left
    // exactly as it is.
    let already = crate::builtins::enabled(project_path).contains(&name);
    let peers = if already {
        Vec::new()
    } else {
        crate::builtins::required_peers(name)
    };
    let mut peers_added = Vec::new();
    for peer in &peers {
        match crate::builtins::enable(project_path, peer) {
            Ok(true) => peers_added.push(*peer),
            Ok(false) => {}
            Err(message) => {
                print_error(format, &message, "E032");
                return 1;
            }
        }
    }
    let added = match crate::builtins::enable(project_path, name) {
        Ok(added) => added,
        Err(message) => {
            print_error(format, &message, "E032");
            return 1;
        }
    };
    match format {
        OutputFormat::Json => {
            let output = json!({
                "action": "add",
                "name": name,
                "source": "builtin",
                "changed": added,
                "peers_enabled": peers_added,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            for peer in &peers_added {
                println!("enabled builtin {peer} (required by {name})");
            }
            if added {
                println!("enabled builtin {}", name);
            } else {
                println!("{} is already enabled", name);
            }
        }
    }
    0
}

fn install_from_registry(
    name: &str,
    version: &str,
    project_path: &Path,
    format: OutputFormat,
    allow_unsigned: bool,
    assume_yes: bool,
) -> i32 {
    let config_path = project_path.join("specforge.json");
    let registries = load_registries(&config_path);

    if registries.is_empty() {
        let msg = "no registries configured. Add a \"registries\" section to specforge.json or set a default registry.";
        match format {
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"error": msg, "code": "R-OPS-001"}))
                        .unwrap()
                );
            }
            OutputFormat::Human => eprintln!("error: {}", msg),
        }
        return 1;
    }

    let client = HttpRegistryClient::new();

    // Resolve version range to a specific version
    let resolved_version = if version == "latest"
        || version.starts_with('^')
        || version.starts_with('~')
        || version.starts_with('>')
        || version == "*"
    {
        let registry = specforge_registry::find_registry_for_specifier(name, &registries)
            .unwrap_or_else(|| registries.first().unwrap());
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

    // Verify publisher signature and apply the TOFU pin policy
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
            print_error(format, &diag.message, &diag.code);
            return 1;
        }
    };

    // Install
    let extensions_dir = project_path.join(".specforge").join("extensions");
    let lock_path = project_path.join("specforge.lock");

    let mut lock = read_lock_file(&lock_path).unwrap_or_default();

    // Record the peers the package declares so doctor can verify them later.
    let peer_dependencies: Vec<specforge_registry::PeerDependency> =
        serde_json::from_str::<specforge_registry::ManifestV2>(&response.manifest)
            .map(|m| m.peer_dependencies)
            .unwrap_or_default();

    // C8-07: a version diamond — this package and some already-locked
    // package both depend on the same peer at incompatible ranges — must be
    // caught and unified here, not left for `doctor` to discover after the
    // fact. If the currently locked version already satisfies this
    // package's range there is nothing to do; that is the common case.
    for peer in &peer_dependencies {
        let Some(locked_peer) = lock.entries.iter().find(|e| e.name == peer.name) else {
            continue;
        };
        let satisfied = match (
            semver::VersionReq::parse(&peer.version),
            semver::Version::parse(&locked_peer.version),
        ) {
            (Ok(req), Ok(v)) => req.matches(&v),
            // Malformed ranges/versions are reported by validate_peer_dependencies (W062);
            // don't block install on them here.
            _ => true,
        };
        if satisfied {
            continue;
        }

        let requirers =
            collect_peer_requirers(&lock, &peer.name, Some((&response.name, &peer.version)));
        let peer_registry =
            specforge_registry::find_registry_for_specifier(&peer.name, &registries)
                .unwrap_or_else(|| registries.first().unwrap());

        let diag = match specforge_registry::resolve_diamond(
            &peer.name,
            &requirers,
            &client,
            peer_registry,
        ) {
            Ok(unified) if unified == locked_peer.version => {
                // Unreachable in practice (unified would have satisfied `req` above),
                // but fall through safely rather than panic if it ever happens.
                continue;
            }
            Ok(unified) => Diagnostic {
                code: "R-RES-006".to_string(),
                severity: Severity::Error,
                message: format!(
                    "version diamond: '{}' requires peer '{}' {} but {} is locked; {} {} would satisfy every requirer",
                    response.name, peer.name, peer.version, locked_peer.version, peer.name, unified
                ),
                span: None,
                suggestion: Some(format!(
                    "no command pins peer versions yet; manually reinstall '{}' at {} (or a version satisfying every requirer), then retry add",
                    peer.name, unified
                )),
            },
            Err(diag) => diag,
        };
        print_error(format, &diag.message, &diag.code);
        return 1;
    }

    match install_extension(
        &response.name,
        &response.version,
        &wasm_bytes,
        &response.sha256,
        &extensions_dir,
        &mut lock,
        trust.key_id.as_deref(),
        peer_dependencies,
    ) {
        Ok(result) => {
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                print_error(format, &diag.message, &diag.code);
                return 1;
            }

            let entry = format!("{}@{}", response.name, resolved_version);
            if let Err(e) =
                specforge_ops::config::add_extension(project_path, &response.name, &entry)
            {
                eprintln!("warning: {} was not enabled: {}", response.name, e.message);
            }

            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "action": "add",
                        "name": result.name,
                        "version": result.version,
                        "sha256": result.wasm_hash,
                        "key_id": trust.key_id,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
                    println!("installed {} v{}", result.name, result.version);
                    match &trust.key_id {
                        Some(key_id) => println!("  signed by key: {}", key_id),
                        None => println!("  unsigned"),
                    }
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

fn install_local(local_path: &Path, project_path: &Path, format: OutputFormat) -> i32 {
    if !local_path.exists() {
        print_error(
            format,
            &format!("file not found: {}", local_path.display()),
            "E054",
        );
        return 1;
    }

    let name = local_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let extensions_dir = project_path.join(".specforge").join("extensions");
    let lock_path = project_path.join("specforge.lock");

    let mut lock = read_lock_file(&lock_path).unwrap_or_default();

    match install_from_local(name, "local", local_path, &extensions_dir, &mut lock) {
        Ok(result) => {
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                print_error(format, &diag.message, &diag.code);
                return 1;
            }

            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "action": "add",
                        "name": result.name,
                        "version": "local",
                        "sha256": result.wasm_hash,
                        "source": "local",
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
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

fn print_error(format: OutputFormat, message: &str, code: &str) {
    match format {
        OutputFormat::Json => {
            let output = json!({"error": message, "code": code});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => eprintln!("error[{}]: {}", code, message),
    }
}
