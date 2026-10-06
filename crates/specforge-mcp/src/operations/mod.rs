//! MCP management operations. Every op performs its real function against
//! the same library backends the CLI uses — canned placeholder responses are
//! forbidden (hardening-plan P1 / success criterion S2: a tool either does
//! real work or refuses with an explicit error).
//!
//! A mutation handler (format, rename, init, add_extension,
//! remove_extension, migrate) returns its reply and what its operation
//! wrote, typed ([`Mutated`], ADR 0022): the files from the operation's
//! [`specforge_ops::Writes`], the entities it changed and its domain event;
//! a preview says it only previewed. It never refreshes the target or
//! records an event itself: `crate::mutation` does.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use specforge_common::{codes, find_project_root};

use crate::args::{lenient, strings};
use crate::mutation::{Mutated, MutationEvent, MutationHandled, Written};
use crate::target::{Call, CallTarget};
use crate::tool::{ErrorCode, Handled, McpError, ToolOutcome};
use specforge_ops::OpErrorKind;

// ── shared helpers ──────────────────────────────────────────────────────────

fn ok(result: Value) -> ToolOutcome {
    ToolOutcome::ok(result)
}

/// A failure with `code` and `message`.
fn fail(code: ErrorCode, message: impl Into<String>) -> ToolOutcome {
    ToolOutcome::error(code, message)
}

// ── format ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FormatArgs {
    #[serde(default, deserialize_with = "strings")]
    paths: Vec<String>,
    #[serde(default, deserialize_with = "lenient")]
    check: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    diff: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    write: Option<bool>,
}

impl FormatArgs {
    /// Whether the call writes or only reports: `specforge format`'s one
    /// reading of check, diff and write.
    pub(crate) fn mode(&self) -> specforge_ops::format::Mode {
        specforge_ops::format::Mode::of_flags(
            self.check.unwrap_or(false),
            self.diff.unwrap_or(false),
            self.write,
        )
    }
}

