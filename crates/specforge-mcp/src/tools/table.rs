//! The core tools: one entry per tool holds its name, description,
//! category, input and output schemas, target and handler (a mutation's
//! says what it wrote, ADR 0022). The listing, dispatch and events all
//! derive from it.

use serde_json::{Value, json};

use super::*;
use crate::args::{NoArgs, choice_schema, fields, names_schema, required_choice_schema};
use crate::operations;
use crate::target::{Freshness, Reach, TargetSpec};
use crate::tool::{Access, Category, Handler, ToolSpec};
use specforge_ops::{export as ops_export, model as ops_model};

/// A handler reading its typed arguments: refused when they don't parse.
/// It returns an outcome, or `Handled` to use `?`.
macro_rules! typed {
    ($handler:path, $args:ty) => {
        Handler::Tool(
            |call, arguments| match crate::args::parse::<$args>(arguments) {
                Ok(args) => crate::tool::IntoOutcome::into_outcome($handler(call, args)),
                Err(refused) => refused,
            },
        )
    };
}

/// A mutation handler reading its typed arguments: a failed mutation when
/// they don't parse (arguments that do not parse cannot say they asked for
/// a preview). It returns `Mutated`, or `MutationHandled` to use `?`.
macro_rules! mutation {
    ($handler:path, $args:ty) => {
        Handler::Mutation(
            |call, arguments| match crate::args::parse::<$args>(arguments) {
                Ok(args) => crate::mutation::IntoMutated::into_mutated($handler(call, args)),
                Err(refused) => crate::mutation::Mutated::refused(refused),
            },
        )
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID to query" },
                    "depth": { "type": "integer", "description": "Number of hops (default 1)", "default": 1 },
                    "kinds": { "type": "array", "items": { "type": "string" }, "description": "Filter by entity kinds" },
                    "format": choice_schema(&ops_export::AGENT_FORMAT, "Output detail level"),
                    "include_coverage": { "type": "boolean", "description": "Include coverage metadata in the response", "default": false }
                },
                "required": ["entity_id"]
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "nodes": { "type": "array" }, "edges": { "type": "array" }, "schema_version": { "type": "string" }, "format_version": { "type": "string" } }, "required": ["nodes", "edges"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<query::Args>,
        handler: typed!(query::call, query::Args),
    },
    ToolSpec {
        name: "specforge.validate",
        description: "Recompile and validate the spec project",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" },
                    "severity_filter": names_schema(specforge_ops::check::SEVERITY_NAMES, "Only report diagnostics of this severity, after strict promotion (case-insensitive). The verdict in _meta[\"specforge/check\"] still counts everything reported"),
                    "strict": { "type": "boolean", "description": "Promote warnings to errors, before severity_filter applies", "default": false },
                    "lint": { "type": "array", "items": names_schema(specforge_project::LINT_PROFILE_NAMES, "A lint profile"), "description": "Extra lint profiles, as `specforge check --lint` takes (inferred: I200/I202 from specforge-infer.json; pedantic is the default and adds nothing)" },
                    "use_cached": { "type": "boolean", "description": "Report cached diagnostics from the last compile instead of recompiling", "default": false }
                }
            })
        },
        output: None,
        target: TargetSpec::new(Reach::AnyProject, Freshness::FreshUnlessCached),
        fields: fields::<validate::Args>,
        handler: typed!(validate::call, validate::Args),
    },
    ToolSpec {
        name: "specforge.analyze",
        description: "Run analysis passes (coverage: proof obligations and discharge funnel; contracts: clause symmetry) over the compiled project",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "pass": { "type": "string", "description": "Analysis pass to run: all, coverage, contracts, or a pass an extension declares (`<extension>:<pass>`)", "default": specforge_ops::analyze::EVERY_PASS },
                    "strict": { "type": "boolean", "description": "Promote warnings to errors" },
                    "test_results": { "type": "string", "description": "Path to a specforge-report.json for proof-level verdicts" },
                    "use_cached": { "type": "boolean", "description": "Analyze the last compiled graph instead of recompiling (a server with no graph compiles anyway)", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                }
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "ok": { "type": "boolean" }, "passes": { "type": "array" }, "orphans": { "type": "array" } }, "required": ["ok", "passes"] }),
        ),
        target: TargetSpec::new(Reach::AnyProject, Freshness::FreshUnlessCached),
        fields: fields::<analyze::Args>,
        handler: typed!(analyze::call, analyze::Args),
    },
    ToolSpec {
        name: "specforge.export",
        description: "Export the graph in various formats",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "format": choice_schema(&ops_export::AGENT_FORMAT, "Export format"),
                    "scope": { "type": "string", "description": "Scope to entity subgraph" },
                    "max_tokens": { "type": "integer", "description": "Optional token budget; truncates the export to the most central entities that fit" },
                    "with_schema": { "type": "boolean", "description": "Embed the Graph Protocol schema in a context, brief or budgeted graph export (a full graph export embeds it already); under max_tokens it counts toward the budget" },
                    "no_schema": { "type": "boolean", "description": "Leave the schema out of a graph export (Graph Protocol 1.0)" }
                }
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<export::Args>,
        handler: typed!(export::call, export::Args),
    },
    ToolSpec {
        name: "specforge.trace",
        description: "Show traceability chain for an entity, or check an agent plan for gaps (entity_id or plan)",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID to trace" },
                    "plan": { "type": "object", "description": "An agent plan, {\"entries\": [{\"entity_id\": ...}]}, to check for gaps against the graph instead of tracing one entity" }
                }
            })
        },
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
        fields: fields::<trace::Args>,
        handler: typed!(trace::call, trace::Args),
    },
    ToolSpec {
        name: "specforge.search",
        description: "Find entities by id, title or string field text, ranked as the LSP ranks them (exact, prefix, substring, field text, then fuzzy)",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "kinds": { "type": "array", "items": { "type": "string" }, "description": "Filter by kinds" },
                    "limit": { "type": "integer", "description": "Max results (default 20)", "default": 20 },
                    "field": { "type": "string", "description": "Only search a specific field" },
                    "value": { "type": "string", "description": "Exact field value filter (with field)" },
                    "references": { "type": "string", "description": "Only entities that reference this entity ID (combined with the other filters)" }
                },
                "required": ["query"]
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<search::Args>,
        handler: typed!(search::call, search::Args),
    },
    ToolSpec {
        name: "specforge.explain",
        description: "Explain a diagnostic code: its title, owner, level, what triggers it and how to fix it, and its docs link",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "A diagnostic code, any case" }
                },
                "required": ["code"]
            })
        },
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
        fields: fields::<explain::Args>,
        handler: typed!(explain::call, explain::Args),
    },
    ToolSpec {
        name: "specforge.schema",
        description: "Get the GraphProtocolSchema: entity kinds with their typed fields, edge types and the loaded extensions",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Filter schema to a specific entity kind" },
                    "include_edges": { "type": "boolean", "description": "Include edge type definitions", "default": specforge_ops::schema::SchemaRequest::default().edges },
                    "include_validation_rules": { "type": "boolean", "description": "Include the validation rules loaded extensions declare", "default": specforge_ops::schema::SchemaRequest::default().validation_rules }
                }
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "schema_version": { "type": "object" }, "extensions": { "type": "array" }, "entity_kinds": { "type": "array" }, "edge_types": { "type": "array" }, "validation_rules": { "type": "array" } }, "required": ["schema_version", "extensions", "entity_kinds"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<schema::Args>,
        handler: typed!(schema::call, schema::Args),
    },
    ToolSpec {
        name: "specforge.model",
        description: "Render the logical data model (entity kinds, fields, relationships)",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "format": choice_schema(&ops_model::MODEL_FORMAT, "Output format"),
                    "group_by": choice_schema(&ops_model::GROUP_BY, "How to group entities"),
                    "fields": choice_schema(&ops_model::MODEL_FIELDS, "Field detail level"),
                    "extension": {
                        "type": "string",
                        "description": "Filter to a single extension"
                    },
                    "kinds": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter to specific entity kinds"
                    },
                    "root": {
                        "type": "string",
                        "description": "Root entity kind for depth-scoped output"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth from root kind (requires root)"
                    }
                }
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<model::Args>,
        handler: typed!(model::call, model::Args),
    },
    ToolSpec {
        name: "specforge.outline_extensions",
        description: "Renders the extension architecture hierarchy — how extensions relate via dependencies, enhancements, and cross-extension edges. Shows entity kinds, edge types, validation rules, and surface contributions per extension. Use this to understand the project's extension topology before making structural changes.",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "format": choice_schema(&ops_model::OUTLINE_FORMAT, "Output format; json is meant for programs"),
                    "fields": choice_schema(&ops_model::OUTLINE_FIELDS, "Detail level"),
                    "deps": choice_schema(&ops_model::DEPS, "Dependency visibility")
                }
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<outline_extensions::Args>,
        handler: typed!(outline_extensions::call, outline_extensions::Args),
    },
    ToolSpec {
        name: "specforge.coverage",
        description: "Get coverage status per entity",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Filter to specific entity" },
                    "kind": { "type": "string", "description": "Filter by entity kind" },
                    "status_filter": choice_schema(&specforge_ops::coverage::STATUS, "Only entities with this coverage status")
                }
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<coverage::Args>,
        handler: typed!(coverage::call, coverage::Args),
    },
    ToolSpec {
        name: "specforge.stats",
        description: "Get project statistics",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {}
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "entity_counts": { "type": "array" }, "declared_pct": { "type": "number" }, "proof_pct": { "type": ["number", "null"] }, "coverage_pct": { "type": "number" }, "edge_count": { "type": "integer" }, "orphan_count": { "type": "integer" }, "diagnostic_summary": { "type": "object" } }, "required": ["entity_counts", "declared_pct", "proof_pct", "edge_count", "orphan_count", "diagnostic_summary"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<NoArgs>,
        handler: typed!(stats::call, NoArgs),
    },
    ToolSpec {
        name: "specforge.list",
        description: "List entities sorted by id, optionally filtered by kind and field values, and paged",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Filter by entity kind (e.g. 'feature', 'behavior')" },
                    "where": {
                        "type": "object",
                        "description": "Only entities whose fields hold these values, e.g. {\"status\": \"done\"}",
                        "additionalProperties": true
                    },
                    "limit": { "type": "integer", "minimum": 0, "description": "Return at most this many entities" },
                    "offset": { "type": "integer", "minimum": 0, "description": "Skip this many entities first" }
                }
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<list::Args>,
        handler: typed!(list::call, list::Args),
    },
    ToolSpec {
        name: "specforge.inspect",
        description: "Get full detail for a specific entity",
        category: Category::Navigation,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID to inspect" }
                },
                "required": ["entity_id"]
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "kind": { "type": "string" }, "title": { "type": ["string", "null"] }, "testable": { "type": "boolean" }, "declared": { "type": "boolean" }, "exempt": { "type": "boolean" }, "obligated": { "type": "boolean" }, "source_extension": { "type": ["string", "null"] }, "reference_count": { "type": "integer" }, "source_span": { "type": "object" }, "contract": { "type": ["string", "null"] }, "fields": { "type": "object" }, "verify_declarations": { "type": ["array", "null"] }, "referenced_by": { "type": "array", "items": { "type": "string" } }, "refers_to": { "type": "array", "items": { "type": "string" } }, "references": { "type": "array" }, "coverage_status": { "type": "string" }, "diagnostics": { "type": "array" } }, "required": ["entity_id", "kind", "testable", "declared", "exempt", "obligated", "source_span", "fields", "referenced_by", "refers_to", "references", "coverage_status", "diagnostics"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<inspect::Args>,
        handler: typed!(inspect::call, inspect::Args),
    },
    ToolSpec {
        name: "specforge.find_definition",
        description: "Find the source location of an entity definition",
        category: Category::Navigation,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID" }
                },
                "required": ["entity_id"]
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "file_path": { "type": "string" }, "line": { "type": "integer" }, "column": { "type": "integer" }, "source_span": { "type": "object" }, "name_span": { "type": "object" }, "precision": { "type": "string", "enum": ["token", "entity"] } }, "required": ["entity_id", "file_path", "line", "column", "source_span", "name_span", "precision"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<find_definition::Args>,
        handler: typed!(find_definition::call, find_definition::Args),
    },
    ToolSpec {
        name: "specforge.find_references",
        description: "Find the references to an entity: each place another entity's field names it, as the identifier token (what an IDE's find-references shows)",
        category: Category::Navigation,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID" },
                    "direction": choice_schema(&specforge_ops::navigate::DIRECTION, "Which references"),
                    "include_declaration": { "type": "boolean", "default": false, "description": "Also return the entity's own declaration (its name)" }
                },
                "required": ["entity_id"]
            })
        },
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
        fields: fields::<find_references::Args>,
        handler: typed!(find_references::call, find_references::Args),
    },
    ToolSpec {
        name: "specforge.outline",
        description: "Get entity outline for a file",
        category: Category::Navigation,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" }
                },
                "required": ["file"]
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<outline::Args>,
        handler: typed!(outline::call, outline::Args),
    },
    ToolSpec {
        name: "specforge.suggest_fixes",
        description: "The fixes the LSP offers as code actions for the project's diagnostics and entities, each with its edits",
        category: Category::Navigation,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID (optional, all if omitted)" },
                    "file_path": { "type": "string", "description": "Only diagnostics in this spec file" },
                    "diagnostic_code": { "type": "string", "description": "Only diagnostics with this code, e.g. W001" }
                }
            })
        },
        output: None,
        target: TargetSpec::SERVED,
        fields: fields::<suggest_fixes::Args>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" },
                    "paths": { "type": "array", "items": { "type": "string" }, "description": "Files or directories to format, relative to the project root (defaults to every spec file)" },
                    "check": { "type": "boolean", "description": "Check only, don't modify", "default": false },
                    "diff": { "type": "boolean", "description": "Return a before/after diff for each file that would change, without modifying it", "default": false },
                    "write": { "type": "boolean", "description": "Write formatted output (defaults to false in check or diff mode)", "default": true }
                }
            })
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "changed_files": { "type": "array" }, "total_checked": { "type": "integer" }, "all_clean": { "type": "boolean" }, "check_only": { "type": "boolean" }, "diagnostics": { "type": "array" }, "diffs": { "type": "array" } }, "required": ["changed_files", "total_checked", "all_clean", "check_only", "diagnostics"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        fields: fields::<operations::FormatArgs>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Current entity ID" },
                    "new_name": { "type": "string", "description": "New entity ID" },
                    "dry_run": { "type": "boolean", "description": "Return the rename plan without changing any file", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                },
                "required": ["entity_id", "new_name"]
            })
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "old_name": { "type": "string" }, "new_name": { "type": "string" }, "affected_files": { "type": "array" }, "edits": { "type": "array" }, "dry_run": { "type": "boolean" }, "diagnostics": { "type": "array" } }, "required": ["old_name", "new_name", "affected_files", "edits"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        fields: fields::<operations::RenameArgs>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory for the new project, outside the current one" },
                    "name": { "type": "string", "description": "Project name (defaults to the directory name)" },
                    "version": { "type": "string", "description": "Project version", "default": "0.1.0" },
                    "extensions": { "type": "array", "items": { "type": "string" }, "description": "Builtin extensions to enable (e.g. @specforge/software) and local .wasm files to install" }
                },
                "required": ["path"]
            })
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "project_path": { "type": "string" }, "config_file": { "type": "string" }, "starter_file": { "type": "string" }, "extensions_installed": { "type": "array" }, "name": { "type": "string" }, "version": { "type": "string" } }, "required": ["project_path", "config_file", "starter_file", "extensions_installed", "name", "version"] }),
            )
        }),
        target: TargetSpec::new(Reach::NewProject, Freshness::Fresh),
        fields: fields::<operations::InitArgs>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "specifier": { "type": "string", "description": "Extension specifier" },
                    "dry_run": { "type": "boolean", "description": "Preview the install without changing any file", "default": false },
                    "allow_unsigned": { "type": "boolean", "description": "Accept a registry package with no publisher signature (publisher verification skipped)", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                },
                "required": ["specifier"]
            })
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "extension": { "type": "string" }, "installed": { "type": "boolean" }, "source": { "type": "string" }, "version": { "type": ["string", "null"] }, "changed": { "type": "boolean" }, "peers_enabled": { "type": "array" }, "note": { "type": "string" }, "sha256": { "type": "string" }, "key_id": { "type": ["string", "null"] }, "dry_run": { "type": "boolean" }, "already_present": { "type": "boolean" }, "message": { "type": "string" } }, "required": ["extension", "installed"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        fields: fields::<operations::AddArgs>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Extension name" },
                    "force": { "type": "boolean", "description": "Force removal", "default": false },
                    "dry_run": { "type": "boolean", "description": "Preview the removal, orphan warnings included, without changing any file", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                },
                "required": ["name"]
            })
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "removed_extension": { "type": "string" }, "success": { "type": "boolean" }, "version": { "type": ["string", "null"] }, "orphan_warnings": { "type": "array" }, "dry_run": { "type": "boolean" } }, "required": ["removed_extension", "success", "orphan_warnings"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        fields: fields::<operations::RemoveArgs>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "dry_run": { "type": "boolean", "description": "Return the diffs without changing any file", "default": false },
                    "target_version": { "type": "string", "description": "Format version to migrate to, as MAJOR.MINOR (defaults to the current format version)" },
                    "no_backup": { "type": "boolean", "description": "Skip the .bak backup of each migrated file", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                }
            })
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "from_version": { "type": "string" }, "to_version": { "type": "string" }, "migrated": { "type": "boolean" }, "dry_run": { "type": "boolean" }, "message": { "type": "string" }, "changes": { "type": "array" }, "files_migrated": { "type": "integer" }, "files_skipped": { "type": "integer" }, "files_failed": { "type": "integer" }, "results": { "type": "array" }, "diffs": { "type": "array" }, "rolled_back": { "type": "boolean" }, "post_migration_validated": { "type": "boolean" }, "post_migration_errors": { "type": "array" } }, "required": ["from_version", "to_version", "migrated", "dry_run"] }),
            )
        }),
        target: TargetSpec::new(Reach::WritesAnyProject, Freshness::Fresh),
        fields: fields::<operations::MigrateArgs>,
        handler: mutation!(operations::migrate_op, operations::MigrateArgs),
    },
    ToolSpec {
        name: "specforge.extensions",
        description: "List installed extensions",
        category: Category::Management,
        access: Access::ReadOnly,
        schema: || json!({ "type": "object", "properties": {} }),
        output: Some(
            || json!({ "type": "object", "properties": { "extensions": { "type": "array" }, "lock_file_entries": { "type": "array" }, "entity_kinds_in_graph": { "type": "array" } }, "required": ["extensions", "lock_file_entries", "entity_kinds_in_graph"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<NoArgs>,
        handler: typed!(operations::extensions_op, NoArgs),
    },
    ToolSpec {
        name: "specforge.providers",
        description: "List configured providers",
        category: Category::Management,
        access: Access::ReadOnly,
        schema: || json!({ "type": "object", "properties": {} }),
        output: Some(
            || json!({ "type": "object", "properties": { "providers": { "type": "array" }, "count": { "type": "integer" }, "diagnostics": { "type": "array" } }, "required": ["providers", "count", "diagnostics"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<NoArgs>,
        handler: typed!(operations::providers_op, NoArgs),
    },
    ToolSpec {
        name: "specforge.doctor",
        description: "Run health checks",
        category: Category::Management,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "use_cached": { "type": "boolean", "description": "Check the last compile instead of recompiling the project", "default": false }
                }
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "extensions_ok": { "type": "boolean" }, "conflicts": { "type": "array" }, "cache_status": { "type": "string" }, "findings": { "type": "array" }, "installed_count": { "type": "integer" }, "extensions": { "type": "array" }, "enhancements": { "type": "object" }, "shadowed": { "type": "array" }, "load_failures": { "type": "array" }, "issues": { "type": "array" }, "z3_available": { "type": "boolean" } }, "required": ["extensions_ok", "conflicts", "installed_count", "issues", "load_failures"] }),
        ),
        target: TargetSpec::new(Reach::Served, Freshness::FreshUnlessCached),
        fields: fields::<operations::DoctorArgs>,
        handler: typed!(operations::doctor_op, operations::DoctorArgs),
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "runner": { "type": "string", "description": "Collector name (e.g. cargo-test); detected from project files if omitted" },
                    "run": { "type": "boolean", "description": "Run the test command first; it must have been approved with `specforge collect` in a terminal (default false: parse the existing report)" },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                }
            })
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
        fields: fields::<operations::CollectArgs>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "format": required_choice_schema(&ops_export::FORMAT, "Renderer to use"),
                    "out_dir": { "type": "string", "description": "Directory to write the rendering into (returned inline when omitted)" },
                    "scope": { "type": "string", "description": "Scope to entity" }
                },
                "required": ["format"]
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "format": { "type": "string" }, "output": { "type": "string" }, "output_files": { "type": "array" } }, "required": ["format", "output_files"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<operations::RenderArgs>,
        handler: typed!(operations::render_op, operations::RenderArgs),
    },
    ToolSpec {
        name: "specforge.infer_progress",
        description: "Check inference progress: summary of analyzed vs unanalyzed source files, stale entries, and entity counts",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {}
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "summary": { "type": "object" }, "unanalyzed": { "type": "array" }, "stale": { "type": "array" }, "deleted": { "type": "array" }, "message": { "type": "string" } }, "required": ["summary", "unanalyzed", "stale", "deleted"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<NoArgs>,
        handler: typed!(infer_progress::call, NoArgs),
    },
    ToolSpec {
        name: "specforge.infer_gaps",
        description: "Analyze inference gaps: public Rust items not yet covered by spec entities (approximate)",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {}
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "total_pub_items": { "type": "integer" }, "covered_items": { "type": "integer" }, "gap_count": { "type": "integer" }, "approximate": { "type": "boolean" }, "scanners_used": { "type": "array" }, "scan_failures": { "type": "array" }, "by_directory": { "type": "array" }, "gaps": { "type": "array" }, "message": { "type": "string" } }, "required": ["total_pub_items", "covered_items", "approximate"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<NoArgs>,
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
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "mark_analyzed", "end"],
                        "description": "Session action to perform"
                    },
                    "agent": {
                        "type": "string",
                        "description": "Agent identifier (for start)"
                    },
                    "source_roots": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Source directories to scan (for start)"
                    },
                    "source_file": {
                        "type": "string",
                        "description": "Relative path to analyzed file (for mark_analyzed)"
                    },
                    "entities_produced": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Entity IDs produced from the file (for mark_analyzed)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to end (for end)"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["completed", "paused"],
                        "description": "Final status (for end, default: completed)"
                    }
                },
                "required": ["action"]
            })
        },
        output: Some(|| {
            with_files_written(
                json!({ "type": "object", "properties": { "session_id": { "type": "string" }, "status": { "type": "string" }, "source_file": { "type": "string" }, "entities_produced": { "type": "array" } }, "required": ["status"] }),
            )
        }),
        target: TargetSpec::SERVED,
        fields: fields::<infer_session::Args>,
        handler: mutation!(infer_session::call, infer_session::Args),
    },
    ToolSpec {
        name: "specforge.find_implementation",
        description: "Find source code locations that implement a specforge entity",
        category: Category::Navigation,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "entity_id": {
                        "type": "string",
                        "description": "Entity ID to find implementations for"
                    }
                },
                "required": ["entity_id"]
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "entity_id": { "type": "string" }, "implementations": { "type": "array" }, "count": { "type": "integer" } }, "required": ["entity_id", "implementations", "count"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<find_implementation::Args>,
        handler: typed!(find_implementation::call, find_implementation::Args),
    },
    ToolSpec {
        name: "specforge.find_spec_for_source",
        description: "Find specforge entities anchored to a source file",
        category: Category::Navigation,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path of the source file, relative to the project root (a directory lists the files under it; a trailing part of a path matches it)"
                    }
                },
                "required": ["file_path"]
            })
        },
        output: Some(
            || json!({ "type": "object", "properties": { "file_path": { "type": "string" }, "match_mode": { "type": "string", "enum": ["exact", "directory", "suffix_path", "none"] }, "entities": { "type": "array" }, "count": { "type": "integer" } }, "required": ["file_path", "match_mode", "entities", "count"] }),
        ),
        target: TargetSpec::SERVED,
        fields: fields::<find_spec_for_source::Args>,
        handler: typed!(find_spec_for_source::call, find_spec_for_source::Args),
    },
];

