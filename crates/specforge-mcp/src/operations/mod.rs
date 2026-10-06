//! MCP management operations. Every op performs its real function against
//! the same library backends the CLI uses — canned placeholder responses are
//! forbidden (hardening-plan P1 / success criterion S2: a tool either does
//! real work or refuses with an explicit error; it never lies).

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use specforge_common::{Diagnostic, find_project_root};

use crate::args::{lenient, strings};
use crate::target::{Call, CallTarget};
use crate::tool::{ErrorCode, Handled, McpError, ToolOutcome, is_diagnostic_code};

/// `specforge.add_extension`: the install, plus `extension_added` when it
/// installed something.
pub(crate) fn add_extension(call: &mut Call<'_>, args: AddArgs) -> Handled {
    let outcome = add_extension_op(call, args)?;
    let added = outcome
        .success_payload()
        .filter(|o| o["installed"] == true)
        .map(|o| json!({"extension": o["extension"], "version": o["version"]}));
    Ok(match added {
        Some(event) => outcome.with_event("extension_added", event),
        None => outcome,
    })
}

// ── shared helpers ──────────────────────────────────────────────────────────

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
        "unknown_format" | "invalid_input" | "unknown_kind" => ErrorCode::InvalidInput,
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

// ── format ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FormatArgs {
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
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

pub(crate) fn format_op(call: &mut Call<'_>, args: FormatArgs) -> Handled {
    use specforge_ops::format::{self, Mode, Request};

    let check = args.check.unwrap_or(false);
    let diff = args.diff.unwrap_or(false);
    let write = args.write.unwrap_or(!check && !diff);

    // The project the call formats: the served one, or the one `path`
    // names; its config decides what is formatted.
    let root = call.project()?.root.to_path_buf();
    let Some(project_root) = find_project_root(&root) else {
        return Ok(ToolOutcome::no_project(format!(
            "no specforge project found at {}",
            root.display()
        )));
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
        return Ok(ok(result));
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
    // What was written is on disk: the project is brought up to date with
    // it, as a successful run's is.
    if outcome.changes.iter().any(|c| c.written(mode)) {
        call.wrote();
    }
    let message = result["message"].as_str().unwrap_or_default().to_string();
    Err(Box::new(
        McpError::new(ErrorCode::InternalError, message).with_data(result),
    ))
}

// ── rename ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RenameArgs {
    entity_id: String,
    new_name: String,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
    path: Option<String>,
}

