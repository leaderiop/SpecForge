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
    install_extension, install_from_local, read_lock_file, uninstall_extension, write_lock_file,
};

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

/// Drop `name` (bare or `name@version`) from specforge.json's extensions list.
fn remove_config_extension(config_path: &Path, name: &str) {
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return;
    };
    let Ok(mut json) = serde_json::from_str::<Value>(&content) else {
        return;
    };
    let Some(exts) = json.get_mut("extensions").and_then(|e| e.as_array_mut()) else {
        return;
    };
    let before = exts.len();
    exts.retain(|e| {
        e.as_str().is_none_or(|entry| {
            entry != name
                && entry
                    .strip_prefix(name)
                    .is_none_or(|rest| !rest.starts_with('@'))
        })
    });
    if exts.len() != before
        && let Ok(pretty) = serde_json::to_string_pretty(&json)
    {
        let _ = std::fs::write(config_path, pretty);
    }
}

// ── format ──────────────────────────────────────────────────────────────────

fn format_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let check = args.get("check").and_then(|v| v.as_bool()).unwrap_or(false);
    let diff = args.get("diff").and_then(|v| v.as_bool()).unwrap_or(false);
    let write = args
        .get("write")
        .and_then(|v| v.as_bool())
        .unwrap_or(!check && !diff);

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
    // Relative paths name files under the project root.
    let explicit: Vec<PathBuf> = args
        .get("paths")
        .and_then(|v| v.as_array())
        .map(|paths| {
            paths
                .iter()
                .filter_map(|p| p.as_str())
                .map(|p| project_root.join(p))
                .collect()
        })
        .unwrap_or_default();
    let targets = specforge_formatter::discover_targets(&search_root, &explicit, &[]);

    let mut changed_files = Vec::new();
    let mut diffs = Vec::new();
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
        let file_path = target.display().to_string();
        if diff {
            let stats = specforge_formatter::unified_diff(&file_path, &source, &result.formatted);
            diffs.push(json!({
                "file_path": file_path,
                "before": source,
                "after": result.formatted,
                "insertions": stats.insertions,
                "deletions": stats.deletions,
            }));
        }
        changed_files.push(file_path);
        // Apply in write mode only.
        if write && let Err(e) = std::fs::write(target, &result.formatted) {
            return err_invalid(id, format!("failed to write {}: {e}", target.display()));
        }
    }
    let _ = config_diags;

    let all_clean = changed_files.is_empty();
    let mut result = json!({
        "changed_files": changed_files,
        "total_checked": total_checked,
        "all_clean": all_clean,
        "check_only": check || !write,
    });
    if diff {
        result["diffs"] = Value::from(diffs);
    }
    ok(id, result)
}

// ── rename ──────────────────────────────────────────────────────────────────

fn rename_op(state: &mut McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
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
    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

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
    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "rename needs a project root (pass {\"path\": ...})");
    };

    let Some(edits) =
        specforge_graph::rename::identifier_edits(&state.graph, entity_id, new_name, |file| {
            std::fs::read_to_string(root.join(file)).ok()
        })
    else {
        return err_invalid(
            id,
            format!("cannot rename '{entity_id}': '{new_name}' exists"),
        );
    };
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
    let mut result = json!({
        "old_name": entity_id,
        "new_name": new_name,
        "affected_files": affected_files,
        "edits": edit_json,
    });
    if dry_run {
        result["dry_run"] = Value::from(true);
        return ok(id, result);
    }

    for file in &affected_files {
        let path = root.join(file);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return err_invalid(id, format!("failed to read {}", path.display()));
        };
        let renamed = apply_line_edits(&text, edits.iter().filter(|e| e.file == *file));
        if let Err(e) = std::fs::write(&path, renamed) {
            return err_invalid(id, format!("failed to write {}: {e}", path.display()));
        }
    }
    state.recompile(&root);
    result["diagnostics"] = serde_json::to_value(&state.diagnostics).unwrap_or_default();
    ok(id, result)
}

/// `text` with each edit's byte range on its 1-based line replaced.
fn apply_line_edits<'a>(
    text: &str,
    edits: impl Iterator<Item = &'a specforge_graph::rename::RenameEdit>,
) -> String {
    let mut by_line: std::collections::BTreeMap<usize, Vec<&specforge_graph::rename::RenameEdit>> =
        std::collections::BTreeMap::new();
    for edit in edits {
        by_line.entry(edit.line).or_default().push(edit);
    }
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let Some(line_edits) = by_line.get_mut(&(index + 1)) else {
            out.push_str(line);
            continue;
        };
        // Right to left, so earlier columns stay valid.
        line_edits.sort_by_key(|e| std::cmp::Reverse(e.start_col));
        let mut line = line.to_string();
        for edit in line_edits.iter() {
            line.replace_range(edit.start_col..edit.end_col, &edit.new_text);
        }
        out.push_str(&line);
    }
    out
}

