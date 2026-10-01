//! MCP management operations. Every op performs its real function against
//! the same library backends the CLI uses — canned placeholder responses are
//! forbidden (hardening-plan P1 / success criterion S2: a tool either does
//! real work or refuses with an explicit error; it never lies).

use serde_json::{Value, json};
use std::path::PathBuf;

use specforge_common::find_project_root;
use specforge_wasm::read_lock_file;

use crate::protocol::error_codes;
use crate::state::McpState;
use crate::tool::ToolOutcome;

/// Run the operation `name`. `id` is not used; the operations keep it in
/// their signatures until the tool table (plan 04 T3) replaces this match.
pub fn handle_operation(
    state: &mut McpState,
    name: &str,
    args: Value,
    id: Option<Value>,
) -> ToolOutcome {
    match name {
        "specforge.format" => format_op(state, args, id),
        "specforge.rename" => rename_op(state, args, id),
        "specforge.init" => init_op(state, args, id),
        "specforge.add_extension" => {
            let outcome = add_extension_op(state, args, id);
            let added = outcome
                .success_payload()
                .filter(|o| o["installed"] == true)
                .map(|o| json!({"extension": o["extension"], "version": o["version"]}));
            match added {
                Some(event) => outcome.with_event("extension_added", event),
                None => outcome,
            }
        }
        "specforge.remove_extension" => remove_extension_op(state, args, id),
        "specforge.migrate" => migrate_op(state, args, id),
        "specforge.extensions" => extensions_op(state, args, id),
        "specforge.providers" => providers_op(state, args, id),
        "specforge.doctor" => doctor_op(state, args, id),
        "specforge.collect" => collect_op(state, args, id),
        "specforge.render" => render_op(state, args, id),
        _ => ToolOutcome::refused(
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

fn err_invalid(_id: Option<Value>, message: impl Into<String>) -> ToolOutcome {
    ToolOutcome::invalid_params(message)
}

fn ok(_id: Option<Value>, result: Value) -> ToolOutcome {
    ToolOutcome::ok(result)
}

/// An operation's failure as an invalid-params error whose `data` carries
/// the diagnostic code and its suggestion, plus the operation's own data.
fn err_op(_id: Option<Value>, error: specforge_ops::OpError) -> ToolOutcome {
    let mut data = json!({
        "code": error.code,
        "diagnostic": {
            "severity": "error",
            "message": error.message,
            "suggestion": error.suggestion,
        },
    });
    if let Some(Value::Object(extra)) = error.data {
        for (key, value) in extra {
            data[key] = value;
        }
    }
    ToolOutcome::refused_with_data(error_codes::INVALID_PARAMS, error.message, data)
}

/// The session's graph exported through the shared operation, with the
/// schema its extensions produce: the one export behind `specforge.export`,
/// `specforge.render` and `specforge://graph` (ADR 0004 D3-a).
pub(crate) fn export_graph(
    state: &McpState,
    request: &specforge_ops::export::Request,
) -> Result<String, specforge_ops::OpError> {
    let schema = specforge_emitter::generate_schema(
        &state.kind_registry,
        &state.edge_registry,
        &state.field_registry,
        &state.extension_info,
    );
    let project = specforge_ops::export::Project {
        graph: &state.graph,
        kinds: &state.kind_registry,
        fields: &state.field_registry,
        schema: &schema,
    };
    specforge_ops::export::export(&project, request)
}

// ── format ──────────────────────────────────────────────────────────────────

fn format_op(state: &mut McpState, args: Value, id: Option<Value>) -> ToolOutcome {
    use specforge_ops::format::{self, Mode, Request};

    let check = args.get("check").and_then(|v| v.as_bool()).unwrap_or(false);
    let diff = args.get("diff").and_then(|v| v.as_bool()).unwrap_or(false);
    let write = args
        .get("write")
        .and_then(|v| v.as_bool())
        .unwrap_or(!check && !diff);

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "format needs a project root (pass {\"path\": ...})");
    };
    let Some(project_root) = find_project_root(&root) else {
        return err_invalid(
            id,
            format!("no specforge project found at {}", root.display()),
        );
    };

    // The run `specforge format` makes. Relative paths name files under
    // the project root.
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
    let mode = if write { Mode::Write } else { Mode::Check };
    let outcome = format::run(&Request {
        root: &project_root,
        config_dir: &project_root,
        paths: &explicit,
        mode,
    });

    let shown = |path: &std::path::Path| path.display().to_string();
    let changed_files: Vec<String> = outcome.changes.iter().map(|c| shown(&c.path)).collect();
    let failed_files: Vec<String> = outcome.write_failures().map(|c| shown(&c.path)).collect();
    let mut result = json!({
        "changed_files": changed_files,
        "total_checked": outcome.checked,
        "all_clean": outcome.changes.is_empty(),
        "check_only": !write,
        "diagnostics": specforge_emitter::diagnostics_json(&outcome.config_diagnostics),
    });
    if diff {
        let diffs: Vec<Value> = outcome
            .changes
            .iter()
            .map(|c| {
                let file_path = shown(&c.path);
                let stats = specforge_formatter::unified_diff(&file_path, &c.before, &c.after);
                json!({
                    "file_path": file_path,
                    "before": c.before,
                    "after": c.after,
                    "insertions": stats.insertions,
                    "deletions": stats.deletions,
                })
            })
            .collect();
        result["diffs"] = Value::from(diffs);
    }
    if failed_files.is_empty() {
        return ok(id, result);
    }

    // Every other file was still formatted; the call failed for these.
    let reasons: Vec<String> = outcome
        .write_failures()
        .map(|c| {
            format!(
                "failed to write {}: {}",
                shown(&c.path),
                c.write_error.as_deref().unwrap_or_default()
            )
        })
        .collect();
    result["message"] = Value::from(reasons.join("; "));
    result["failed_files"] = Value::from(failed_files);
    // What was written is on disk: serve it, as a successful run would be.
    if outcome.changes.iter().any(|c| c.written(mode)) && !state.serves_other_than(&project_root) {
        state.recompile(&project_root);
    }
    ToolOutcome::failed_with(result)
}

// ── rename ──────────────────────────────────────────────────────────────────

fn rename_op(state: &mut McpState, args: Value, id: Option<Value>) -> ToolOutcome {
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

    // Spans are relative to the spec root the graph was compiled from.
    let spec_root = state.spec_root.clone().unwrap_or_else(|| root.clone());
    let Some(edits) =
        specforge_graph::rename::identifier_edits(&state.graph, entity_id, new_name, |file| {
            std::fs::read_to_string(spec_root.join(file)).ok()
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
        let path = spec_root.join(file);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return err_invalid(id, format!("failed to read {}", path.display()));
        };
        let renamed =
            specforge_graph::rename::apply_edits(&text, edits.iter().filter(|e| e.file == *file));
        if let Err(e) = std::fs::write(&path, renamed) {
            return err_invalid(id, format!("failed to write {}: {e}", path.display()));
        }
    }
    state.recompile(&root);
    result["diagnostics"] = serde_json::to_value(&state.diagnostics).unwrap_or_default();
    ok(id, result)
}

// ── init ────────────────────────────────────────────────────────────────────

fn init_op(state: &mut McpState, args: Value, id: Option<Value>) -> ToolOutcome {
    use specforge_ops::init;

    let Some(path) = args.get("path").and_then(|v| v.as_str()).map(PathBuf::from) else {
        return err_invalid(id, "Missing required parameter: path");
    };
    let extensions: Vec<String> = args
        .get("extensions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    // The scaffold `specforge init` writes; the new project must not land
    // inside the one this server serves.
    let request = init::Request {
        dir: &path,
        name: args.get("name").and_then(|v| v.as_str()),
        version: args.get("version").and_then(|v| v.as_str()),
        extensions: &extensions,
        forbid_inside: state.project_root.as_deref(),
    };
    let outcome = match init::plan(&request).and_then(|plan| init::apply(&path, &plan)) {
        Ok(outcome) => outcome,
        Err(error) => return err_op(id, error),
    };
    let result = ok(
        id,
        json!({
            "project_path": path.display().to_string(),
            "config_file": "specforge.json",
            "starter_file": init::STARTER_FILE,
            "extensions_installed": outcome.extensions,
            "name": outcome.name,
            "version": outcome.version,
        }),
    );
    state.push_event(
        "project_initialized",
        json!({"path": path.display().to_string(), "name": outcome.name}),
    );
    result
}

// ── add / remove ────────────────────────────────────────────────────────────

fn add_extension_op(state: &McpState, args: Value, id: Option<Value>) -> ToolOutcome {
    use specforge_ops::extension::{self, AddOutcome, AddRequest, Origin, Trust};

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
    let source = match extension::parse(&specifier) {
        Ok(source) => source,
        Err(error) => return err_op(id, error),
    };

    // The shared operation `specforge add` runs. An agent can't be asked,
    // so a publisher key change is refused rather than re-pinned.
    let request = AddRequest {
        root: &root,
        source,
        allow_unsigned,
        trust: Trust::Refuse,
        dry_run,
    };
    let registry = specforge_ops::registry::HttpRegistry::for_project(&root, "add_extension");
    let source_of = |origin: &Origin| match origin {
        Origin::Builtin => "builtin".to_string(),
        Origin::Installed { source } => source.clone(),
    };
    match extension::add(&request, &registry) {
        Ok(AddOutcome::Builtin {
            name,
            changed,
            peers_enabled,
        }) => ok(
            id,
            json!({
                "extension": name,
                "installed": changed,
                "source": "builtin",
                "changed": changed,
                "peers_enabled": peers_enabled,
                "note": "re-run specforge.analyze (use_cached=false) to load it",
            }),
        ),
        Ok(AddOutcome::Installed {
            name,
            version,
            sha256,
            key_id,
            origin,
        }) => ok(
            id,
            json!({
                "extension": name,
                "installed": true,
                "version": version,
                "sha256": sha256,
                "key_id": key_id,
                "source": source_of(&origin),
                "note": "re-run specforge.analyze (use_cached=false) to load it",
            }),
        ),
        // Already installed and enabled: an info response, nothing changed.
        Ok(AddOutcome::AlreadyPresent { name, version }) => ok(
            id,
            json!({
                "extension": name,
                "installed": false,
                "already_present": true,
                "version": version,
                "message": format!("{name} {version} is already installed; specforge.json is unchanged"),
            }),
        ),
        Ok(AddOutcome::Planned {
            name,
            version,
            origin,
        }) => ok(
            id,
            json!({
                "extension": name,
                "installed": false,
                "dry_run": true,
                "version": version,
                "source": source_of(&origin),
            }),
        ),
        Err(error) => err_op(id, error),
    }
}

fn remove_extension_op(state: &McpState, args: Value, id: Option<Value>) -> ToolOutcome {
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

    // The shared operation, over what the session loaded.
    let request = specforge_ops::extension::RemoveRequest {
        root: &root,
        name: &name,
        force,
        dry_run,
        loaded: &state.manifests,
        kinds: &state.kind_registry,
        graph: &state.graph,
    };
    match specforge_ops::extension::remove(&request) {
        Ok(outcome) => {
            let mut result = json!({
                "removed_extension": outcome.name,
                "success": true,
                "version": outcome.version,
                "orphan_warnings": outcome.orphan_warnings,
            });
            if outcome.dry_run {
                result["dry_run"] = Value::from(true);
            }
            ok(id, result)
        }
        Err(mut error) => {
            if error.code == specforge_ops::extension::NOT_FOUND {
                error.data = Some(json!({"extension": name}));
            }
            err_op(id, error)
        }
    }
}

// ── migrate ─────────────────────────────────────────────────────────────────

fn migrate_op(state: &McpState, args: Value, id: Option<Value>) -> ToolOutcome {
    let Some(path) = project_root_of(state, &args) else {
        return err_invalid(id, "migrate needs a project root (pass {\"path\": ...})");
    };
    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let no_backup = args
        .get("no_backup")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // The format version to migrate to, checked as `specforge migrate
    // --target-version` checks it.
    let target = match specforge_ops::migrate::parse_target(
        args.get("target_version").and_then(|v| v.as_str()),
    ) {
        Ok(target) => target,
        Err(error) => return err_op(id, error),
    };

    if !path.join("specforge.json").is_file() {
        return err_invalid(id, "no specforge.json found in the project root");
    }
    // The migration `specforge migrate` runs, hooks and rollback included.
    let runtime = state.wasm_runtime(&path);
    let request = specforge_ops::migrate::Request {
        root: &path,
        target,
        dry_run,
        no_backup,
    };
    let outcome = specforge_ops::migrate::run(&request, Some(runtime.as_ref()));
    let (from, to) = (outcome.from.to_string(), outcome.to.to_string());
    // The format version lives in each spec file's header: with no file
    // behind the target, the project is current and nothing ran.
    if !outcome.pending {
        return ok(
            id,
            json!({
                "from_version": from,
                "to_version": to,
                "migrated": false,
                "dry_run": dry_run,
                "changes": [],
                "message": "project is already at the latest format version",
            }),
        );
    }

    let summary = &outcome.summary;
    let post_migration_errors: Vec<Value> = outcome
        .post_errors()
        .map(|d| json!({"code": d.code, "message": d.message}))
        .collect();
    let result = json!({
        "from_version": from,
        "to_version": to,
        "migrated": outcome.migrated(),
        "dry_run": dry_run,
        "files_migrated": summary.migrated_count,
        "files_skipped": summary.skipped_count,
        "files_failed": summary.failed_count,
        "results": summary.results,
        "diffs": summary.diffs,
        "diagnostics": summary.diagnostics,
        "hooks_invoked": outcome.hooks_invoked,
        "hook_failures": outcome.hook_failures,
        "schema_warnings": specforge_emitter::diagnostics_json(&outcome.schema_warnings),
        "structural_differences": specforge_emitter::diagnostics_json(&outcome.structural_differences),
        "rolled_back": outcome.rollback.is_some(),
        "rollback": outcome.rollback,
        "post_migration_validated": outcome.validated,
        "post_migration_errors": post_migration_errors,
    });
    if outcome.failed() {
        return ToolOutcome::failed_with(result);
    }
    ok(id, result)
}

// ── extensions ──────────────────────────────────────────────────────────────

fn extensions_op(state: &McpState, _args: Value, id: Option<Value>) -> ToolOutcome {
    use specforge_ops::extension::{self, Origin};

    let Some(root) = &state.project_root else {
        return err_invalid(id, "no project root available");
    };
    // The shared listing, over what the session compiled.
    let entries = extension::list(root, &state.manifests, &state.kind_registry, &state.graph);
    let listed: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "version": e.version,
                "source": match &e.origin {
                    Origin::Builtin => "builtin",
                    Origin::Installed { source } => source.as_str(),
                },
                "status": e.status.as_str(),
                "entity_kinds": e.entity_kinds,
                "entity_count": e.entity_count,
                "validation_rules": e.validation_rules,
            })
        })
        .collect();

    let lock_entries: Vec<Value> = read_lock_file(&root.join("specforge.lock"))
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
            "extensions": listed,
            "lock_file_entries": lock_entries,
            "entity_kinds_in_graph": kinds,
        }),
    )
}