#[cfg(test)]
mod tests {
    use super::CORE_TOOLS;
    use crate::target::{Freshness, Reach};

    /// The arguments a tool's schema declares.
    fn declares(tool: &crate::tool::ToolSpec, argument: &str) -> bool {
        (tool.schema)()["properties"].get(argument).is_some()
    }

    #[specforge_test_macros::test(
        behavior = "list_mcp_tools",
        verify = "every tool that reads path or use_cached declares it in its target"
    )]
    fn every_tool_declares_how_it_reaches_its_project() {
        for tool in CORE_TOOLS {
            let reaches_by_path = matches!(
                tool.target.reach,
                Reach::AnyProject | Reach::WritesAnyProject | Reach::NewProject
            );
            assert_eq!(
                declares(tool, "path"),
                reaches_by_path,
                "{}: path in its schema, yet reach {:?}",
                tool.name,
                tool.target.reach
            );
            assert_eq!(
                declares(tool, "use_cached"),
                tool.target.freshness == Freshness::FreshUnlessCached,
                "{}: use_cached in its schema, yet freshness {:?}",
                tool.name,
                tool.target.freshness
            );
            // A tool that writes the files of the project its path names
            // is a mutation; one that reads no project reads no path.
            if tool.target.reach == Reach::WritesAnyProject {
                assert!(
                    matches!(tool.handler, crate::tool::Handler::Mutation(_)),
                    "{}",
                    tool.name
                );
            }
            if tool.target.reach == Reach::Unscoped {
                assert!(!declares(tool, "path"), "{}", tool.name);
            }
        }
    }
}