// ── init ────────────────────────────────────────────────────────────────────

fn init_op(state: &mut McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let Some(path) = args.get("path").and_then(|v| v.as_str()).map(PathBuf::from) else {
        return err_invalid(id, "Missing required parameter: path");
    };
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
    let mut extensions: Vec<String> = args
        .get("extensions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    if let Err(reason) = specforge_common::validate_project_name(&name) {
        return err_invalid(id, format!("invalid project name '{name}': {reason}"));
    }
    // Init only enables builtins; anything else installs afterwards.
    let builtins: Vec<&str> = specforge_component::builtins::BUILTIN_EXTENSIONS
        .iter()
        .map(|(builtin, _)| *builtin)
        .collect();
    if let Some(unknown) = extensions.iter().find(|e| !builtins.contains(&e.as_str())) {
        let message = format!("unknown extension '{unknown}': not a builtin extension");
        return JsonRpcResponse::error_with_data(
            id,
            error_codes::INVALID_PARAMS,
            message.clone(),
            json!({
                "code": "extension_not_found",
                "extension": unknown,
                "diagnostic": {
                    "severity": "error",
                    "message": message,
                    "suggestion": format!(
                        "init with builtins ({}), then install it with specforge.add_extension",
                        builtins.join(", ")
                    ),
                },
            }),
        );
    }
    // Test obligations on software kinds come from @specforge/testing (ADR 0002).
    if extensions.iter().any(|e| e == "@specforge/software")
        && !extensions.iter().any(|e| e == "@specforge/testing")
    {
        extensions.push("@specforge/testing".to_string());
    }

    // The new project must not land inside the one this server serves.
    let absolute = |p: &Path| {
        std::path::absolute(p)
            .map(|p| p.canonicalize().unwrap_or(p))
            .unwrap_or_else(|_| p.to_path_buf())
    };
    if let Some(current) = &state.project_root {
        let current = absolute(current);
        let mut target = absolute(&path);
        // Canonicalize through the nearest existing ancestor.
        let mut existing = target.clone();
        let mut rest = Vec::new();
        while !existing.exists() {
            let Some(name) = existing.file_name().map(|n| n.to_os_string()) else {
                break;
            };
            rest.push(name);
            if !existing.pop() {
                break;
            }
        }
        if let Ok(canonical) = existing.canonicalize() {
            target = rest.iter().rev().fold(canonical, |p, part| p.join(part));
        }
        if target.starts_with(&current) {
            return err_invalid(
                id,
                format!(
                    "{} is inside the current project at {}",
                    path.display(),
                    current.display()
                ),
            );
        }
    }

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
    let starter = format!("spec \"{name}\" {{\n  version \"{version}\"\n}}\n");
    if let Err(e) = std::fs::write(path.join("spec").join("specforge.spec"), starter) {
        return err_invalid(id, format!("cannot write starter file: {e}"));
    }
    state.push_event(
        "project_initialized",
        json!({"path": path.display().to_string(), "name": name}),
    );

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
    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "add needs a project root (pass {\"path\": ...})");
    };

    let config_path = root.join("specforge.json");
    let extensions_dir = root.join(".specforge").join("extensions");
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
        if dry_run {
            return ok(
                id,
                json!({
                    "extension": name,
                    "installed": false,
                    "dry_run": true,
                    "version": "0.0.0",
                    "source": "local",
                }),
            );
        }
        return match install_from_local(&name, "0.0.0", &local, &extensions_dir, &mut lock) {
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
        let Some(registry) = specforge_registry::find_registry_for_specifier(&name, &registries)
            .or_else(|| registries.first())
        else {
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
    if dry_run {
        // Resolved, not downloaded: nothing on disk changes.
        return ok(
            id,
            json!({
                "extension": response.name,
                "installed": false,
                "dry_run": true,
                "version": response.version,
                "source": "registry",
            }),
        );
    }
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

    let peer_dependencies: Vec<specforge_registry::PeerDependency> =
        serde_json::from_str::<specforge_registry::ManifestV2>(&response.manifest)
            .map(|m| m.peer_dependencies)
            .unwrap_or_default();
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
    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "remove needs a project root (pass {\"path\": ...})");
    };

    let lock_path = root.join("specforge.lock");
    let extensions_dir = root.join(".specforge").join("extensions");

    let not_found = |id, message: String| {
        JsonRpcResponse::error_with_data(
            id,
            error_codes::INVALID_PARAMS,
            message,
            json!({"code": "extension_not_found", "extension": name}),
        )
    };
    let mut lock = match read_lock_file(&lock_path) {
        Ok(lock) => lock,
        Err(_) => {
            return not_found(
                id,
                format!("extension '{name}' is not installed (no lock file found)"),
            );
        }
    };
    let Some(version) = lock
        .entries
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.version.clone())
    else {
        return not_found(id, format!("extension '{name}' is not installed"));
    };

    let orphan_warnings = orphan_warnings(state, &name);
    if dry_run {
        let dependents = specforge_wasm::check_dependents(&name, &state.manifests);
        if !dependents.is_empty() && !force {
            return err_invalid(
                id,
                format!(
                    "E027: cannot uninstall '{name}': required by {}",
                    dependents.join(", ")
                ),
            );
        }
        return ok(
            id,
            json!({
                "removed_extension": name,
                "success": true,
                "dry_run": true,
                "version": version,
                "orphan_warnings": orphan_warnings,
            }),
        );
    }

    match uninstall_extension(&name, &state.manifests, &extensions_dir, &mut lock, force) {
        Ok(result) => {
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                return err_invalid(id, diag.message);
            }
            remove_config_extension(&root.join("specforge.json"), &name);
            ok(
                id,
                json!({
                    "removed_extension": name,
                    "success": true,
                    "version": result.version,
                    "orphan_warnings": orphan_warnings,
                }),
            )
        }
        Err(diag) => err_invalid(id, format!("{}: {}", diag.code, diag.message)),
    }
}

