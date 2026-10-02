//! MCP management operations. Every op performs its real function against
//! the same library backends the CLI uses — canned placeholder responses are
//! forbidden (hardening-plan P1 / success criterion S2: a tool either does
//! real work or refuses with an explicit error; it never lies).

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use specforge_common::{Diagnostic, find_project_root};
use specforge_wasm::read_lock_file;

use crate::args::{lenient, strings};
use crate::state::McpState;
use crate::tool::{ErrorCode, McpError, ToolOutcome, is_diagnostic_code};

/// `specforge.add_extension`: the install, plus `extension_added` when it
/// installed something.
pub(crate) fn add_extension(state: &mut McpState, args: AddArgs) -> ToolOutcome {
    let outcome = add_extension_op(state, args);
    let added = outcome
        .success_payload()
        .filter(|o| o["installed"] == true)
        .map(|o| json!({"extension": o["extension"], "version": o["version"]}));
    match added {
        Some(event) => outcome.with_event("extension_added", event),
        None => outcome,
    }
}

// ── shared helpers ──────────────────────────────────────────────────────────

/// Resolve the project root, preferring an explicit `path` argument.
fn project_root_of(state: &McpState, path: Option<&str>) -> Option<PathBuf> {
    path.map(PathBuf::from)
        .or_else(|| state.project_root.clone())
}

fn ok(result: Value) -> ToolOutcome {
    ToolOutcome::ok(result)
}

/// A failure with `code` and `message`.
fn fail(code: ErrorCode, message: impl Into<String>) -> ToolOutcome {
    ToolOutcome::error(code, message)
}

/// An operation's failure as an `McpError`. A diagnostic code (`E027`)
/// rides in `diagnostic`, with its suggestion; a slug (`extension_not_found`)
/// picks the error code, and its suggestion and the operation's own data
/// ride in `data`.
pub(crate) fn op_error(error: specforge_ops::OpError) -> McpError {
    let code = match error.code.as_ref() {
        "extension_not_found" => ErrorCode::ExtensionNotFound,
        "config_not_found" => ErrorCode::FileNotFound,
        "config_invalid" | "invalid_schema_version" => ErrorCode::SchemaMismatch,
        specforge_ops::infer::MANIFEST_INVALID => ErrorCode::SchemaMismatch,
        "unknown_format" => ErrorCode::InvalidInput,
        "extension_conflict" | "project_exists" => ErrorCode::Conflict,
        "invalid_name" => ErrorCode::InvalidInput,
        code => ErrorCode::for_diagnostic(code),
    };
    let mut mcp_error = McpError::new(code, error.message.clone());
    let mut data = error.data.unwrap_or_else(|| json!({}));
    if is_diagnostic_code(&error.code) {
        let mut diagnostic = Diagnostic::error(error.code.as_ref(), error.message);
        if let Some(suggestion) = error.suggestion {
            diagnostic = diagnostic.with_suggestion(suggestion);
        }
        mcp_error = mcp_error.with_diagnostic(&diagnostic);
    } else if let Some(suggestion) = error.suggestion {
        data["suggestion"] = Value::from(suggestion);
    }
    if data.as_object().is_some_and(|d| !d.is_empty()) {
        mcp_error = mcp_error.with_data(data);
    }
    mcp_error
}

/// [`op_error`] as the tool's result.
fn err_op(error: specforge_ops::OpError) -> ToolOutcome {
    op_error(error).into()
}

/// The session's graph exported through the shared operation, with the
/// schema its extensions produce: the one export behind `specforge.export`,
/// `specforge.render` and `specforge://graph` (ADR 0004 D3-a). The schema
/// carries the version `specforge export` would give it, computed against
/// the project's `.specforge/schema-cache.json`; the server only reads the
/// cache, as `specforge schema` does, so the next CLI export still sees
/// what changed.
pub(crate) fn export_graph(
    state: &McpState,
    request: &specforge_ops::export::Request,
) -> Result<String, specforge_ops::OpError> {
    let schema = project_schema(state);
    let project = specforge_ops::export::Project {
        graph: state.graph(),
        kinds: &state.registries().kinds,
        fields: &state.registries().fields,
        schema: &schema,
    };
    specforge_ops::export::export(&project, request)
}