pub(crate) fn format_op(call: &mut Call<'_>, args: FormatArgs) -> MutationHandled {
    use specforge_ops::format::{self, Mode, Request};

    let diff = args.diff.unwrap_or(false);
    // The one reading of check, diff and write: a run that does not write
    // is a preview.
    let mode = args.mode();
    let preview = mode != Mode::Write;

    // The project the call formats: the served one, or the one `path`
    // names; its config decides what is formatted.
    let root = call.project()?.root.to_path_buf();
    let Some(project_root) = find_project_root(&root) else {
        return Ok(Mutated::refused_unless_preview(
            preview,
            ToolOutcome::no_project(format!("no specforge project found at {}", root.display())),
        ));
    };

    // The run `specforge format` makes. Relative paths name files under
    // the project root.
    let explicit: Vec<PathBuf> = args.paths.iter().map(|p| project_root.join(p)).collect();
    let outcome = format::run(&Request {
        root: &project_root,
        paths: &explicit,
        mode,
    });

    let shown = |path: &std::path::Path| path.display().to_string();
    let changed_files: Vec<String> = outcome.changes.iter().map(|c| shown(&c.path)).collect();
    let mut result = json!({
        "changed_files": changed_files,
        "total_checked": outcome.checked,
        "all_clean": outcome.clean(),
        "check_only": mode == Mode::Check,
        "diagnostics": specforge_common::diagnostics_json(&outcome.diagnostics),
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
    let written = |reply: ToolOutcome| match preview {
        true => Mutated::preview(reply),
        false => Mutated::wrote(reply, Written::files(outcome.writes())),
    };
    if outcome.succeeded() {
        return Ok(written(ok(result)));
    }

    // Every other file was still formatted, and what was written is
    // reported; the call failed for these (read or write), with the kind
    // the files share (permission denied for locked files, not found for
    // missing ones), else an internal failure.
    let reasons: Vec<String> = outcome.failures.iter().map(ToString::to_string).collect();
    result["message"] = Value::from(reasons.join("; "));
    result["failed_files"] = Value::from(
        outcome
            .failures
            .iter()
            .map(|f| shown(f.path()))
            .collect::<Vec<_>>(),
    );
    result["failures"] = Value::from(
        outcome
            .failures
            .iter()
            .map(|f| {
                json!({
                    "file": shown(f.path()),
                    "operation": f.verb(),
                    "code": ErrorCode::from(f.kind()).as_str(),
                    "message": f.to_string(),
                })
            })
            .collect::<Vec<_>>(),
    );
    let message = result["message"].as_str().unwrap_or_default().to_string();
    let code = ErrorCode::from(outcome.failure_kind().unwrap_or(OpErrorKind::Internal));
    Ok(written(
        McpError::new(code, message).with_data(result).into(),
    ))
}

// ── rename ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RenameArgs {
    entity_id: String,
    new_name: String,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
}

pub(crate) fn rename_op(call: &mut Call<'_>, args: RenameArgs) -> MutationHandled {
    use specforge_ops::rename;
    let entity_id = args.entity_id.as_str();
    let new_name = args.new_name.as_str();
    let dry_run = args.dry_run.unwrap_or(false);

    // Planned on the call's project as it is on disk (the target brought
    // the served project up to date, or compiled the project `path`
    // names), whose spans are relative to its spec root.
    let spec_root = call.project()?.spec_root.to_path_buf();
    let planned = rename::plan(&crate::tools::navigator(call), entity_id, new_name);
    let refused = |outcome: ToolOutcome| Ok(Mutated::refused_unless_preview(dry_run, outcome));
    let plan = match planned {
        Ok(plan) => plan,
        // The operation decided what kind of failure it is, and which
        // entity it is about; an invalid new ID is the argument's fault.
        Err(e) => {
            let argument = (e.kind == OpErrorKind::InvalidInput).then_some("new_name");
            let error = McpError::from(e);
            return refused(
                match argument {
                    Some(argument) => error.with_argument(argument),
                    None => error,
                }
                .into(),
            );
        }
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
        return Ok(Mutated::preview(ok(result)));
    }
    // A failed write restores what it wrote: nothing is left written.
    let writes = match rename::apply(&plan, &spec_root) {
        Ok(writes) => writes,
        Err(e) => return refused(McpError::from(e).into()),
    };
    // The reply's `diagnostics` are what `specforge check` reports for the
    // project as it is on disk now, edits made since the last call
    // included (filled in once the target is brought up to date).
    Ok(Mutated::wrote(
        ok(result),
        Written::files(writes)
            .with_entities([new_name])
            .with_fresh_diagnostics(),
    ))
}

// ── init ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InitArgs {
    #[serde(default, deserialize_with = "lenient")]
    name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    version: Option<String>,
    #[serde(default, deserialize_with = "strings")]
    extensions: Vec<String>,
}

pub(crate) fn init_op(call: &mut Call<'_>, args: InitArgs) -> Mutated {
    use specforge_ops::init;

    // The directory the target names (as given: init creates it); the
    // target refuses a call that names none.
    let Some(path) = call.new_project_dir().map(Path::to_path_buf) else {
        return Mutated::refused(McpError::from(crate::target::TargetError::PathRequired));
    };
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
        Err(error) => return Mutated::refused_after(false, error),
    };
    let result = ok(json!({
        "project_path": path.display().to_string(),
        "config_file": "specforge.json",
        "starter_file": init::STARTER_FILE,
        "extensions_installed": outcome.extensions,
        "name": outcome.name,
        "version": outcome.version,
    }));
    // With no project served, the server serves the one it created (ADR
    // 0014 D5): `mutation::refresh` does, once it wrote.
    let event = MutationEvent::ProjectInitialized {
        project_name: outcome.name.clone(),
        extension_count: outcome.extensions.len(),
        spec_file_path: init::STARTER_FILE.to_string(),
    };
    Mutated::wrote(result, Written::files(outcome.writes).with_event(event))
}

// ── add / remove ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddArgs {
    specifier: String,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    allow_unsigned: Option<bool>,
}