/// One warning per entity whose kind only `extension` defines.
fn orphan_warnings(state: &McpState, extension: &str) -> Vec<String> {
    let mut warnings: Vec<String> = state
        .graph
        .nodes()
        .into_iter()
        .filter(|node| {
            state
                .kind_registry
                .get(node.kind.raw.as_str())
                .is_some_and(|kind| kind.source_extension == extension)
        })
        .map(|node| {
            format!(
                "{} '{}' uses a kind only {extension} defines",
                node.kind.raw, node.id.raw
            )
        })
        .collect();
    warnings.sort();
    warnings
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
    let mut installed: Vec<serde_json::Value> = state
        .manifests
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "version": m.version,
                "entity_kinds": m.entity_kinds.iter().map(|k| k.name.clone()).collect::<Vec<_>>(),
                "validation_rules": m.validation_rules.len(),
                "status": "loaded",
            })
        })
        .collect();
    // specforge.json names an extension the compile did not load: listed,
    // so the answer reflects the configuration.
    let configured = state
        .project_root
        .as_ref()
        .map(|root| specforge_common::load_project_config(root).extensions)
        .unwrap_or_default();
    for entry in &configured {
        // `name` or `name@version`; a scope's leading `@` is not a version.
        let (name, version) = match entry.char_indices().skip(1).find(|(_, c)| *c == '@') {
            Some((at, _)) => (&entry[..at], Some(&entry[at + 1..])),
            None => (entry.as_str(), None),
        };
        if !state.manifests.iter().any(|m| m.name == name) {
            installed.push(json!({
                "name": name,
                "version": version,
                "entity_kinds": [],
                "validation_rules": 0,
                "status": "not_loaded",
            }));
        }
    }

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
    // Each configured provider with the loaded extension backing its scheme:
    // `registered` when one contributes providers, `unregistered` otherwise.
    let (configured, _) = specforge_registry::load_provider_configurations(&config);
    let manifests: Vec<(String, specforge_registry::ManifestV2)> = state
        .manifests
        .iter()
        .map(|m| (m.name.clone(), m.clone()))
        .collect();
    let (schemes, _) = specforge_registry::register_provider_schemes(&configured, &manifests);
    let providers: Vec<Value> = configured
        .iter()
        .map(|provider| {
            let backing = schemes
                .entries
                .iter()
                .find(|e| e.scheme == provider.scheme && e.provider_name == provider.name);
            json!({
                "scheme": provider.scheme,
                "alias": provider.name,
                "extension": backing.map(|e| e.extension_name.as_str()),
                "status": if backing.is_some() { "registered" } else { "unregistered" },
            })
        })
        .collect();
    let count = providers.len();
    ok(id, json!({ "providers": providers, "count": count }))
}

// ── doctor ──────────────────────────────────────────────────────────────────