/// The GraphProtocolSchema the session's extensions produce, versioned as
/// `specforge export` would version it: the schema a full export embeds,
/// and the one `specforge.schema` and `specforge://schema` serve.
pub(crate) fn project_schema(state: &McpState) -> specforge_emitter::GraphProtocolSchema {
    let mut schema = specforge_emitter::generate_schema(
        &state.registries().kinds,
        &state.registries().edges,
        &state.registries().fields,
        &state.registries().extension_info,
    );
    if let Some(root) = &state.project_root {
        specforge_emitter::attach_schema_version(&mut schema, &root.join(".specforge"));
    }
    schema
}

// ── format ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FormatArgs {
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
    #[serde(default, deserialize_with = "strings")]
    paths: Vec<String>,
    #[serde(default, deserialize_with = "lenient")]
    check: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    diff: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    write: Option<bool>,
}

pub(crate) fn format_op(state: &mut McpState, args: FormatArgs) -> ToolOutcome {
    use specforge_ops::format::{self, Mode, Request};

    let check = args.check.unwrap_or(false);
    let diff = args.diff.unwrap_or(false);
    let write = args.write.unwrap_or(!check && !diff);

    let Some(root) = project_root_of(state, args.path.as_deref()) else {
        return ToolOutcome::no_project("format needs a project root (pass {\"path\": ...})");
    };
    let Some(project_root) = find_project_root(&root) else {
        return ToolOutcome::no_project(format!(
            "no specforge project found at {}",
            root.display()
        ));
    };

    // The run `specforge format` makes. Relative paths name files under
    // the project root.
    let explicit: Vec<PathBuf> = args.paths.iter().map(|p| project_root.join(p)).collect();
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
        "diagnostics": specforge_common::diagnostics_json(&outcome.config_diagnostics),
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
        return ok(result);
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
        state.reload(&project_root);
    }
    let message = result["message"].as_str().unwrap_or_default().to_string();
    McpError::new(ErrorCode::InternalError, message)
        .with_data(result)
        .into()
}

// ── rename ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RenameArgs {
    entity_id: String,
    new_name: String,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
}

pub(crate) fn rename_op(state: &mut McpState, args: RenameArgs) -> ToolOutcome {
    use specforge_ops::rename;
    let entity_id = args.entity_id.as_str();
    let new_name = args.new_name.as_str();
    let dry_run = args.dry_run.unwrap_or(false);

    // Spans are relative to the spec root the graph was compiled from.
    let root = project_root_of(state, args.path.as_deref());
    let spec_root = state
        .spec_root()
        .map(Path::to_path_buf)
        .or_else(|| root.clone());
    let read = |file: &str| {
        spec_root
            .as_ref()
            .and_then(|dir| std::fs::read_to_string(dir.join(file)).ok())
    };
    let plan = match rename::plan(state.graph(), entity_id, new_name, read) {
        Ok(plan) => plan,
        Err(e) if e.code == rename::INVALID_ID => {
            return ToolOutcome::invalid_input("new_name", e.message);
        }
        Err(e) if e.code == rename::NOT_FOUND => {
            return McpError::new(ErrorCode::EntityNotFound, e.message)
                .with_entity(entity_id)
                .into();
        }
        Err(e) if e.code == rename::TAKEN => {
            return McpError::new(ErrorCode::Conflict, e.message)
                .with_entity(entity_id)
                .into();
        }
        Err(e) => return fail(ErrorCode::InternalError, e.message),
    };
    let (Some(root), Some(spec_root)) = (root, spec_root) else {
        return ToolOutcome::no_project("rename needs a project root (pass {\"path\": ...})");
    };

    let edit_json: Vec<serde_json::Value> = plan
        .edits
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
        "affected_files": plan.affected_files(),
        "edits": edit_json,
    });
    if dry_run {
        result["dry_run"] = Value::from(true);
        return ok(result);
    }
    if let Err(e) = rename::apply(&plan, &spec_root) {
        return fail(ErrorCode::InternalError, e.message);
    }
    // Recompile from disk, not just the renamed files: the diagnostics
    // returned are what `specforge check` reports now, edits made since
    // the last load included.
    state.reload(&root);
    result["diagnostics"] =
        serde_json::to_value(specforge_common::diagnostics_json(&state.diagnostics()))
            .unwrap_or_default();
    ok(result)
}

// ── init ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InitArgs {
    path: String,
    #[serde(default, deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    version: Option<String>,
    #[serde(default, deserialize_with = "strings")]
    extensions: Vec<String>,
}