/// `specforge.add_extension`: the shared add, its reply, the files it
/// wrote and `extension_added` (for an extension already there too,
/// `wasDuplicate`).
pub(crate) fn add_extension(call: &mut Call<'_>, args: AddArgs) -> MutationHandled {
    use specforge_ops::extension::{self, AddOutcome, AddRequest, Origin, Source, Trust};

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
    let source = match extension::parse(&args.specifier) {
        Ok(source) => source,
        Err(error) => return Ok(Mutated::refused_after(dry_run, error)),
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
    let added = match extension::add(&request, &registry) {
        Ok(added) => added,
        // An install that failed after placing its module reports it.
        Err(error) => {
            return Ok(Mutated::refused_after(dry_run, error).with_diagnostics(reported));
        }
    };
    let source_of = Origin::source;
    let (reply, was_duplicate) = match added.outcome {
        AddOutcome::Builtin {
            name,
            changed,
            peers_enabled,
        } => (
            json!({
                "extension": name,
                "installed": changed,
                "source": "builtin",
                "changed": changed,
                "peers_enabled": peers_enabled,
                "note": note,
            }),
            !changed,
        ),
        AddOutcome::Installed {
            name,
            version,
            sha256,
            key_id,
            origin,
        } => (
            json!({
                "extension": name,
                "installed": true,
                "version": version,
                "sha256": sha256,
                "key_id": key_id,
                "source": source_of(&origin),
                "note": note,
            }),
            false,
        ),
        // Already installed and enabled: an info response, nothing changed.
        AddOutcome::AlreadyPresent { name, version } => (
            json!({
                "extension": name,
                "installed": false,
                "already_present": true,
                "version": version,
                "message": format!("{name} {version} is already installed; specforge.json is unchanged"),
            }),
            true,
        ),
        AddOutcome::Planned {
            name,
            version,
            origin,
        } => {
            let plan = json!({
                "extension": name,
                "installed": false,
                "dry_run": true,
                "version": version,
                "source": source_of(&origin),
            });
            return Ok(Mutated::preview(ok(plan).with_diagnostics(reported)));
        }
    };
    let event = MutationEvent::ExtensionAdded {
        specifier: args.specifier,
        total_extensions: added.extensions_enabled,
        was_duplicate,
    };
    Ok(Mutated::wrote(
        ok(reply).with_diagnostics(reported),
        Written::files(added.writes).with_event(event),
    ))
}

#[derive(Debug, Deserialize)]
pub struct RemoveArgs {
    name: String,
    #[serde(default, deserialize_with = "lenient")]
    force: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    dry_run: Option<bool>,
}

pub(crate) fn remove_extension_op(call: &mut Call<'_>, args: RemoveArgs) -> MutationHandled {
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
                    return Ok(Mutated::preview(ok(result)));
                }
                Mutated::wrote(
                    ok(result),
                    Written::files(outcome.writes).with_entities(outcome.orphaned),
                )
            }
            // A removal that failed after editing specforge.json reports it.
            Err(error) => Mutated::refused_after(dry_run, error),
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
}