// ── providers ───────────────────────────────────────────────────────────────

fn providers_op(state: &McpState, _args: Value, id: Option<Value>) -> ToolOutcome {
    let Some(root) = &state.project_root else {
        return err_invalid(id, "no project root available");
    };
    // The providers specforge.json configures, as the scheme registry built
    // from the loaded extensions sees them: the listing the CLI prints.
    let (providers, diagnostics) = specforge_ops::extension::providers(root, &state.manifests);
    let listed: Vec<Value> = providers
        .iter()
        .map(|p| {
            json!({
                "scheme": p.scheme,
                "alias": p.alias,
                "extension": p.extension,
                "status": p.status.as_str(),
            })
        })
        .collect();
    let count = listed.len();
    ok(
        id,
        json!({
            "providers": listed,
            "count": count,
            "diagnostics": specforge_emitter::diagnostics_json(&diagnostics),
        }),
    )
}

// ── doctor ──────────────────────────────────────────────────────────────────

fn doctor_op(state: &mut McpState, args: Value, id: Option<Value>) -> ToolOutcome {
    let Some(root) = state.project_root.clone() else {
        return err_invalid(id, "doctor needs a project root");
    };
    // Like specforge.validate, a fresh compile unless the caller opts into
    // the last one (ADR 0004 D3-d): the agent may have edited the project
    // since, and the session would not know.
    let use_cached = args
        .get("use_cached")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !use_cached || state.loaded_at.is_none() {
        state.recompile(&root);
    }
    // The same report `specforge doctor` prints.
    let report = specforge_ops::doctor::diagnose(&root, &state.manifests, &state.diagnostics);
    let conflicts: Vec<&str> = report
        .conflicts
        .iter()
        .map(|c| c.message.as_str())
        .collect();
    ok(
        id,
        json!({
            "extensions_ok": report.issues.is_empty() && report.load_failures.is_empty(),
            "conflicts": conflicts,
            "cache_status": report.cache_status,
            "findings": report.findings,
            "installed_count": report.extensions_checked,
            "extensions": report.extensions,
            "enhancements": report.enhancements,
            "shadowed": report.shadowed,
            "load_failures": report.load_failures,
            "issues": report.issues,
            "z3_available": report.z3_available,
        }),
    )
}

