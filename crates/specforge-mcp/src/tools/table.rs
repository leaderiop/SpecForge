//! The core tools: one entry per tool holds its name, description,
//! category, output schema, target and handler with its arguments (a
//! mutation's says what it wrote, ADR 0022). A tool's input schema is derived
//! from its typed arguments (ADR 0033). The listing, dispatch and events all
//! derive from it.

use serde_json::{Value, json};

use super::*;
use crate::args::NoArgs;
use crate::operations;
use crate::target::{Freshness, Reach, TargetSpec};
use crate::tool::{Access, Category, Handler, ToolSpec};

/// A handler reading its typed arguments: refused when they don't parse.
/// It returns an outcome, or `Handled` to use `?`.
macro_rules! typed {
    ($handler:path, $args:ty) => {
        Handler::Tool {
            arguments: <$args as crate::args::Arguments>::declared,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::tool::IntoOutcome::into_outcome($handler(call, args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
}

/// A mutation handler reading its typed arguments: a failed mutation when
/// they don't parse (arguments that do not parse cannot say they asked for
/// a preview). It returns `Mutated`, or `MutationHandled` to use `?`.
macro_rules! mutation {
    ($handler:path, $args:ty) => {
        Handler::Mutation {
            arguments: <$args as crate::args::Arguments>::declared,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::mutation::IntoMutated::into_mutated($handler(call, args)),
                Err(refused) => {
                    crate::mutation::Mutated::refused(crate::tool::ToolOutcome::Refused(refused))
                }
            },
        }
    };
}

/// The output-schema property every mutation reply that wrote carries:
/// the files it wrote, relative to the project root.
fn files_written() -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": "The files the call created, rewrote or removed, relative to the project root (absolute outside it); absent from a preview" })
}

/// `schema` (an object schema) with the `files_written` property.
fn with_files_written(mut schema: Value) -> Value {
    schema["properties"][crate::mutation::FILES_WRITTEN] = files_written();
    schema
}

pub static CORE_TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "specforge.query",
        description: "Query the graph at multiple resolutions",
        category: Category::Core,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "nodes": { "type": "array" }, "edges": { "type": "array" }, "schema_version": { "type": "string" }, "format_version": { "type": "string" } }, "required": ["nodes", "edges"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(query::call, query::Args),
    },
    ToolSpec {
        name: "specforge.validate",
        description: "Recompile and validate the spec project",
        category: Category::Core,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::new(Reach::AnyProject, Freshness::FreshUnlessCached),
        handler: typed!(validate::call, validate::Args),
    },
    ToolSpec {
        name: "specforge.analyze",
        description: "Run analysis passes (coverage: proof obligations and discharge funnel; contracts: clause symmetry) over the compiled project",
        category: Category::Core,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "ok": { "type": "boolean" }, "passes": { "type": "array" }, "orphans": { "type": "array" } }, "required": ["ok", "passes"] }),
        ),
        target: TargetSpec::new(Reach::AnyProject, Freshness::FreshUnlessCached),
        handler: typed!(analyze::call, analyze::Args),
    },
    ToolSpec {
        name: "specforge.export",
        description: "Export the graph in various formats",
        category: Category::Core,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(export::call, export::Args),
    },
    ToolSpec {
        name: "specforge.trace",
        description: "Show traceability chain for an entity, or check an agent plan for gaps (entity_id or plan)",
        category: Category::Core,
        access: Access::ReadOnly,
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
        target: TargetSpec::SERVED,
        handler: typed!(trace::call, trace::Args),
    },
    ToolSpec {
        name: "specforge.search",
        description: "Find entities by id, title or string field text, ranked as the LSP ranks them (exact, prefix, substring, field text, then fuzzy)",
        category: Category::Core,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(search::call, search::Args),
    },
    ToolSpec {
        name: "specforge.explain",
        description: "Explain a diagnostic code: its title, owner, level, what triggers it and how to fix it, and its docs link",
        category: Category::Core,
        access: Access::ReadOnly,
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
        target: TargetSpec::UNSCOPED,
        handler: typed!(explain::call, explain::Args),
    },
    ToolSpec {
        name: "specforge.schema",
        description: "Get the GraphProtocolSchema: entity kinds with their typed fields, edge types and the loaded extensions",
        category: Category::Core,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "schema_version": { "type": "object" }, "extensions": { "type": "array" }, "entity_kinds": { "type": "array" }, "edge_types": { "type": "array" }, "validation_rules": { "type": "array" } }, "required": ["schema_version", "extensions", "entity_kinds"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(schema::call, schema::Args),
    },
    ToolSpec {
        name: "specforge.model",
        description: "Render the logical data model (entity kinds, fields, relationships)",
        category: Category::Core,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(model::call, model::Args),
    },
    ToolSpec {
        name: "specforge.outline_extensions",
        description: "Renders the extension architecture hierarchy — how extensions relate via dependencies, enhancements, and cross-extension edges. Shows entity kinds, edge types, validation rules, and surface contributions per extension. Use this to understand the project's extension topology before making structural changes.",
        category: Category::Core,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(outline_extensions::call, outline_extensions::Args),
    },
    ToolSpec {
        name: "specforge.coverage",
        description: "Get coverage status per entity",
        category: Category::Core,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(coverage::call, coverage::Args),
    },
    ToolSpec {
        name: "specforge.stats",
        description: "Get project statistics",
        category: Category::Core,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "entity_counts": { "type": "array" }, "declared_pct": { "type": "number" }, "proof_pct": { "type": ["number", "null"] }, "coverage_pct": { "type": "number" }, "edge_count": { "type": "integer" }, "unconnected_count": { "type": "integer" }, "diagnostic_summary": { "type": "object" } }, "required": ["entity_counts", "declared_pct", "proof_pct", "edge_count", "unconnected_count", "diagnostic_summary"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(stats::call, NoArgs),
    },
    ToolSpec {
        name: "specforge.list",
        description: "List entities sorted by id, optionally filtered by kind and field values, and paged",
        category: Category::Core,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(list::call, list::Args),
    },
    ToolSpec {
        name: "specforge.inspect",
        description: "Get full detail for a specific entity",
        category: Category::Navigation,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "kind": { "type": "string" }, "title": { "type": ["string", "null"] }, "testable": { "type": "boolean" }, "declared": { "type": "boolean" }, "exempt": { "type": "boolean" }, "obligated": { "type": "boolean" }, "source_extension": { "type": ["string", "null"] }, "reference_count": { "type": "integer" }, "source_span": { "type": "object" }, "contract": { "type": ["string", "null"] }, "fields": { "type": "object" }, "verify_declarations": { "type": ["array", "null"] }, "referenced_by": { "type": "array", "items": { "type": "string" } }, "refers_to": { "type": "array", "items": { "type": "string" } }, "references": { "type": "array" }, "coverage_status": { "type": "string" }, "diagnostics": { "type": "array" } }, "required": ["entity_id", "kind", "testable", "declared", "exempt", "obligated", "source_span", "fields", "referenced_by", "refers_to", "references", "coverage_status", "diagnostics"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(inspect::call, inspect::Args),
    },
    ToolSpec {
        name: "specforge.find_definition",
        description: "Find the source location of an entity definition",
        category: Category::Navigation,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "file_path": { "type": "string" }, "line": { "type": "integer" }, "column": { "type": "integer" }, "source_span": { "type": "object" }, "name_span": { "type": "object" }, "precision": { "type": "string", "enum": ["token", "entity"] } }, "required": ["entity_id", "file_path", "line", "column", "source_span", "name_span", "precision"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(find_definition::call, find_definition::Args),
    },
    ToolSpec {
        name: "specforge.find_references",
        description: "Find the references to an entity: each place another entity's field names it, as the identifier token (what an IDE's find-references shows)",
        category: Category::Navigation,
        access: Access::ReadOnly,
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
        target: TargetSpec::SERVED,
        handler: typed!(find_references::call, find_references::Args),
    },
    ToolSpec {
        name: "specforge.outline",
        description: "Get entity outline for a file",
        category: Category::Navigation,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(outline::call, outline::Args),
    },
    ToolSpec {
        name: "specforge.suggest_fixes",
        description: "The fixes the LSP offers as code actions for the project's diagnostics and entities, each with its edits",
        category: Category::Navigation,
        access: Access::ReadOnly,
        output: None,
        target: TargetSpec::SERVED,
        handler: typed!(suggest_fixes::call, suggest_fixes::Args),
    },
    ToolSpec {
        name: "specforge.format",
        description: "Format spec files",
        category: Category::Mutation,
        access: Access::Writes {
            destructive: true,
            idempotent: true,
            open_world: false,
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "changed_files": { "type": "array" }, "total_checked": { "type": "integer" }, "ok": { "type": "boolean" }, "all_clean": { "type": "boolean" }, "check_only": { "type": "boolean" }, "diagnostics": { "type": "array" }, "diffs": { "type": "array" } }, "required": ["changed_files", "total_checked", "ok", "all_clean", "check_only", "diagnostics"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        handler: mutation!(operations::format_op, operations::FormatArgs),
    },
    ToolSpec {
        name: "specforge.rename",
        description: "Rename an entity across all files",
        category: Category::Mutation,
        access: Access::Writes {
            destructive: true,
            idempotent: false,
            open_world: false,
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "old_name": { "type": "string" }, "new_name": { "type": "string" }, "affected_files": { "type": "array" }, "edits": { "type": "array" }, "dry_run": { "type": "boolean" }, "diagnostics": { "type": "array" } }, "required": ["old_name", "new_name", "affected_files", "edits"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        handler: mutation!(operations::rename_op, operations::RenameArgs),
    },
    ToolSpec {
        name: "specforge.init",
        description: "Initialize a new SpecForge project",
        category: Category::Mutation,
        access: Access::Writes {
            destructive: false,
            idempotent: true,
            open_world: false,
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "project_path": { "type": "string" }, "config_file": { "type": "string" }, "starter_file": { "type": "string" }, "extensions_installed": { "type": "array" }, "name": { "type": "string" }, "version": { "type": "string" } }, "required": ["project_path", "config_file", "starter_file", "extensions_installed", "name", "version"] }),
            )
        }),
        target: TargetSpec::new(Reach::NewProject, Freshness::Fresh),
        handler: mutation!(operations::init_op, operations::InitArgs),
    },
    ToolSpec {
        name: "specforge.add_extension",
        description: "Install an extension",
        category: Category::Mutation,
        access: Access::Writes {
            destructive: false,
            idempotent: true,
            open_world: true,
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "extension": { "type": "string" }, "installed": { "type": "boolean" }, "source": { "type": "string" }, "version": { "type": ["string", "null"] }, "changed": { "type": "boolean" }, "peers_enabled": { "type": "array" }, "note": { "type": "string" }, "sha256": { "type": "string" }, "key_id": { "type": ["string", "null"] }, "dry_run": { "type": "boolean" }, "already_present": { "type": "boolean" }, "message": { "type": "string" } }, "required": ["extension", "installed"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        handler: mutation!(operations::add_extension, operations::AddArgs),
    },
    ToolSpec {
        name: "specforge.remove_extension",
        description: "Remove an installed extension",
        category: Category::Mutation,
        access: Access::Writes {
            destructive: true,
            idempotent: true,
            open_world: false,
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "removed_extension": { "type": "string" }, "success": { "type": "boolean" }, "version": { "type": ["string", "null"] }, "orphan_warnings": { "type": "array" }, "dry_run": { "type": "boolean" } }, "required": ["removed_extension", "success", "orphan_warnings"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        handler: mutation!(operations::remove_extension_op, operations::RemoveArgs),
    },
    ToolSpec {
        name: "specforge.migrate",
        description: "Run migration pipeline",
        category: Category::Mutation,
        access: Access::Writes {
            destructive: true,
            idempotent: true,
            open_world: false,
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "ok": { "type": "boolean" }, "from_version": { "type": "string" }, "to_version": { "type": "string" }, "migrated": { "type": "boolean" }, "dry_run": { "type": "boolean" }, "message": { "type": "string" }, "changes": { "type": "array" }, "files_migrated": { "type": "integer" }, "files_skipped": { "type": "integer" }, "files_failed": { "type": "integer" }, "results": { "type": "array" }, "diffs": { "type": "array" }, "rolled_back": { "type": "boolean" }, "post_migration_validated": { "type": "boolean" }, "post_migration_errors": { "type": "array" } }, "required": ["ok", "from_version", "to_version", "migrated", "dry_run"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        handler: mutation!(operations::migrate_op, operations::MigrateArgs),
    },
    ToolSpec {
        name: "specforge.extensions",
        description: "List installed extensions",
        category: Category::Management,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "extensions": { "type": "array" }, "lock_file_entries": { "type": "array" }, "entity_kinds_in_graph": { "type": "array" } }, "required": ["extensions", "lock_file_entries", "entity_kinds_in_graph"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(operations::extensions_op, NoArgs),
    },
    ToolSpec {
        name: "specforge.providers",
        description: "List configured providers",
        category: Category::Management,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "providers": { "type": "array" }, "count": { "type": "integer" }, "diagnostics": { "type": "array" } }, "required": ["providers", "count", "diagnostics"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(operations::providers_op, NoArgs),
    },
    ToolSpec {
        name: "specforge.doctor",
        description: "Run health checks",
        category: Category::Management,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "extensions_ok": { "type": "boolean" }, "conflicts": { "type": "array" }, "cache_status": { "type": "string" }, "findings": { "type": "array" }, "installed_count": { "type": "integer" }, "extensions": { "type": "array" }, "enhancements": { "type": "object" }, "shadowed": { "type": "array" }, "load_failures": { "type": "array" }, "issues": { "type": "array" }, "z3_available": { "type": "boolean" } }, "required": ["extensions_ok", "conflicts", "installed_count", "issues", "load_failures"] }),
        ),
        target: TargetSpec::new(Reach::Served, Freshness::FreshUnlessCached),
        handler: typed!(operations::doctor_op, NoArgs),
    },
    ToolSpec {
        name: "specforge.collect",
        description: "Record which entities the project's tests prove, from the test runner's report (runs the runner only with run: true and prior approval)",
        category: Category::Management,
        access: Access::Writes {
            destructive: true,
            idempotent: false,
            open_world: true,
        },
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
        target: TargetSpec::new(Reach::AnyProject, Freshness::Fresh),
        handler: typed!(operations::collect_op, operations::CollectArgs),
    },
    ToolSpec {
        name: "specforge.render",
        description: "Render output in a specified format",
        category: Category::Management,
        access: Access::Writes {
            destructive: true,
            idempotent: true,
            open_world: false,
        },
        output: Some(
            || json!({ "type": "object", "properties": { "format": { "type": "string" }, "output": { "type": "string" }, "output_files": { "type": "array" } }, "required": ["format", "output_files"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(operations::render_op, operations::RenderArgs),
    },
    ToolSpec {
        name: "specforge.infer_progress",
        description: "Check inference progress: summary of analyzed vs unanalyzed source files, stale entries, and entity counts",
        category: Category::Core,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "summary": { "type": "object" }, "unanalyzed": { "type": "array" }, "stale": { "type": "array" }, "deleted": { "type": "array" }, "sessions": { "type": "array" }, "message": { "type": "string" } }, "required": ["summary", "unanalyzed", "stale", "deleted"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(infer_progress::call, NoArgs),
    },
    ToolSpec {
        name: "specforge.infer_gaps",
        description: "Analyze inference gaps: public Rust items not yet covered by spec entities (approximate)",
        category: Category::Core,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "total_pub_items": { "type": "integer" }, "covered_items": { "type": "integer" }, "gap_count": { "type": "integer" }, "approximate": { "type": "boolean" }, "scanners_used": { "type": "array" }, "scan_failures": { "type": "array" }, "by_directory": { "type": "array" }, "gaps": { "type": "array" }, "message": { "type": "string" } }, "required": ["total_pub_items", "covered_items", "approximate"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(infer_gaps::call, NoArgs),
    },
    ToolSpec {
        name: "specforge.infer_session",
        description: "Manage inference sessions: start a new session, mark files as analyzed, or end a session",
        category: Category::Mutation,
        access: Access::Writes {
            destructive: false,
            idempotent: false,
            open_world: false,
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "session_id": { "type": "string" }, "status": { "type": "string" }, "source_file": { "type": "string" }, "entities_produced": { "type": "array" } }, "required": ["status"] }),
            )
        }),
        target: TargetSpec::SERVED,
        handler: mutation!(infer_session::call, infer_session::Args),
    },
    ToolSpec {
        name: "specforge.find_implementation",
        description: "Find source code locations that implement a specforge entity",
        category: Category::Navigation,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "implementations": { "type": "array" }, "count": { "type": "integer" } }, "required": ["entity_id", "implementations", "count"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(find_implementation::call, find_implementation::Args),
    },
    ToolSpec {
        name: "specforge.find_spec_for_source",
        description: "Find specforge entities anchored to a source file",
        category: Category::Navigation,
        access: Access::ReadOnly,
        output: Some(
            || json!({ "type": "object", "properties": { "file_path": { "type": "string" }, "match_mode": { "type": "string", "enum": ["exact", "directory", "suffix_path", "none"] }, "entities": { "type": "array" }, "count": { "type": "integer" } }, "required": ["file_path", "match_mode", "entities", "count"] }),
        ),
        target: TargetSpec::SERVED,
        handler: typed!(find_spec_for_source::call, find_spec_for_source::Args),
    },
];

#[cfg(test)]
mod tests {
    use super::CORE_TOOLS;
    use crate::target::Reach;

    /// A tool that writes the files of the project its path names is a
    /// mutation; one that reads no project takes no path.
    #[test]
    fn a_tool_that_writes_its_target_is_a_mutation() {
        for tool in CORE_TOOLS {
            if tool.target.reach == Reach::WritesAnyProject {
                assert!(
                    matches!(tool.handler, crate::tool::Handler::Mutation { .. }),
                    "{}",
                    tool.name
                );
            }
            if tool.target.reach == Reach::Unscoped {
                assert!(
                    !tool.input_schema()["properties"]
                        .as_object()
                        .is_some_and(|properties| properties.contains_key("path")),
                    "{}",
                    tool.name
                );
            }
        }
    }
}
