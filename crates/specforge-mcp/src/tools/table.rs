//! The core tools: one entry per tool holds its name, description,
//! category, input schema, mutation effect and handler. The listing,
//! dispatch and events all derive from it.

use serde_json::{Value, json};

use super::*;
use crate::args::{NoArgs, fields};
use crate::operations;
use crate::tool::{Access, Category, Effect, MutationSpec, ToolSpec, writes_unless_dry_run};

/// A handler reading its typed arguments: refused when they don't parse.
macro_rules! typed {
    ($handler:path, $args:ty) => {
        |state, arguments| match crate::args::parse::<$args>(arguments) {
            Ok(args) => $handler(state, args),
            Err(refused) => refused,
        }
    };
}

fn effect(files_changed: usize, entities_affected: usize) -> Effect {
    Effect {
        files_changed,
        entities_affected,
    }
}

/// The length of the array at `key` in a tool's payload.
fn count(payload: &Value, key: &str) -> usize {
    payload[key].as_array().map_or(0, Vec::len)
}

/// Whether a format call writes: check and diff modes only report, unless
/// `write` says otherwise.
fn format_writes(args: &Value) -> bool {
    let flag = |key: &str| args.get(key).and_then(Value::as_bool);
    flag("write").unwrap_or(!flag("check").unwrap_or(false) && !flag("diff").unwrap_or(false))
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
                    "format": { "type": "string", "description": "Output detail level (default \"graph\")", "default": "graph" },
                    "include_coverage": { "type": "boolean", "description": "Include coverage metadata in the response", "default": false }
                },
                "required": ["entity_id"]
            })
        },
        mutation: None,
        fields: fields::<query::Args>,
        call: typed!(query::call, query::Args),
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
                    "severity_filter": { "type": "string", "description": "Only report diagnostics of this severity (error, warning, info)" },
                    "strict": { "type": "boolean", "description": "Promote warnings to errors, before severity_filter applies", "default": false },
                    "lint": { "type": "array", "items": { "type": "string" }, "description": "Extra lint profiles, as `specforge check --lint` takes (inferred: I200/I202 from specforge-infer.json)" },
                    "use_cached": { "type": "boolean", "description": "Report cached diagnostics from the last compile instead of recompiling", "default": false }
                }
            })
        },
        mutation: None,
        fields: fields::<validate::Args>,
        call: typed!(validate::call, validate::Args),
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
                    "pass": { "type": "string", "description": "Analysis pass to run (all, coverage, contracts)" },
                    "strict": { "type": "boolean", "description": "Promote warnings to errors" },
                    "test_results": { "type": "string", "description": "Path to a specforge-report.json for proof-level verdicts" },
                    "use_cached": { "type": "boolean", "description": "Analyze the last compiled graph instead of recompiling (a server with no graph compiles anyway)", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                }
            })
        },
        mutation: None,
        fields: fields::<analyze::Args>,
        call: typed!(analyze::call, analyze::Args),
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
                    "format": { "type": "string", "enum": ["graph", "context", "brief"], "default": "graph" },
                    "scope": { "type": "string", "description": "Scope to entity subgraph" },
                    "max_tokens": { "type": "integer", "description": "Optional token budget; truncates the export to the most central entities that fit" },
                    "with_schema": { "type": "boolean", "description": "Embed the Graph Protocol schema in a context, brief or budgeted graph export (a full graph export embeds it already); under max_tokens it counts toward the budget" },
                    "no_schema": { "type": "boolean", "description": "Leave the schema out of a graph export (Graph Protocol 1.0)" }
                }
            })
        },
        mutation: None,
        fields: fields::<export::Args>,
        call: typed!(export::call, export::Args),
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
        mutation: None,
        fields: fields::<trace::Args>,
        call: typed!(trace::call, trace::Args),
    },
    ToolSpec {
        name: "specforge.search",
        description: "Fuzzy search over graph nodes",
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
                    "references": { "type": "string", "description": "Find entities with edges to this target" }
                },
                "required": ["query"]
            })
        },
        mutation: None,
        fields: fields::<search::Args>,
        call: typed!(search::call, search::Args),
    },
    ToolSpec {
        name: "specforge.schema",
        description: "Get the graph schema definition",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Filter schema to a specific entity kind" },
                    "include_edges": { "type": "boolean", "description": "Include edge labels", "default": true },
                    "include_validation_rules": { "type": "boolean", "description": "Include the validation rules loaded extensions declare", "default": false }
                }
            })
        },
        mutation: None,
        fields: fields::<schema::Args>,
        call: typed!(schema::call, schema::Args),
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
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "mermaid", "dot", "json", "dbml"],
                        "description": "Output format (default: markdown)"
                    },
                    "group_by": {
                        "type": "string",
                        "enum": ["extension", "none"],
                        "description": "Group entities by extension or list flat (default: extension)"
                    },
                    "fields": {
                        "type": "string",
                        "enum": ["none", "keys", "all"],
                        "description": "Field detail level (default: keys)"
                    },
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
        mutation: None,
        fields: fields::<model::Args>,
        call: typed!(model::call, model::Args),
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
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "mermaid", "dot", "json"],
                        "description": "Output format (default: json). JSON recommended for programmatic consumption."
                    },
                    "fields": {
                        "type": "string",
                        "enum": ["none", "keys", "all"],
                        "description": "Detail level: none (counts only), keys (names + rule codes), all (full field attribution). Default: keys"
                    },
                    "deps": {
                        "type": "string",
                        "enum": ["direct", "effective", "full"],
                        "description": "Dependency visibility: direct (declared only), effective (direct + used transitive), full (all transitive). Default: direct"
                    }
                }
            })
        },
        mutation: None,
        fields: fields::<outline_extensions::Args>,
        call: typed!(outline_extensions::call, outline_extensions::Args),
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
                    "status_filter": { "type": "string", "enum": ["covered", "partial", "uncovered"], "description": "Only entities with this coverage status" }
                }
            })
        },
        mutation: None,
        fields: fields::<coverage::Args>,
        call: typed!(coverage::call, coverage::Args),
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
        mutation: None,
        fields: fields::<NoArgs>,
        call: typed!(stats::call, NoArgs),
    },
    ToolSpec {
        name: "specforge.list",
        description: "List entities, optionally filtered by kind",
        category: Category::Core,
        access: Access::ReadOnly,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Filter by entity kind (e.g. 'feature', 'behavior')" }
                }
            })
        },
        mutation: None,
        fields: fields::<list::Args>,
        call: typed!(list::call, list::Args),
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
        mutation: None,
        fields: fields::<inspect::Args>,
        call: typed!(inspect::call, inspect::Args),
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
        mutation: None,
        fields: fields::<find_definition::Args>,
        call: typed!(find_definition::call, find_definition::Args),
    },
    ToolSpec {
        name: "specforge.find_references",
        description: "Find all references to an entity",
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
        mutation: None,
        fields: fields::<find_references::Args>,
        call: typed!(find_references::call, find_references::Args),
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
        mutation: None,
        fields: fields::<outline::Args>,
        call: typed!(outline::call, outline::Args),
    },
    ToolSpec {
        name: "specforge.suggest_fixes",
        description: "Get suggested fixes for diagnostics",
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
        mutation: None,
        fields: fields::<suggest_fixes::Args>,
        call: typed!(suggest_fixes::call, suggest_fixes::Args),
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
        mutation: Some(MutationSpec {
            writes: format_writes,
            effect: |o| effect(count(o, "changed_files"), 0),
            recompiles: true,
        }),
        fields: fields::<operations::FormatArgs>,
        call: typed!(operations::format_op, operations::FormatArgs),
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
        mutation: Some(MutationSpec {
            writes: writes_unless_dry_run,
            effect: |o| effect(count(o, "affected_files"), 1),
            recompiles: true,
        }),
        fields: fields::<operations::RenameArgs>,
        call: typed!(operations::rename_op, operations::RenameArgs),
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
        mutation: Some(MutationSpec {
            writes: writes_unless_dry_run,
            effect: |_| effect(3, 0),
            recompiles: true,
        }),
        fields: fields::<operations::InitArgs>,
        call: typed!(operations::init_op, operations::InitArgs),
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
        mutation: Some(MutationSpec {
            writes: writes_unless_dry_run,
            effect: |o| {
                if o["installed"] == true {
                    effect(3, 0)
                } else {
                    Effect::default()
                }
            },
            recompiles: true,
        }),
        fields: fields::<operations::AddArgs>,
        call: typed!(operations::add_extension, operations::AddArgs),
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
        mutation: Some(MutationSpec {
            writes: writes_unless_dry_run,
            effect: |o| {
                if o["success"] == true {
                    effect(3, count(o, "orphan_warnings"))
                } else {
                    Effect::default()
                }
            },
            recompiles: true,
        }),
        fields: fields::<operations::RemoveArgs>,
        call: typed!(operations::remove_extension_op, operations::RemoveArgs),
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
        mutation: Some(MutationSpec {
            writes: writes_unless_dry_run,
            effect: |o| {
                if o["migrated"] == true {
                    effect(o["files_migrated"].as_u64().unwrap_or(0) as usize, 0)
                } else {
                    Effect::default()
                }
            },
            recompiles: true,
        }),
        fields: fields::<operations::MigrateArgs>,
        call: typed!(operations::migrate_op, operations::MigrateArgs),
    },
    ToolSpec {
        name: "specforge.extensions",
        description: "List installed extensions",
        category: Category::Management,
        access: Access::ReadOnly,
        schema: || json!({ "type": "object", "properties": {} }),
        mutation: None,
        fields: fields::<NoArgs>,
        call: typed!(operations::extensions_op, NoArgs),
    },
    ToolSpec {
        name: "specforge.providers",
        description: "List configured providers",
        category: Category::Management,
        access: Access::ReadOnly,
        schema: || json!({ "type": "object", "properties": {} }),
        mutation: None,
        fields: fields::<NoArgs>,
        call: typed!(operations::providers_op, NoArgs),
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
        mutation: None,
        fields: fields::<operations::DoctorArgs>,
        call: typed!(operations::doctor_op, operations::DoctorArgs),
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
        mutation: None,
        fields: fields::<operations::CollectArgs>,
        call: typed!(operations::collect_op, operations::CollectArgs),
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
                    "format": { "type": "string", "enum": ["json", "dot", "context", "brief"], "description": "Renderer to use" },
                    "out_dir": { "type": "string", "description": "Directory to write the rendering into (returned inline when omitted)" },
                    "scope": { "type": "string", "description": "Scope to entity" }
                },
                "required": ["format"]
            })
        },
        mutation: None,
        fields: fields::<operations::RenderArgs>,
        call: typed!(operations::render_op, operations::RenderArgs),
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
        mutation: None,
        fields: fields::<NoArgs>,
        call: typed!(infer_progress::call, NoArgs),
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
        mutation: None,
        fields: fields::<NoArgs>,
        call: typed!(infer_gaps::call, NoArgs),
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
        mutation: Some(MutationSpec {
            writes: writes_unless_dry_run,
            effect: |o| effect(1, count(o, "entities_produced")),
            recompiles: false,
        }),
        fields: fields::<infer_session::Args>,
        call: typed!(infer_session::call, infer_session::Args),
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
        mutation: None,
        fields: fields::<find_implementation::Args>,
        call: typed!(find_implementation::call, find_implementation::Args),
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
                        "description": "Relative path to source file"
                    }
                },
                "required": ["file_path"]
            })
        },
        mutation: None,
        fields: fields::<find_spec_for_source::Args>,
        call: typed!(find_spec_for_source::call, find_spec_for_source::Args),
    },
];
