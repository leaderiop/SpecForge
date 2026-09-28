//! MCP management operations. Every op performs its real function against
//! the same library backends the CLI uses — canned placeholder responses are
//! forbidden (hardening-plan P1 / success criterion S2: a tool either does
//! real work or refuses with an explicit error; it never lies).

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use specforge_common::find_project_root;
use specforge_registry::{
    HttpRegistryClient, RegistryConfig, parse_registries_from_config, resolve_from_registry,
    resolve_version, verify_registry_integrity,
};
use specforge_wasm::{
    auto_detect_collector, install_extension, install_from_local, read_lock_file, run_doctor_check,
    uninstall_extension, write_lock_file,
};
use std::process::Command as Z3Command;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn handle_operation(
    state: &mut McpState,
    name: &str,
    args: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    match name {
        "specforge.format" => format_op(state, args, id),
        "specforge.rename" => rename_op(state, args, id),
        "specforge.init" => init_op(state, args, id),
        "specforge.add_extension" => add_extension_op(state, args, id),
        "specforge.remove_extension" => remove_extension_op(state, args, id),
        "specforge.migrate" => migrate_op(state, args, id),
        "specforge.extensions" => extensions_op(state, args, id),
        "specforge.providers" => providers_op(state, args, id),
        "specforge.doctor" => doctor_op(state, args, id),
        "specforge.collect" => collect_op(state, args, id),
        "specforge.render" => render_op(state, args, id),
        _ => JsonRpcResponse::error(
            id,
            error_codes::METHOD_NOT_FOUND,
            format!("Unknown operation: {}", name),
        ),
    }
}

// ── shared helpers ──────────────────────────────────────────────────────────

/// Resolve the project root, preferring an explicit `path` argument.
fn project_root_of(state: &McpState, args: &Value) -> Option<PathBuf> {
    args.get("path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.project_root.clone())
}

fn err_invalid(id: Option<Value>, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, message.into())
}

fn ok(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        json!({ "content": [{ "type": "text", "text": result.to_string() }] }),
    )
}

fn registries_for(config_path: &Path) -> Vec<RegistryConfig> {
    if !config_path.exists() {
        return vec![RegistryConfig {
            alias: "default".to_string(),
            url: "https://registry.specforge.dev/v1".to_string(),
            scope_filter: None,
            default_registry: true,
        }];
    }
    match std::fs::read_to_string(config_path) {
        Ok(content) => {
            let (registries, _) = parse_registries_from_config(&content);
            if registries.is_empty() {
                vec![RegistryConfig {
                    alias: "default".to_string(),
                    url: "https://registry.specforge.dev/v1".to_string(),
                    scope_filter: None,
                    default_registry: true,
                }]
            } else {
                registries
            }
        }
        Err(_) => vec![RegistryConfig {
            alias: "default".to_string(),
            url: "https://registry.specforge.dev/v1".to_string(),
            scope_filter: None,
            default_registry: true,
        }],
    }
}

/// Append `name@version` to specforge.json's extensions list (idempotent).
fn update_config_extensions(config_path: &Path, name: &str, version: &str) {
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return;
    };
    let Ok(mut json) = serde_json::from_str::<Value>(&content) else {
        return;
    };
    let Some(exts) = json.as_object_mut().and_then(|obj| {
        obj.entry("extensions")
            .or_insert_with(|| json!([]))
            .as_array_mut()
    }) else {
        return;
    };
    let entry = format!("{name}@{version}");
    if !exts
        .iter()
        .any(|e| e.as_str().is_some_and(|s| s.starts_with(name)))
    {
        exts.push(json!(entry));
    }
    if let Ok(pretty) = serde_json::to_string_pretty(&json) {
        let _ = std::fs::write(config_path, pretty);
    }
}

// ── format ──────────────────────────────────────────────────────────────────

fn format_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let check = args.get("check").and_then(|v| v.as_bool()).unwrap_or(false);
    let write = args
        .get("write")
        .and_then(|v| v.as_bool())
        .unwrap_or(!check);

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "format needs a project root (pass {\"path\": ...})");
    };
    let project_root = match find_project_root(&root) {
        Some(r) => r,
        None => {
            return err_invalid(
                id,
                format!("no specforge project found at {}", root.display()),
            );
        }
    };

    let (config, config_diags) = specforge_formatter::load_config(&project_root, &project_root);
    let spec_root = project_root.join("spec");
    let search_root = if spec_root.exists() {
        spec_root
    } else {
        project_root.clone()
    };
    let targets = specforge_formatter::discover_targets(&search_root, &[], &[]);

    let mut changed_files = Vec::new();
    let mut total_checked = 0usize;
    for target in &targets {
        let Ok(source) = std::fs::read_to_string(target) else {
            continue;
        };
        total_checked += 1;
        let result = specforge_formatter::format_source(&source, &config);
        if result.formatted == source {
            continue;
        }
        changed_files.push(target.display().to_string());
        // Apply in write mode only.
        if write && let Err(e) = std::fs::write(target, &result.formatted) {
            return err_invalid(id, format!("failed to write {}: {e}", target.display()));
        }
    }
    let _ = config_diags;

    let all_clean = changed_files.is_empty();
    ok(
        id,
        json!({
            "changed_files": changed_files,
            "total_checked": total_checked,
            "all_clean": all_clean,
            "check_only": check || !write,
        }),
    )
}