// ── collect ─────────────────────────────────────────────────────────────────

fn collect_op(state: &McpState, args: Value, id: Option<Value>) -> ToolOutcome {
    use specforge_emitter::collect::{self, Mode, Request, RunnerOutput};

    let Some(root) = project_root_of(state, &args) else {
        return err_invalid(id, "collect needs a project root (pass {\"path\": ...})");
    };
    let runner = args
        .get("runner")
        .and_then(|v| v.as_str())
        .filter(|r| *r != "auto");
    let run = args.get("run").and_then(|v| v.as_bool()).unwrap_or(false);

    let runtime = specforge_component::project_runtime(&root);
    let ctx = specforge_project::CompiledProject::compile(&root, Some(&runtime)).into_context();
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

fn render_op(state: &McpState, args: Value, id: Option<Value>) -> ToolOutcome {
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
        return ToolOutcome::refused_with_data(
            error_codes::INVALID_PARAMS,
            format!(
                "Unrecognized renderer format: {format} (available: {})",
                available.join(", ")
            ),
            json!({ "available_renderers": available }),
        );
    };

    // "json" is the full graph export: Graph Protocol 2.0 with the schema,
    // as `specforge export --format graph` writes it.
    let request = specforge_ops::export::Request {
        format: format.parse().ok(),
        scope: args.get("scope").and_then(|v| v.as_str()),
        ..specforge_ops::export::Request::default()
    };
    let output = match export_graph(state, &request) {
        Ok(text) => text,
        Err(e) => return err_invalid(id, format!("render failed: {}", e.message)),
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