pub(crate) fn init_op(state: &mut McpState, args: InitArgs) -> ToolOutcome {
    use specforge_ops::init;

    let path = PathBuf::from(&args.path);
    let extensions = &args.extensions;

    // The scaffold `specforge init` writes; the new project must not land
    // inside the one this server serves.
    let request = init::Request {
        dir: &path,
        name: args.name.as_deref(),
        version: args.version.as_deref(),
        extensions,
        forbid_inside: state.project_root.as_deref(),
    };
    let outcome = match init::plan(&request).and_then(|plan| init::apply(&path, &plan)) {
        Ok(outcome) => outcome,
        Err(error) => return err_op(error),
    };
    let result = ok(json!({
        "project_path": path.display().to_string(),
        "config_file": "specforge.json",
        "starter_file": init::STARTER_FILE,
        "extensions_installed": outcome.extensions,
        "name": outcome.name,
        "version": outcome.version,
    }));
    state.push_event(
        "project_initialized",
        json!({"path": path.display().to_string(), "name": outcome.name}),
    );
    result
}

// ── add / remove ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddArgs {
    specifier: String,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    allow_unsigned: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
}

fn add_extension_op(state: &McpState, args: AddArgs) -> ToolOutcome {
    use specforge_ops::extension::{self, AddOutcome, AddRequest, Origin, Source, Trust};

    let specifier = args.specifier.clone();
    let allow_unsigned = args.allow_unsigned.unwrap_or(false);
    let dry_run = args.dry_run.unwrap_or(false);

    let Some(root) = project_root_of(state, args.path.as_deref()) else {
        return ToolOutcome::no_project("add needs a project root (pass {\"path\": ...})");
    };
    let source = match extension::parse(&specifier) {
        Ok(source) => source,
        Err(error) => return err_op(error),
    };

    let registry = specforge_ops::registry::HttpRegistry::for_project(&root, "add_extension");
    // What reading the registry configuration reported (E067, W140,
    // I003), as `specforge add` shows it: only a registry package reads it.
    let reported = match &source {
        Source::Registry { .. } => registry.diagnostics().to_vec(),
        _ => Vec::new(),
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
    let source_of = |origin: &Origin| match origin {
        Origin::Builtin => "builtin".to_string(),
        Origin::Installed { source } => source.clone(),
    };
    let outcome = match extension::add(&request, &registry) {
        Ok(AddOutcome::Builtin {
            name,
            changed,
            peers_enabled,
        }) => ok(json!({
            "extension": name,
            "installed": changed,
            "source": "builtin",
            "changed": changed,
            "peers_enabled": peers_enabled,
            "note": "re-run specforge.analyze (use_cached=false) to load it",
        })),
        Ok(AddOutcome::Installed {
            name,
            version,
            sha256,
            key_id,
            origin,
        }) => ok(json!({
            "extension": name,
            "installed": true,
            "version": version,
            "sha256": sha256,
            "key_id": key_id,
            "source": source_of(&origin),
            "note": "re-run specforge.analyze (use_cached=false) to load it",
        })),
        // Already installed and enabled: an info response, nothing changed.
        Ok(AddOutcome::AlreadyPresent { name, version }) => ok(json!({
            "extension": name,
            "installed": false,
            "already_present": true,
            "version": version,
            "message": format!("{name} {version} is already installed; specforge.json is unchanged"),
        })),
        Ok(AddOutcome::Planned {
            name,
            version,
            origin,
        }) => ok(json!({
            "extension": name,
            "installed": false,
            "dry_run": true,
            "version": version,
            "source": source_of(&origin),
        })),
        Err(error) => err_op(error),
    };
    outcome.with_diagnostics(reported)
}

#[derive(Debug, Deserialize)]
pub struct RemoveArgs {
    name: String,
    #[serde(default, deserialize_with = "lenient")]
    force: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
}

pub(crate) fn remove_extension_op(state: &McpState, args: RemoveArgs) -> ToolOutcome {
    let name = args.name.clone();
    let force = args.force.unwrap_or(false);
    let dry_run = args.dry_run.unwrap_or(false);

    let Some(root) = project_root_of(state, args.path.as_deref()) else {
        return ToolOutcome::no_project("remove needs a project root (pass {\"path\": ...})");
    };

    // The shared operation, over what the session loaded.
    let request = specforge_ops::extension::RemoveRequest {
        root: &root,
        name: &name,
        force,
        dry_run,
        loaded: &state.registries().manifests,
        kinds: &state.registries().kinds,
        graph: state.graph(),
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
            ok(result)
        }
        Err(mut error) => {
            if error.code == specforge_ops::extension::NOT_FOUND {
                error.data = Some(json!({"extension": name}));
            }
            err_op(error)
        }
    }
}