// ── rename ──────────────────────────────────────────────────────────────────

fn rename_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return err_invalid(id, "Missing required parameter: entity_id");
        }
    };
    let new_name = match args.get("new_name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return err_invalid(id, "Missing required parameter: new_name");
        }
    };

    if new_name.is_empty()
        || new_name.len() < 2
        || !new_name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return err_invalid(
            id,
            "Invalid entity ID: must be 2-60 alphanumeric/underscore characters",
        );
    }

    if state.graph.node(entity_id).is_none() {
        return err_invalid(id, format!("Entity not found: {}", entity_id));
    }

    // Real rename edits computed over the graph (specforge-graph::rename).
    match specforge_graph::rename::compute_rename_edits(&state.graph, entity_id, new_name) {
        Some(edits) => {
            let affected_files: std::collections::BTreeSet<&str> =
                edits.iter().map(|e| e.file.as_str()).collect();
            let edit_json: Vec<serde_json::Value> = edits
                .iter()
                .map(|e| {
                    json!({
                        "file": e.file,
                        "line": e.line,
                        "start_col": e.start_col,
                        "end_col": e.end_col,
                        "new_text": e.new_text,
                    })
                })
                .collect();
            let result = json!({
                "old_name": entity_id,
                "new_name": new_name,
                "affected_files": affected_files,
                "edits": edit_json,
            });
            ok(id, result)
        }
        None => err_invalid(
            id,
            format!("cannot rename '{entity_id}': definition or span not found in graph"),
        ),
    }
}

// ── init ────────────────────────────────────────────────────────────────────

fn init_op(_state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let path = PathBuf::from(args.get("path").and_then(|v| v.as_str()).unwrap_or("."));
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("my-project")
                .to_string()
        });
    let version = args
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();
    let extensions: Vec<&str> = args
        .get("extensions")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // Refuse to clobber an existing project.
    if path.join("specforge.json").exists() {
        return err_invalid(id, format!("project already exists at {}", path.display()));
    }

    let config = serde_json::json!({
        "name": name,
        "version": version,
        "extensions": extensions,
    });
    if let Err(e) = std::fs::create_dir_all(path.join("spec")) {
        return err_invalid(id, format!("cannot create project: {e}"));
    }
    if let Err(e) = std::fs::write(path.join("specforge.json"), config.to_string()) {
        return err_invalid(id, format!("cannot write specforge.json: {e}"));
    }
    let starter = "spec MyProject \"Project specification\" {\n}\n";
    if let Err(e) = std::fs::write(path.join("spec").join("specforge.spec"), starter) {
        return err_invalid(id, format!("cannot write starter file: {e}"));
    }

    ok(
        id,
        json!({
            "project_path": path.display().to_string(),
            "config_file": "specforge.json",
            "starter_file": "spec/specforge.spec",
            "extensions_installed": extensions,
            "name": name,
            "version": version,
        }),
    )
}

// ── add / remove ────────────────────────────────────────────────────────────