pub(crate) fn rename_op(call: &mut Call<'_>, args: RenameArgs) -> Handled {
    use specforge_ops::rename;
    let entity_id = args.entity_id.as_str();
    let new_name = args.new_name.as_str();
    let dry_run = args.dry_run.unwrap_or(false);

    // Planned on the call's project as it is on disk (the target brought
    // the served project up to date, or compiled the project `path`
    // names), whose spans are relative to its spec root.
    let spec_root = call.project()?.spec_root.to_path_buf();
    let planned = rename::plan(&crate::tools::navigator(call), entity_id, new_name);
    let plan = match planned {
        Ok(plan) => plan,
        Err(e) if e.code == rename::INVALID_ID => {
            return Ok(ToolOutcome::invalid_input("new_name", e.message));
        }
        Err(e) if e.code == rename::NOT_FOUND => {
            return Err(Box::new(
                McpError::new(ErrorCode::EntityNotFound, e.message).with_entity(entity_id),
            ));
        }
        Err(e) if e.code == rename::TAKEN => {
            return Err(Box::new(
                McpError::new(ErrorCode::Conflict, e.message).with_entity(entity_id),
            ));
        }
        Err(e) => return Ok(fail(ErrorCode::InternalError, e.message)),
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
        return Ok(ok(result));
    }
    if let Err(e) = rename::apply(&plan, &spec_root) {
        return Ok(fail(ErrorCode::InternalError, e.message));
    }
    // The project as it is on disk now, edits made since the last call
    // included: the diagnostics returned are what `specforge check`
    // reports for it.
    let diagnostics = call.wrote();
    result["diagnostics"] =
        serde_json::to_value(specforge_common::diagnostics_json(&diagnostics)).unwrap_or_default();
    Ok(ok(result))
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

pub(crate) fn init_op(call: &mut Call<'_>, args: InitArgs) -> ToolOutcome {
    use specforge_ops::init;

    // The directory the target names (as given: init creates it).
    let path = call
        .new_project_dir()
        .map_or_else(|| PathBuf::from(&args.path), Path::to_path_buf);
    let extensions = &args.extensions;
    let served = call.state.session().root().map(Path::to_path_buf);

    // The scaffold `specforge init` writes; the new project must not land
    // inside the one this server serves.
    let request = init::Request {
        dir: &path,
        name: args.name.as_deref(),
        version: args.version.as_deref(),
        extensions,
        forbid_inside: served.as_deref(),
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
    call.state.push_event(
        "project_initialized",
        json!({"path": path.display().to_string(), "name": outcome.name}),
    );
    // With no project served, the server serves the one it created (D5).
    if served.is_none() {
        call.state.serve(&path);
    }
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
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
    path: Option<String>,
}

fn add_extension_op(call: &Call<'_>, args: AddArgs) -> Handled {
    use specforge_ops::extension::{self, AddOutcome, AddRequest, Origin, Source, Trust};

    let specifier = args.specifier.clone();
    let allow_unsigned = args.allow_unsigned.unwrap_or(false);
    let dry_run = args.dry_run.unwrap_or(false);

    // The project the call installs into: the served one, or the one
    // `path` names.
    let root = call.project()?.root.to_path_buf();
    let served = matches!(call.target(), CallTarget::Served);
    // Once it is enabled, the server serves it (the next request brings the
    // project up to date); another project only has it on disk.
    let note = if served {
        "the server serves it from the next call on"
    } else {
        "installed in the project the path names; the server keeps serving its own"
    };
    let source = match extension::parse(&specifier) {
        Ok(source) => source,
        Err(error) => return Ok(err_op(error)),
    };

    let registry = specforge_ops_registry::HttpRegistry::for_project(&root, "add_extension");
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
    let source_of = Origin::source;
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
            "note": note,
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
            "note": note,
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
    Ok(outcome.with_diagnostics(reported))
}

#[derive(Debug, Deserialize)]
pub struct RemoveArgs {
    name: String,
    #[serde(default, deserialize_with = "lenient")]
    force: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
    path: Option<String>,
}

pub(crate) fn remove_extension_op(call: &mut Call<'_>, args: RemoveArgs) -> Handled {
    let name = args.name.clone();
    let force = args.force.unwrap_or(false);
    let dry_run = args.dry_run.unwrap_or(false);

    // The shared operation, over the view of the call's project: its
    // dependents and its orphaned entities, the served project's or those
    // of the project `path` names.
    let request = specforge_ops::extension::RemoveRequest {
        name: &name,
        force,
        dry_run,
    };
    Ok(
        match specforge_ops::extension::remove(&call.project()?.view(), &request) {
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
        },
    )
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
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
    path: Option<String>,
}

pub(crate) fn migrate_op(call: &mut Call<'_>, args: MigrateArgs) -> Handled {
    // The project the call migrates, and the runtime its hooks run in.
    let project = call.project()?;
    let path = project.root;
    let dry_run = args.dry_run.unwrap_or(false);
    let no_backup = args.no_backup.unwrap_or(false);
    // The format version to migrate to, checked as `specforge migrate
    // --target-version` checks it.
    let target = match specforge_ops::migrate::parse_target(args.target_version.as_deref()) {
        Ok(target) => target,
        Err(error) => return Ok(err_op(error)),
    };

    if !path.join("specforge.json").is_file() {
        return Ok(ToolOutcome::no_project(
            "no specforge.json found in the project root",
        ));
    }
    let runtime = project.runtime;
    // The migration `specforge migrate` runs, hooks and rollback included.
    let request = specforge_ops::migrate::Request {
        root: path,
        target,
        dry_run,
        no_backup,
    };
    let outcome = specforge_ops::migrate::run(&request, Some(runtime.as_ref()));
    let (from, to) = (outcome.from.to_string(), outcome.to.to_string());
    // The format version lives in each spec file's header: with no file
    // behind the target, the project is current and nothing ran.
    if !outcome.pending {
        return Ok(ok(json!({
            "from_version": from,
            "to_version": to,
            "migrated": false,
            "dry_run": dry_run,
            "changes": [],
            "message": "project is already at the latest format version",
        })));
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
        return Err(Box::new(McpError::new(code, message).with_data(result)));
    }
    Ok(ok(result))
}

// ── extensions ──────────────────────────────────────────────────────────────

pub(crate) fn extensions_op(call: &mut Call<'_>, _args: crate::args::NoArgs) -> Handled {
    // The shared listing, over the project view: what the project
    // compiled, its lock and the kinds its graph uses.
    let listing = specforge_ops::extension::list(&call.project()?.view());
    let extensions: Vec<Value> = listing.extensions.iter().map(|e| e.to_json()).collect();
    let lock_entries: Vec<Value> = listing
        .locked
        .iter()
        .map(|e| json!({ "name": e.name, "version": e.version }))
        .collect();
    Ok(ok(json!({
        "extensions": extensions,
        "lock_file_entries": lock_entries,
        "entity_kinds_in_graph": listing.kinds_in_graph,
    })))
}

// ── providers ───────────────────────────────────────────────────────────────

pub(crate) fn providers_op(call: &mut Call<'_>, _args: crate::args::NoArgs) -> Handled {
    // The providers specforge.json configures, as the scheme registry built
    // from the loaded extensions sees them: the listing the CLI prints.
    let listing = specforge_ops::extension::providers(&call.project()?.view());
    Ok(ok(listing.to_json()))
}

// ── doctor ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DoctorArgs {
    /// Read by the call's target (`Freshness::FreshUnlessCached`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target applies use_cached")]
    use_cached: Option<bool>,
}

pub(crate) fn doctor_op(call: &mut Call<'_>, _args: DoctorArgs) -> Handled {
    // The target brought the project up to date with disk unless the
    // caller opted into the last compile (`use_cached`, ADR 0004 D3-d).
    let project = call.project()?;
    // The same report `specforge doctor` prints, as the spec's
    // McpDoctorReport plus its sections. Credential health is the user's,
    // not the project's: only the CLI reports it.
    let report = specforge_ops::doctor::diagnose(&project.view());
    Ok(ok(json!({
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
    })))
}

// ── collect ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CollectArgs {
    #[serde(default, deserialize_with = "lenient")]
    runner: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    run: Option<bool>,
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
    path: Option<String>,
}

pub(crate) fn collect_op(call: &mut Call<'_>, args: CollectArgs) -> Handled {
    use specforge_ops::collect::{self, Consent, Mode, Request, RunnerOutput};

    let runner = args.runner.as_deref().filter(|r| *r != "auto");
    let run = args.run.unwrap_or(false);

    // Tests map to the entities on disk now: the target brought the served
    // project up to date, or compiled the project `path` names for this
    // call, in the runtime it collects with.
    let project = call.project()?;
    let runtime = project.runtime;
    let known = collect::KnownEntities::from_graph(project.graph);

    let request = Request {
        root: project.root,
        runner,
        mode: if run {
            // The server owns stdio: the runner's output is discarded.
            Mode::Run(RunnerOutput::Discard)
        } else {
            Mode::NoRun
        },
    };
    Ok(
        match collect::collect(
            &request,
            project.env.registries.declarations(),
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
            Err(e) => {
                McpError::from_diagnostic(&Diagnostic::error(e.code.as_ref(), e.message)).into()
            }
        },
    )
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

pub(crate) fn render_op(call: &mut Call<'_>, args: RenderArgs) -> ToolOutcome {
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
    let output = match specforge_ops::export::export(&call.view(), &request) {
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