pub(crate) fn migrate_op(call: &mut Call<'_>, args: MigrateArgs) -> MutationHandled {
    // The project the call migrates, and the runtime its hooks run in.
    let project = call.project()?;
    let path = project.root;
    let dry_run = args.dry_run.unwrap_or(false);
    let no_backup = args.no_backup.unwrap_or(false);
    // The format version to migrate to, checked as `specforge migrate
    // --target-version` checks it.
    let target = match specforge_ops::migrate::parse_target(args.target_version.as_deref()) {
        Ok(target) => target,
        Err(error) => return Ok(Mutated::refused_after(dry_run, error)),
    };

    if !path.join("specforge.json").is_file() {
        return Ok(Mutated::refused_unless_preview(
            dry_run,
            ToolOutcome::no_project("no specforge.json found in the project root"),
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
    // behind the target, the project is current and nothing ran: a
    // migration that wrote nothing (a dry run is a preview).
    let migration = |reply: ToolOutcome, writes: specforge_ops::Writes| match dry_run {
        true => Mutated::preview(reply),
        false => Mutated::wrote(reply, Written::files(writes)),
    };
    if !outcome.pending {
        let current = json!({
            "from_version": from,
            "to_version": to,
            "migrated": false,
            "dry_run": dry_run,
            "changes": [],
            "message": "project is already at the latest format version",
        });
        return Ok(migration(ok(current), outcome.writes));
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
    // A failed run's report rides in `data`, and what it left written (its
    // backups after a rollback, the files migrated before a failure) is
    // reported.
    let reply = if outcome.failed() {
        let (code, message) = if outcome.post_errors().next().is_some() {
            (
                ErrorCode::CompilationFailed,
                "the migrated project does not compile",
            )
        } else {
            (ErrorCode::InternalError, "the migration failed")
        };
        McpError::new(code, message).with_data(result).into()
    } else {
        ok(result)
    };
    Ok(migration(reply, outcome.writes))
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
pub struct DoctorArgs {}

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
}

pub(crate) fn collect_op(call: &mut Call<'_>, args: CollectArgs) -> Handled {
    use specforge_ops::collect::{self, Consent, Mode, Request, RunnerOutput};

    let runner = args.runner.as_deref().filter(|r| *r != "auto");
    let run = args.run.unwrap_or(false);

    // Tests map to the entities on disk now: the target brought the served
    // project up to date, or compiled the project `path` names for this
    // call, in the runtime it collects with.
    let project = call.project()?;
    let request = Request {
        runner,
        mode: if run {
            // The server owns stdio: the runner's output is discarded.
            Mode::Run(RunnerOutput::Discard)
        } else {
            Mode::NoRun
        },
        // The server never prompts: a command runs only if the user already
        // approved it for this project with `specforge collect` in a terminal.
        consent: Consent::Approved,
        announce: &mut |_, _| {},
    };
    Ok(
        match collect::collect(&project.view(), project.runtime.as_ref(), request) {
            Ok(outcome) => ok(outcome.to_json()),
            Err(mut e) => {
                if e.is(codes::E059) {
                    e.message = format!(
                        "the test command isn't approved for this project; run `specforge collect` \
                         in a terminal once to approve it ({})",
                        e.message
                    );
                }
                McpError::from(e).into()
            }
        },
    )
}

// ── render ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RenderArgs {
    /// Required: a renderer is named, never assumed.
    format: String,
    #[serde(default, deserialize_with = "lenient")]
    out_dir: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    scope: Option<String>,
}

pub(crate) fn render_op(call: &mut Call<'_>, args: RenderArgs) -> ToolOutcome {
    use specforge_ops::export::{FORMAT, Format};

    // The renderers are the export formats, named as `specforge export
    // --format` names them (ADR 0027 D8); `json` is `graph`'s alias.
    let format = match FORMAT.parse(&args.format) {
        Ok(format) => format,
        Err(error) => {
            let mut refusal = McpError::from(error).with_argument("format");
            let mut data = refusal.data.take().unwrap_or_else(|| json!({}));
            data["available_renderers"] = json!(FORMAT.accepted().collect::<Vec<_>>());
            return refusal.with_data(data).into();
        }
    };
    // The file each renderer writes into out_dir.
    let file_name = match format {
        Format::Graph => "graph.json",
        Format::Dot => "graph.dot",
        Format::Context => "context.json",
        Format::Brief => "brief.json",
    };
    let name = FORMAT.name_of(format);

    // `graph` is the full graph export: Graph Protocol 2.0 with the schema,
    // as `specforge export --format graph` writes it.
    let request = specforge_ops::export::Request {
        format: Some(format),
        scope: args.scope.as_deref(),
        ..specforge_ops::export::Request::default()
    };
    let output = match specforge_ops::export::export(&call.view(), &request) {
        Ok(text) => text,
        Err(e) => return McpError::from(e).into(),
    };

    // With out_dir the rendering lands on disk; without it, inline.
    let Some(out_dir) = args.out_dir.as_deref() else {
        return ok(json!({ "format": name, "output": output, "output_files": [] }));
    };
    let out_dir = PathBuf::from(out_dir);
    let path = out_dir.join(file_name);
    if let Err(e) = std::fs::create_dir_all(&out_dir).and_then(|()| std::fs::write(&path, output)) {
        return fail(
            ErrorCode::InternalError,
            format!("failed to write {}: {e}", path.display()),
        );
    }
    ok(json!({ "format": name, "output_files": [path.display().to_string()] }))
}