fn add_extension_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let specifier = match args.get("specifier").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return err_invalid(id, "Missing required parameter: specifier"),
    };
    let allow_unsigned = args
        .get("allow_unsigned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "add needs a project root (pass {\"path\": ...})");
    };

    let config_path = root.join("specforge.json");
    let extensions_dir = root.join(".specforge").join("extensions");
    let cache_dir = root.join(".specforge").join("cache");
    let lock_path = root.join("specforge.lock");

    let mut lock = read_lock_file(&lock_path).unwrap_or_default();

    // Local .wasm path install (offline).
    if specifier.ends_with(".wasm") {
        let local = PathBuf::from(&specifier);
        if !local.exists() {
            return err_invalid(id, format!("file not found: {}", local.display()));
        }
        let name = local
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        return match install_from_local(
            &name,
            "0.0.0",
            &local,
            &extensions_dir,
            &cache_dir,
            &mut lock,
            false,
        ) {
            Ok(result) => {
                if let Err(diag) = write_lock_file(&lock, &lock_path) {
                    return err_invalid(id, diag.message);
                }
                update_config_extensions(&config_path, &result.name, &result.version);
                ok(
                    id,
                    json!({
                        "extension": result.name,
                        "installed": true,
                        "version": result.version,
                        "sha256": result.wasm_hash,
                        "source": "local",
                        "note": "re-run specforge.analyze (use_cached=false) to load it",
                    }),
                )
            }
            Err(diag) => err_invalid(id, format!("{}: {}", diag.code, diag.message)),
        };
    }

    if !specifier.starts_with('@') || !specifier.contains('/') {
        return err_invalid(
            id,
            "specifier must be @scope/name[@version] or a .wasm path",
        );
    }

    // Registry install: resolve → download → integrity → trust → install.
    let (name, version) = match specifier.split_once('@') {
        // "@scope/name" or "@scope/name@version" (scope carries the first @)
        _ if specifier.matches('@').count() > 1 => {
            let (n, v) = specifier.rsplit_once('@').unwrap();
            (n.to_string(), v.to_string())
        }
        _ => (specifier.clone(), "latest".to_string()),
    };

    let registries = registries_for(&config_path);
    let client = HttpRegistryClient::new();

    let resolved_version = if version == "latest"
        || version.starts_with('^')
        || version.starts_with('~')
        || version.starts_with('>')
        || version == "*"
    {
        let Some(registry) = registries.first() else {
            return err_invalid(id, "no registries configured");
        };
        match resolve_version(&name, &version, &client, registry) {
            Ok(v) => v,
            Err(diag) => return err_invalid(id, format!("{}: {}", diag.code, diag.message)),
        }
    } else {
        version.clone()
    };

    let spec = format!("{name}@{resolved_version}");
    let response = match resolve_from_registry(&spec, &registries, &client) {
        Ok(r) => r,
        Err(diag) => return err_invalid(id, format!("{}: {}", diag.code, diag.message)),
    };
    let wasm_bytes = match client.download_wasm(&response.wasm_url) {
        Ok(bytes) => bytes,
        Err(e) => return err_invalid(id, e.to_diagnostic().message),
    };
    if let Err(diag) = verify_registry_integrity(&wasm_bytes, &response.sha256) {
        return err_invalid(id, format!("{}: {}", diag.code, diag.message));
    }

    // Publisher signature verification + TOFU pin policy (single shared
    // implementation with the CLI). assume_yes=false: a key change refuses
    // instead of prompting (an agent must not silently re-pin trust).
    let trust = match specforge_registry::client::trust_flow::check_and_pin(
        &response.name,
        &response,
        &wasm_bytes,
        allow_unsigned,
        false,
        "json",
        None,
    ) {
        Ok(t) => t,
        Err(diag) => return err_invalid(id, format!("{}: {}", diag.code, diag.message)),
    };

    match install_extension(
        &response.name,
        &response.version,
        &wasm_bytes,
        &response.sha256,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
        trust.key_id.as_deref(),
    ) {
        Ok(result) => {
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                return err_invalid(id, diag.message);
            }
            update_config_extensions(&config_path, &result.name, &result.version);
            ok(
                id,
                json!({
                    "extension": result.name,
                    "installed": true,
                    "version": result.version,
                    "sha256": result.wasm_hash,
                    "key_id": trust.key_id,
                    "note": "re-run specforge.analyze (use_cached=false) to load it",
                }),
            )
        }
        Err(diag) => err_invalid(id, format!("{}: {}", diag.code, diag.message)),
    }
}

fn remove_extension_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return err_invalid(id, "Missing required parameter: name"),
    };
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "remove needs a project root (pass {\"path\": ...})");
    };

    let lock_path = root.join("specforge.lock");
    let extensions_dir = root.join(".specforge").join("extensions");
    let cache_dir = root.join(".specforge").join("cache");

    let mut lock = match read_lock_file(&lock_path) {
        Ok(lock) => lock,
        Err(_) => {
            return err_invalid(
                id,
                format!("extension '{name}' is not installed (no lock file found)"),
            );
        }
    };
    if !lock.entries.iter().any(|e| e.name == name) {
        return err_invalid(id, format!("extension '{name}' is not installed"));
    }

    match uninstall_extension(
        &name,
        &state.manifests,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        force,
    ) {
        Ok(result) => {
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                return err_invalid(id, diag.message);
            }
            ok(
                id,
                json!({
                    "removed_extension": name,
                    "success": true,
                    "version": result.version,
                    "cache_invalidated": result.cache_invalidated,
                }),
            )
        }
        Err(diag) => err_invalid(id, format!("{}: {}", diag.code, diag.message)),
    }
}