// ── migrate ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MigrateArgs {
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    target_version: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    no_backup: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
}

pub(crate) fn migrate_op(state: &McpState, args: MigrateArgs) -> ToolOutcome {
    let Some(path) = project_root_of(state, args.path.as_deref()) else {
        return ToolOutcome::no_project("migrate needs a project root (pass {\"path\": ...})");
    };
    let dry_run = args.dry_run.unwrap_or(false);
    let no_backup = args.no_backup.unwrap_or(false);
    // The format version to migrate to, checked as `specforge migrate
    // --target-version` checks it.
    let target = match specforge_ops::migrate::parse_target(args.target_version.as_deref()) {
        Ok(target) => target,
        Err(error) => return err_op(error),
    };

    if !path.join("specforge.json").is_file() {
        return ToolOutcome::no_project("no specforge.json found in the project root");
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
        return ok(json!({
            "from_version": from,
            "to_version": to,
            "migrated": false,
            "dry_run": dry_run,
            "changes": [],
            "message": "project is already at the latest format version",
        }));
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
        "schema_warnings": specforge_common::diagnostics_json(&outcome.schema_warnings),
        "structural_differences": specforge_common::diagnostics_json(&outcome.structural_differences),
        "rolled_back": outcome.rollback.is_some(),
        "rollback": outcome.rollback,
        "post_migration_validated": outcome.validated,
        "post_migration_errors": post_migration_errors,
    });
    // A failed run's report rides in `data`.
    if outcome.failed() {
        let (code, message) = if outcome.post_errors().next().is_some() {
            (
                ErrorCode::CompilationFailed,
                "the migrated project does not compile",
            )
        } else {
            (ErrorCode::InternalError, "the migration failed")
        };
        return McpError::new(code, message).with_data(result).into();
    }
    ok(result)
}

// ── extensions ──────────────────────────────────────────────────────────────

pub(crate) fn extensions_op(state: &McpState, _args: crate::args::NoArgs) -> ToolOutcome {
    use specforge_ops::extension::{self, Origin};

    let Some(root) = &state.project_root else {
        return ToolOutcome::no_project("no project root available");
    };
    // The shared listing, over what the session compiled.
    let entries = extension::list(
        root,
        &state.registries().manifests,
        &state.registries().kinds,
        state.graph(),
    );
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
        .graph()
        .nodes()
        .iter()
        .map(|n| n.kind.raw.to_string())
        .collect();

    ok(json!({
        "extensions": listed,
        "lock_file_entries": lock_entries,
        "entity_kinds_in_graph": kinds,
    }))
}

// ── providers ───────────────────────────────────────────────────────────────

pub(crate) fn providers_op(state: &McpState, _args: crate::args::NoArgs) -> ToolOutcome {
    let Some(root) = &state.project_root else {
        return ToolOutcome::no_project("no project root available");
    };
    // The providers specforge.json configures, as the scheme registry built
    // from the loaded extensions sees them: the listing the CLI prints.
    let (providers, diagnostics) =
        specforge_ops::extension::providers(root, &state.registries().manifests);
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
    ok(json!({
        "providers": listed,
        "count": count,
        "diagnostics": specforge_common::diagnostics_json(&diagnostics),
    }))
}

// ── doctor ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DoctorArgs {
    #[serde(default, deserialize_with = "lenient")]
    use_cached: Option<bool>,
}

pub(crate) fn doctor_op(state: &mut McpState, args: DoctorArgs) -> ToolOutcome {
    let Some(root) = state.project_root.clone() else {
        return ToolOutcome::no_project("doctor needs a project root");
    };
    // Like specforge.validate, a fresh compile unless the caller opts into
    // the last one (ADR 0004 D3-d): the agent may have edited the project
    // since, and the session would not know.
    let use_cached = args.use_cached.unwrap_or(false);
    if !use_cached || state.loaded_at.is_none() {
        state.reload(&root);
    }
    // The same report `specforge doctor` prints, as the spec's
    // McpDoctorReport plus its sections. Credential health is the user's,
    // not the project's: only the CLI reports it.
    let report =
        specforge_ops::doctor::diagnose(&root, &state.registries().manifests, &state.diagnostics());
    ok(json!({
        "extensions_ok": report.extensions_ok(),
        "conflicts": report.conflict_messages(),
        "cache_status": report.cache_status,
        "findings": report.findings,
        "installed_count": report.extensions_checked,
        "extensions": report.extensions,
        "enhancements": report.enhancements,
        "shadowed": report.shadowed,
        "load_failures": report.load_failures,
        "issues": report.issues,
        "z3_available": report.z3_available,
    }))
}

