//! The core tools: one entry per tool holds its name, description, output
//! schema and effect (what it does, with its handler and its arguments; a
//! mutation's handler says what it wrote, ADR 0022). A tool's category,
//! annotations, call target and input schema derive from it (ADR 0024, ADR
//! 0033). The listing, dispatch and events all derive from it.

use serde_json::json;

use super::*;
use crate::args::NoArgs;
use crate::target::ProjectTarget;
use crate::tool::{Effect, Handler, MutationHandler, ToolGroup, ToolSpec, WriteHints};

/// A handler that reads no project, given its typed arguments only: refused
/// when they don't parse. It returns an outcome, or `Handled` to use `?`.
macro_rules! unscoped {
    ($handler:path, $args:ty) => {
        Handler::Unscoped {
            arguments: <$args as crate::args::Arguments>::declared,
            run: |_, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::tool::IntoOutcome::into_outcome($handler(args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
}

/// A handler given the project view of its target (the empty session's with
/// nothing served) and its typed arguments: refused when they don't parse.
/// It returns an outcome, or `Handled` to use `?`.
macro_rules! view {
    ($handler:path, $args:ty, $target:expr) => {
        Handler::View {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::tool::IntoOutcome::into_outcome($handler(call.view(), args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
}

/// A handler given the project it acts on and its typed arguments: refused
/// when they don't parse. It returns an outcome, or `Handled` to use `?`.
macro_rules! project {
    ($handler:path, $args:ty, $target:expr) => {
        Handler::Project {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                // The call target refused a call with no project before
                // this runs.
                Ok(args) => match call.project() {
                    Ok(project) => crate::tool::IntoOutcome::into_outcome($handler(&project, args)),
                    Err(refused) => refused.into(),
                },
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
}

/// A mutation handler given the project it writes and its typed arguments:
/// a failed mutation when they don't parse (arguments that do not parse
/// cannot say they asked for a preview). It returns `Mutated`, or
/// `MutationHandled` to use `?`.
macro_rules! mutation {
    ($handler:path, $args:ty, $target:expr) => {
        MutationHandler::Project {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => match call.project() {
                    Ok(project) => {
                        crate::mutation::IntoMutated::into_mutated($handler(&project, args))
                    }
                    Err(refused) => crate::mutation::Mutated::refused(refused),
                },
                Err(refused) => {
                    crate::mutation::Mutated::refused(crate::tool::ToolOutcome::Refused(refused))
                }
            },
        }
    };
}

/// A mutation handler given the directory it creates a project in, the
/// runtime its extensions' declarations are read in, and its typed
/// arguments.
macro_rules! create {
    ($handler:path, $args:ty) => {
        MutationHandler::New {
            arguments: <$args as crate::args::Arguments>::declared,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => match call.new_project_dir() {
                    Some(dir) => crate::mutation::IntoMutated::into_mutated($handler(
                        dir,
                        &call.runtime(),
                        args,
                    )),
                    // The call target refused a call without a path.
                    None => crate::mutation::Mutated::refused(crate::tool::McpError::from(
                        crate::target::TargetError::PathRequired,
                    )),
                },
                Err(refused) => {
                    crate::mutation::Mutated::refused(crate::tool::ToolOutcome::Refused(refused))
                }
            },
        }
    };
}

pub static CORE_TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "specforge.query",
        description: "Query the graph at multiple resolutions",
        output: Some(
            || json!({ "type": "object", "properties": { "nodes": { "type": "array" }, "edges": { "type": "array" }, "schema_version": { "type": "string" }, "format_version": { "type": "string" } }, "required": ["nodes", "edges"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(query::call, query::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.validate",
        description: "Recompile and validate the spec project",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(
                validate::call,
                validate::Args,
                ProjectTarget::ANY_UNLESS_CACHED
            ),
        },
    },
    ToolSpec {
        name: "specforge.analyze",
        description: "Run analysis passes (coverage: proof obligations and discharge funnel; contracts: clause symmetry) over the compiled project",
        output: Some(
            || json!({ "type": "object", "properties": { "ok": { "type": "boolean" }, "passes": { "type": "array" }, "gate": { "type": "object" }, "stray_records": { "type": "array" } }, "required": ["ok", "passes"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(
                analyze::call,
                analyze::Args,
                ProjectTarget::ANY_UNLESS_CACHED
            ),
        },
    },
    ToolSpec {
        name: "specforge.export",
        description: "Export the graph in various formats",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(export::call, export::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.trace",
        description: "Show traceability chain for an entity, or check an agent plan for gaps (entity_id or plan)",
        // An entity's chain is the document `specforge trace <entity>
        // --format json` writes; a plan's check is an McpTracePlanResult.
        output: Some(|| {
            json!({
                "type": "object",
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "schema_version": { "type": "string" },
                            "entity_id": { "type": "string" },
                            "entity_kind": { "type": "string" },
                            "upstream": { "type": "array", "items": { "type": "object" } },
                            "downstream": { "type": "array", "items": { "type": "object" } },
                            "missing": { "type": "array", "items": { "type": "object" } }
                        },
                        "required": ["schema_version", "entity_id", "entity_kind", "upstream", "downstream", "missing"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "affected_entities": { "type": "array", "items": { "type": "string" } },
                            "gaps": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "source_entity": { "type": "string" },
                                        "target_entity": { "type": "string" },
                                        "missing_link_type": { "type": "string" },
                                        "gap_context": { "type": "string" }
                                    },
                                    "required": ["source_entity", "target_entity", "missing_link_type"]
                                }
                            }
                        },
                        "required": ["affected_entities", "gaps"]
                    }
                ]
            })
        }),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(trace::call, trace::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.search",
        description: "Find entities by id, title or string field text, ranked as the LSP ranks them (exact, prefix, substring, field text, then fuzzy)",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(search::call, search::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.explain",
        description: "Explain a diagnostic code: its title, owner, level, what triggers it and how to fix it, and its docs link",
        output: Some(|| {
            let entry = json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string" },
                    "title": { "type": "string" },
                    "owner": { "type": "string" },
                    "level": { "type": "string" },
                    "explanation": { "type": "string" },
                    "docs": { "type": ["string", "null"] }
                },
                "required": ["code", "title", "owner", "level", "explanation", "docs"]
            });
            json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string" },
                    "retired": { "type": "boolean" },
                    "title": { "type": "string" },
                    "owner": { "type": "string" },
                    "level": { "type": "string" },
                    "explanation": { "type": "string" },
                    "docs": { "type": ["string", "null"] },
                    "replaced_by": { "type": ["object", "null"], "properties": entry["properties"].clone() }
                },
                "required": ["code", "retired"]
            })
        }),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: unscoped!(explain::call, explain::Args),
        },
    },
    ToolSpec {
        name: "specforge.schema",
        description: "Get the GraphProtocolSchema: entity kinds with their typed fields, edge types and the loaded extensions",
        output: Some(
            || json!({ "type": "object", "properties": { "schema_version": { "type": "object" }, "extensions": { "type": "array" }, "entity_kinds": { "type": "array" }, "edge_types": { "type": "array" }, "validation_rules": { "type": "array" } }, "required": ["schema_version", "extensions", "entity_kinds"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(schema::call, schema::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.model",
        description: "Render the logical data model (entity kinds, fields, relationships)",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(model::call, model::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.outline_extensions",
        description: "Renders the extension architecture hierarchy — how extensions relate via dependencies, enhancements, and cross-extension edges. Shows entity kinds, edge types, validation rules, and surface contributions per extension. Use this to understand the project's extension topology before making structural changes.",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(
                outline_extensions::call,
                outline_extensions::Args,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.coverage",
        description: "Get coverage status per entity",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(coverage::call, coverage::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.stats",
        description: "Get project statistics",
        output: Some(
            || json!({ "type": "object", "properties": { "entity_counts": { "type": "array" }, "declared_pct": { "type": "number" }, "proof_pct": { "type": ["number", "null"] }, "coverage_pct": { "type": "number" }, "edge_count": { "type": "integer" }, "unconnected_count": { "type": "integer" }, "diagnostic_summary": { "type": "object" } }, "required": ["entity_counts", "declared_pct", "proof_pct", "edge_count", "unconnected_count", "diagnostic_summary"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(stats::call, NoArgs, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.list",
        description: "List entities sorted by id, optionally filtered by kind and field values, and paged",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(list::call, list::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.inspect",
        description: "Get full detail for a specific entity",
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "kind": { "type": "string" }, "title": { "type": ["string", "null"] }, "testable": { "type": "boolean" }, "declared": { "type": "boolean" }, "exempt": { "type": "boolean" }, "obligated": { "type": "boolean" }, "source_extension": { "type": ["string", "null"] }, "reference_count": { "type": "integer" }, "source_span": { "type": "object" }, "contract": { "type": ["string", "null"] }, "fields": { "type": "object" }, "verify_declarations": { "type": ["array", "null"] }, "referenced_by": { "type": "array", "items": { "type": "string" } }, "refers_to": { "type": "array", "items": { "type": "string" } }, "references": { "type": "array" }, "coverage_status": { "type": "string" }, "diagnostics": { "type": "array" } }, "required": ["entity_id", "kind", "testable", "declared", "exempt", "obligated", "source_span", "fields", "referenced_by", "refers_to", "references", "coverage_status", "diagnostics"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(inspect::call, inspect::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.find_definition",
        description: "Find the source location of an entity definition",
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "file_path": { "type": "string" }, "line": { "type": "integer" }, "column": { "type": "integer" }, "source_span": { "type": "object" }, "name_span": { "type": "object" }, "precision": { "type": "string", "enum": ["token", "entity"] } }, "required": ["entity_id", "file_path", "line", "column", "source_span", "name_span", "precision"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(
                find_definition::call,
                find_definition::Args,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.find_references",
        description: "Find the references to an entity: each place another entity's field names it, as the identifier token (what an IDE's find-references shows)",
        output: Some(|| {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string" },
                    "direction": { "type": "string" },
                    "locations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "referencing_entity_id": { "type": "string" },
                                "referenced_entity_id": { "type": "string" },
                                "field": { "type": ["string", "null"] },
                                "role": { "type": "string", "enum": ["declaration", "reference"] },
                                "precision": { "type": "string", "enum": ["token", "entity"] },
                                "source_span": { "type": "object" }
                            },
                            "required": ["referencing_entity_id", "referenced_entity_id", "field", "role", "precision", "source_span"]
                        }
                    }
                },
                "required": ["entity_id", "direction", "locations"]
            })
        }),
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(
                find_references::call,
                find_references::Args,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.outline",
        description: "Get entity outline for a file",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(outline::call, outline::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.suggest_fixes",
        description: "The fixes the LSP offers as code actions for the project's diagnostics and entities, each with its edits",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(
                suggest_fixes::call,
                suggest_fixes::Args,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.format",
        description: "Format spec files",
        output: Some(
            || json!({ "type": "object", "properties": { "changed_files": { "type": "array" }, "total_checked": { "type": "integer" }, "ok": { "type": "boolean" }, "all_clean": { "type": "boolean" }, "check_only": { "type": "boolean" }, "diagnostics": { "type": "array" }, "diffs": { "type": "array" } }, "required": ["changed_files", "total_checked", "ok", "all_clean", "check_only", "diagnostics"] }),
        ),
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(format::call, format::Args, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.rename",
        description: "Rename an entity across all files",
        output: Some(
            || json!({ "type": "object", "properties": { "old_name": { "type": "string" }, "new_name": { "type": "string" }, "affected_files": { "type": "array" }, "edits": { "type": "array" }, "dry_run": { "type": "boolean" }, "diagnostics": { "type": "array" } }, "required": ["old_name", "new_name", "affected_files", "edits"] }),
        ),
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(rename::call, rename::Args, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.init",
        description: "Initialize a new SpecForge project",
        output: Some(
            || json!({ "type": "object", "properties": { "project_path": { "type": "string" }, "config_file": { "type": "string" }, "starter_file": { "type": "string" }, "extensions_installed": { "type": "array" }, "name": { "type": "string" }, "version": { "type": "string" } }, "required": ["project_path", "config_file", "starter_file", "extensions_installed", "name", "version"] }),
        ),
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: false,
                idempotent: true,
                open_world: false,
            },
            handler: create!(init::call, init::Args),
        },
    },
    ToolSpec {
        name: "specforge.add_extension",
        description: "Install an extension",
        output: Some(
            || json!({ "type": "object", "properties": { "extension": { "type": "string" }, "installed": { "type": "boolean" }, "source": { "type": "string" }, "version": { "type": ["string", "null"] }, "changed": { "type": "boolean" }, "peers_enabled": { "type": "array" }, "note": { "type": "string" }, "sha256": { "type": "string" }, "key_id": { "type": ["string", "null"] }, "dry_run": { "type": "boolean" }, "already_present": { "type": "boolean" }, "message": { "type": "string" } }, "required": ["extension", "installed"] }),
        ),
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: true,
            },
            handler: mutation!(add_extension::call, add_extension::Args, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.remove_extension",
        description: "Remove an installed extension",
        output: Some(
            || json!({ "type": "object", "properties": { "removed_extension": { "type": "string" }, "success": { "type": "boolean" }, "version": { "type": ["string", "null"] }, "stranded": { "type": "array", "items": { "type": "object" } }, "dry_run": { "type": "boolean" } }, "required": ["removed_extension", "success", "stranded"] }),
        ),
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(
                remove_extension::call,
                remove_extension::Args,
                ProjectTarget::ANY
            ),
        },
    },
    ToolSpec {
        name: "specforge.migrate",
        description: "Run migration pipeline",
        output: Some(
            || json!({ "type": "object", "properties": { "ok": { "type": "boolean" }, "from_version": { "type": "string" }, "to_version": { "type": "string" }, "migrated": { "type": "boolean" }, "dry_run": { "type": "boolean" }, "message": { "type": "string" }, "changes": { "type": "array" }, "files_migrated": { "type": "integer" }, "files_skipped": { "type": "integer" }, "files_failed": { "type": "integer" }, "results": { "type": "array" }, "diffs": { "type": "array" }, "rolled_back": { "type": "boolean" }, "post_migration_validated": { "type": "boolean" }, "post_migration_errors": { "type": "array" } }, "required": ["ok", "from_version", "to_version", "migrated", "dry_run"] }),
        ),
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(migrate::call, migrate::Args, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.extensions",
        description: "List installed extensions",
        output: Some(
            || json!({ "type": "object", "properties": { "extensions": { "type": "array" }, "lock_file_entries": { "type": "array" }, "entity_kinds_in_graph": { "type": "array" } }, "required": ["extensions", "lock_file_entries", "entity_kinds_in_graph"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Management,
            handler: project!(extensions::call, NoArgs, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.providers",
        description: "List configured providers",
        output: Some(
            || json!({ "type": "object", "properties": { "providers": { "type": "array" }, "count": { "type": "integer" }, "diagnostics": { "type": "array" } }, "required": ["providers", "count", "diagnostics"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Management,
            handler: project!(providers::call, NoArgs, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.doctor",
        description: "Run health checks",
        output: Some(
            || json!({ "type": "object", "properties": { "ok": { "type": "boolean" }, "extensions_ok": { "type": "boolean" }, "conflicts": { "type": "array" }, "cache_status": { "type": "string" }, "findings": { "type": "array" }, "installed_count": { "type": "integer" }, "extensions": { "type": "array" }, "enhancements": { "type": "object" }, "z3_available": { "type": "boolean" } }, "required": ["ok", "extensions_ok", "conflicts", "cache_status", "findings"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Management,
            handler: project!(doctor::call, NoArgs, ProjectTarget::SERVED_UNLESS_CACHED),
        },
    },
    ToolSpec {
        name: "specforge.collect",
        description: "Record which entities the project's tests prove, from the test runner's report (runs the runner only with run: true and prior approval)",
        output: Some(|| {
            json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string", "enum": ["collected"] },
                    "runners": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "extension": { "type": "string" },
                                "ran": { "type": "boolean" },
                                "exit_code": { "type": "integer" },
                                "files": { "type": "integer" },
                                "entities": { "type": "integer" },
                                "passed": { "type": "integer" },
                                "failed": { "type": "integer" },
                                "skipped": { "type": "integer" },
                                "by_convention": { "type": "integer" }
                            },
                            "required": ["name", "extension", "ran", "files", "entities", "passed", "failed", "skipped", "by_convention"]
                        }
                    },
                    "diagnostics": { "type": "array" },
                    "report": { "type": "string" }
                },
                "required": ["status", "runners", "diagnostics", "report"]
            })
        }),
        effect: Effect::WritesOutput {
            group: ToolGroup::Management,
            hints: WriteHints {
                destructive: true,
                idempotent: false,
                open_world: true,
            },
            handler: project!(collect::call, collect::Args, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.render",
        description: "Render output in a specified format",
        output: Some(
            || json!({ "type": "object", "properties": { "format": { "type": "string" }, "output": { "type": "string" }, "output_files": { "type": "array" } }, "required": ["format", "output_files"] }),
        ),
        effect: Effect::WritesOutput {
            group: ToolGroup::Management,
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: view!(render::call, render::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.infer_progress",
        description: "Check inference progress: summary of analyzed vs unanalyzed source files, stale entries, and entity counts",
        output: Some(
            || json!({ "type": "object", "properties": { "summary": { "type": "object" }, "unanalyzed": { "type": "array" }, "stale": { "type": "array" }, "deleted": { "type": "array" }, "sessions": { "type": "array" }, "message": { "type": "string" } }, "required": ["summary", "unanalyzed", "stale", "deleted"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(infer_progress::call, NoArgs, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.infer_gaps",
        description: "Analyze inference gaps: public Rust items not yet covered by spec entities (approximate)",
        output: Some(
            || json!({ "type": "object", "properties": { "total_pub_items": { "type": "integer" }, "covered_items": { "type": "integer" }, "gap_count": { "type": "integer" }, "approximate": { "type": "boolean" }, "scanners_used": { "type": "array" }, "scan_failures": { "type": "array" }, "by_directory": { "type": "array" }, "gaps": { "type": "array" }, "message": { "type": "string" } }, "required": ["total_pub_items", "covered_items", "approximate"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(infer_gaps::call, NoArgs, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.infer_session",
        description: "Manage inference sessions: start a new session, mark files as analyzed, or end a session",
        output: Some(
            || json!({ "type": "object", "properties": { "session_id": { "type": "string" }, "status": { "type": "string" }, "source_file": { "type": "string" }, "entities_produced": { "type": "array" } }, "required": ["status"] }),
        ),
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: false,
                open_world: false,
            },
            handler: mutation!(
                infer_session::call,
                infer_session::Args,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.find_implementation",
        description: "Find source code locations that implement a specforge entity",
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "implementations": { "type": "array" }, "count": { "type": "integer" } }, "required": ["entity_id", "implementations", "count"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: project!(
                find_implementation::call,
                find_implementation::Args,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.find_spec_for_source",
        description: "Find specforge entities anchored to a source file",
        output: Some(
            || json!({ "type": "object", "properties": { "file_path": { "type": "string" }, "match_mode": { "type": "string", "enum": ["exact", "directory", "suffix_path", "none"] }, "entities": { "type": "array" }, "count": { "type": "integer" } }, "required": ["file_path", "match_mode", "entities", "count"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: project!(
                find_spec_for_source::call,
                find_spec_for_source::Args,
                ProjectTarget::SERVED
            ),
        },
    },
];