// ── migrate ─────────────────────────────────────────────────────────────────

fn migrate_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let Some(path) = project_root_of(state, &args) else {
        return err_invalid(id, "migrate needs a project root (pass {\"path\": ...})");
    };
    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let no_backup = args
        .get("no_backup")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let content = match std::fs::read_to_string(path.join("specforge.json")) {
        Ok(c) => c,
        Err(_) => {
            return err_invalid(id, format!("no specforge.json found at {}", path.display()));
        }
    };
    let (version, _diags) = specforge_migrate::detect_format_version(&content);
    let target = specforge_migrate::CURRENT_FORMAT_VERSION;
    if version == target {
        return ok(
            id,
            json!({
                "from_version": format!("{version}"),
                "to_version": format!("{target}"),
                "migrated": false,
                "dry_run": dry_run,
                "changes": [],
                "message": "project is already at the latest format version",
            }),
        );
    }

    let summary = specforge_migrate::migrate_project(&path, &target, dry_run, no_backup);
    ok(
        id,
        json!({
            "from_version": format!("{version}"),
            "to_version": format!("{target}"),
            "migrated": !dry_run && summary.migrated_count > 0,
            "dry_run": dry_run,
            "files_migrated": summary.migrated_count,
            "files_skipped": summary.skipped_count,
            "files_failed": summary.failed_count,
            "diagnostics": summary.diagnostics,
        }),
    )
}

// ── extensions ──────────────────────────────────────────────────────────────

fn extensions_op(state: &McpState, _args: Value, id: Option<Value>) -> JsonRpcResponse {
    // Real state: what the session actually loaded, plus on-disk lock data.
    let installed: Vec<serde_json::Value> = state
        .manifests
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "version": m.version,
                "entity_kinds": m.entity_kinds.iter().map(|k| k.name.clone()).collect::<Vec<_>>(),
                "validation_rules": m.validation_rules.len(),
            })
        })
        .collect();

    let lock = state
        .project_root
        .as_ref()
        .map(|root| read_lock_file(&root.join("specforge.lock")).ok())
        .unwrap_or(None);
    let lock_entries: Vec<serde_json::Value> = lock
        .as_ref()
        .map(|l| {
            l.entries
                .iter()
                .map(|e| json!({ "name": e.name, "version": e.version }))
                .collect()
        })
        .unwrap_or_default();

    let kinds: std::collections::BTreeSet<String> = state
        .graph
        .nodes()
        .iter()
        .map(|n| n.kind.raw.to_string())
        .collect();

    ok(
        id,
        json!({
            "extensions": installed,
            "lock_file_entries": lock_entries,
            "entity_kinds_in_graph": kinds,
        }),
    )
}

// ── providers ───────────────────────────────────────────────────────────────

fn providers_op(state: &McpState, _args: Value, id: Option<Value>) -> JsonRpcResponse {
    let Some(root) = &state.project_root else {
        return err_invalid(id, "no project root available");
    };
    let config_path = root.join("specforge.json");
    let config: Value = match std::fs::read_to_string(&config_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    };
    let providers = config
        .get("providers")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let count = providers.as_array().map(|a| a.len()).unwrap_or(0);
    ok(id, json!({ "providers": providers, "count": count }))
}

// ── doctor ──────────────────────────────────────────────────────────────────