// ── collect ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CollectArgs {
    #[serde(default, deserialize_with = "lenient")]
    runner: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    run: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
}

pub(crate) fn collect_op(state: &mut McpState, args: CollectArgs) -> ToolOutcome {
    use specforge_ops::collect::{self, Consent, Mode, Request, RunnerOutput};

    let Some(root) = project_root_of(state, args.path.as_deref()) else {
        return ToolOutcome::no_project("collect needs a project root (pass {\"path\": ...})");
    };
    let runner = args.runner.as_deref().filter(|r| *r != "auto");
    let run = args.run.unwrap_or(false);

    // Tests map to the entities on disk now. The served project is
    // reloaded and collected with its own runtime; another project is
    // compiled for the call only.
    let other = state.serves_other_than(&root);
    if !other {
        state.reload(&root);
    }
    let state: &McpState = state;
    let runtime = state.wasm_runtime(&root);
    let compiled =
        other.then(|| specforge_project::CompiledProject::compile(&root, Some(runtime.as_ref())));
    let (graph, manifests) = match &compiled {
        Some(project) => (&project.graph, &project.env.registries.manifests),
        None => (state.graph(), &state.registries().manifests),
    };
    let known = collect::KnownEntities::from_graph(graph);

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
        manifests,
        runtime.as_ref(),
        &known,
        // The server never prompts: a command runs only if the user already
        // approved it for this project with `specforge collect` in a terminal.
        Consent::Approved,
        &mut |_, _| {},
    ) {
        Ok(outcome) => ok(outcome.to_json()),
        Err(e) if e.code == "E059" => McpError::from_diagnostic(&Diagnostic::error(
            e.code,
            format!(
                "the test command isn't approved for this project; run `specforge collect` \
                 in a terminal once to approve it ({})",
                e.message
            ),
        ))
        .into(),
        Err(e) => McpError::from_diagnostic(&Diagnostic::error(e.code.as_ref(), e.message)).into(),
    }
}

// ── render ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RenderArgs {
    #[serde(default, deserialize_with = "lenient")]
    format: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    out_dir: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    scope: Option<String>,
}

pub(crate) fn render_op(state: &McpState, args: RenderArgs) -> ToolOutcome {
    let format = args.format.as_deref().unwrap_or("json");

    // Each renderer and the file it writes into out_dir.
    const RENDERERS: [(&str, &str); 4] = [
        ("json", "graph.json"),
        ("dot", "graph.dot"),
        ("context", "context.json"),
        ("brief", "brief.json"),
    ];
    let Some((_, file_name)) = RENDERERS.iter().find(|(name, _)| *name == format) else {
        let available: Vec<&str> = RENDERERS.iter().map(|(name, _)| *name).collect();
        return McpError::new(
            ErrorCode::InvalidInput,
            format!(
                "Unrecognized renderer format: {format} (available: {})",
                available.join(", ")
            ),
        )
        .with_argument("format")
        .with_data(json!({ "available_renderers": available }))
        .into();
    };

    // "json" is the full graph export: Graph Protocol 2.0 with the schema,
    // as `specforge export --format graph` writes it.
    let request = specforge_ops::export::Request {
        format: format.parse().ok(),
        scope: args.scope.as_deref(),
        ..specforge_ops::export::Request::default()
    };
    let output = match export_graph(state, &request) {
        Ok(text) => text,
        Err(e) => return err_op(e),
    };

    // With out_dir the rendering lands on disk; without it, inline.
    let Some(out_dir) = args.out_dir.as_deref() else {
        return ok(json!({ "format": format, "output": output, "output_files": [] }));
    };
    let out_dir = PathBuf::from(out_dir);
    let path = out_dir.join(file_name);
    if let Err(e) = std::fs::create_dir_all(&out_dir).and_then(|()| std::fs::write(&path, output)) {
        return fail(
            ErrorCode::InternalError,
            format!("failed to write {}: {e}", path.display()),
        );
    }
    ok(json!({ "format": format, "output_files": [path.display().to_string()] }))
}