fn doctor_op(state: &McpState, _args: Value, id: Option<Value>) -> JsonRpcResponse {
    let Some(root) = &state.project_root else {
        return err_invalid(id, "doctor needs a project root");
    };
    // The same report `specforge doctor` prints, over the server's compile.
    let report = specforge_emitter::doctor::diagnose(root, &state.manifests, &state.diagnostics);
    let conflicts: Vec<&str> = report
        .conflicts
        .iter()
        .map(|c| c.message.as_str())
        .collect();
    ok(
        id,
        json!({
            "extensions_ok": report.issues.is_empty(),
            "conflicts": conflicts,
            "cache_status": report.cache_status,
            "findings": report.findings,
            "installed_count": report.extensions_checked,
            "extensions": report.extensions,
            "enhancements": report.enhancements,
            "shadowed": report.shadowed,
        }),
    )
}

// ── collect ─────────────────────────────────────────────────────────────────

fn collect_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    use specforge_emitter::collect::{self, Mode, Request, RunnerOutput};

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "collect needs a project root (pass {\"path\": ...})");
    };
    let runner = args
        .get("runner")
        .or_else(|| args.get("collector"))
        .and_then(|v| v.as_str())
        .filter(|r| *r != "auto");
    let run = args.get("run").and_then(|v| v.as_bool()).unwrap_or(false);

    let runtime = specforge_component::project_runtime(&root);
    let ctx = specforge_emitter::compile::compile_with_runtime(&root, Some(&runtime));
    let known = collect::KnownEntities::from_graph(&ctx.graph);

    // The server never prompts: a command runs only if the user already
    // approved it for this project with `specforge collect` in a terminal.
    let store = collect::consent_path();
    let mut approve = |c: &collect::Collector, _: &[String]| collect::is_approved(&store, c, &root);
    let request = Request {
        root: &root,
        runner,
        mode: if run {
            // The server owns stdio: the runner's output is discarded.
            Mode::Run(RunnerOutput::Discard)
        } else {
            Mode::NoRun
        },
    };
    match collect::collect(
        &request,
        &ctx.manifests,
        &runtime,
        &known,
        &mut approve,
        &mut |_, _| {},
    ) {
        Ok(outcome) => ok(
            id,
            json!({
                "status": "collected",
                "runners": outcome.runners,
                "diagnostics": outcome.diagnostics,
                "report": outcome.report.display().to_string(),
            }),
        ),
        Err(e) if e.code == "E059" => err_invalid(
            id,
            format!(
                "E059: the test command isn't approved for this project; run `specforge collect` \
                 in a terminal once to approve it ({})",
                e.message
            ),
        ),
        Err(e) => err_invalid(id, format!("{}: {}", e.code, e.message)),
    }
}

// ── render ──────────────────────────────────────────────────────────────────

fn render_op(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("json");

    // Each renderer and the file it writes into out_dir.
    const RENDERERS: [(&str, &str); 4] = [
        ("json", "graph.json"),
        ("dot", "graph.dot"),
        ("context", "context.json"),
        ("brief", "brief.json"),
    ];
    let Some((_, file_name)) = RENDERERS.iter().find(|(name, _)| *name == format) else {
        let available: Vec<&str> = RENDERERS.iter().map(|(name, _)| *name).collect();
        return JsonRpcResponse::error_with_data(
            id,
            error_codes::INVALID_PARAMS,
            format!(
                "Unrecognized renderer format: {format} (available: {})",
                available.join(", ")
            ),
            json!({ "available_renderers": available }),
        );
    };

    use specforge_emitter::{EmitFormat, EmitOptions, emit};
    let emit_format = match format {
        "json" => EmitFormat::Json,
        "dot" => EmitFormat::Dot,
        "context" => EmitFormat::Context,
        _ => EmitFormat::Brief,
    };
    let output = match emit(
        &state.graph,
        &EmitOptions {
            format: emit_format,
            scope: args.get("scope").and_then(|v| v.as_str()),
            field_registry: Some(&state.field_registry),
            ..EmitOptions::default()
        },
    ) {
        Ok(text) => text,
        Err(e) => return err_invalid(id, format!("render failed: {e}")),
    };

    // With out_dir the rendering lands on disk; without it, inline.
    let Some(out_dir) = args.get("out_dir").and_then(|v| v.as_str()) else {
        return ok(
            id,
            json!({ "format": format, "output": output, "output_files": [] }),
        );
    };
    let out_dir = PathBuf::from(out_dir);
    let path = out_dir.join(file_name);
    if let Err(e) = std::fs::create_dir_all(&out_dir).and_then(|()| std::fs::write(&path, output)) {
        return err_invalid(id, format!("failed to write {}: {e}", path.display()));
    }
    ok(
        id,
        json!({ "format": format, "output_files": [path.display().to_string()] }),
    )
}