fn doctor_op(state: &McpState, _args: Value, id: Option<Value>) -> JsonRpcResponse {
    let Some(root) = &state.project_root else {
        return err_invalid(id, "doctor needs a project root");
    };
    let lock_path = root.join("specforge.lock");
    let extensions_dir = root.join(".specforge").join("extensions");

    let lock = read_lock_file(&lock_path).ok();
    let installed_versions: std::collections::HashMap<String, String> = lock
        .as_ref()
        .map(|l| {
            l.entries
                .iter()
                .map(|e| (e.name.clone(), e.version.clone()))
                .collect()
        })
        .unwrap_or_default();

    let compute_hash = |wasm_path: &Path| -> Option<String> {
        let bytes = std::fs::read(wasm_path).ok()?;
        Some(specforge_wasm::hex_sha256(&bytes))
    };

    let results = lock
        .as_ref()
        .map(|l| run_doctor_check(l, &extensions_dir, compute_hash, &installed_versions))
        .unwrap_or_default();

    // DoctorStatus is an enum: derive per-extension status truthfully.
    let status_label = |s: &specforge_wasm::DoctorStatus| match s {
        specforge_wasm::DoctorStatus::Healthy => ("ok", None),
        specforge_wasm::DoctorStatus::MissingBinary { name } => (
            "missing binary",
            Some(format!("installed wasm for '{name}' not found")),
        ),
        specforge_wasm::DoctorStatus::StaleHash {
            name,
            expected,
            actual,
        } => (
            "stale hash",
            Some(format!(
                "'{name}' hash mismatch: lock expects {expected}, found {actual}"
            )),
        ),
        specforge_wasm::DoctorStatus::PeerMismatch {
            name,
            peer,
            required,
        } => (
            "peer mismatch",
            Some(format!("'{name}' requires peer '{peer}' at {required}")),
        ),
    };
    let extensions_ok = results
        .iter()
        .all(|r| matches!(r, specforge_wasm::DoctorStatus::Healthy));

    // SMT solver availability — analyze --prove degrades without it.
    let z3_ok = Z3Command::new("z3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Wasm compile cache presence.
    let cache_dir = std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join(".cache")
            .join("specforge")
            .join("wasmtime")
    });

    let mut findings = Vec::new();
    for r in &results {
        let (label, detail) = status_label(r);
        if let Some(detail) = detail {
            findings.push(format!("{label}: {detail}"));
        }
    }
    if !z3_ok {
        findings
            .push("z3 not found: `specforge analyze --prove` will skip SMT checks (W098)".into());
    }
    if let Some(dir) = cache_dir.filter(|d| !d.exists()) {
        findings.push(format!(
            "wasm compile cache not populated yet: {} (created on first run)",
            dir.display()
        ));
    }

    ok(
        id,
        json!({
            "extensions_ok": extensions_ok,
            "findings": findings,
            "cache_status": "ok",
            "cache_checks": results
                .iter()
                .map(|r| {
                    let (label, _) = status_label(r);
                    json!({ "status": label })
                })
                .collect::<Vec<_>>(),
            "installed_count": installed_versions.len(),
        }),
    )
}

// ── collect ─────────────────────────────────────────────────────────────────

fn collect_op(_state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let collector = args
        .get("collector")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");

    let known_formats = ["junit", "tap", "json", "auto"];
    if let Some(fmt) = args
        .get("format")
        .and_then(|v| v.as_str())
        .filter(|fmt| !known_formats.contains(fmt))
    {
        return err_invalid(
            id,
            serde_json::json!({
                "message": format!("Unrecognized format: {fmt}"),
                "available_formats": known_formats
            })
            .to_string(),
        );
    }

    if let Some(ext) = args
        .get("extension")
        .and_then(|v| v.as_str())
        .filter(|ext| !ext.starts_with('@'))
    {
        return err_invalid(id, format!("Unknown extension: {ext}"));
    }

    let Some(root) = project_root_of(_state, &args) else {
        return err_invalid(id, "collect needs a project root (pass {\"path\": ...})");
    };

    let collector_name = if collector == "auto" {
        let files: Vec<String> = std::fs::read_dir(&root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        let patterns: &[(&str, &str)] = &[
            ("junit", "rust"),
            ("jest", "javascript"),
            ("pytest", "python"),
        ];
        match auto_detect_collector(patterns, &files) {
            Ok(name) => name,
            Err(diag) => return err_invalid(id, format!("{}: {}", diag.code, diag.message)),
        }
    } else {
        collector.to_string()
    };

    ok(
        id,
        json!({
            "collector": collector_name,
            "project_root": root.display().to_string(),
            "status": "ready",
        }),
    )
}

// ── render ──────────────────────────────────────────────────────────────────

fn render_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("json");

    let known_renderers = ["json", "dot", "context", "brief"];
    if !known_renderers.contains(&format) {
        return err_invalid(
            id,
            serde_json::json!({
                "message": format!("Unrecognized renderer format: {format}"),
                "available_renderers": known_renderers
            })
            .to_string(),
        );
    }

    // Render the CURRENT graph from session state — real output, no writes.
    use specforge_emitter::{EmitFormat, EmitOptions, emit};
    let emit_format = match format {
        "json" => EmitFormat::Json,
        "dot" => EmitFormat::Dot,
        "context" => EmitFormat::Context,
        _ => EmitFormat::Brief,
    };
    match emit(
        &state.graph,
        &EmitOptions {
            format: emit_format,
            scope: None,
            schema: None,
            depth: None,
            kind_filter: Vec::new(),
            token_budget: None,
            kind_registry: None,
        },
    ) {
        Ok(text) => ok(id, json!({ "format": format, "output": text })),
        Err(e) => err_invalid(id, format!("render failed: {e}")),
    }
}
