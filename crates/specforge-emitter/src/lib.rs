pub mod analyze;
mod brief;
mod budget;
pub mod collect;
pub use budget::filter_graph_within_budget;
pub mod compile;
mod context;
mod convention;
mod diagnostic_fmt;
pub mod doctor;
mod dot;
mod emit;
mod error;
mod exit_code;
mod json;
pub mod model;
pub mod outline;
mod plan;
mod query;
pub mod scanner_dispatch;
pub mod schema;
mod scope;
mod stats;
mod trace;

// --- Primary API (use these) ---
pub use compile::{
    CompilationContext, WasmCustomRules, build_validation_entities, compile_simple,
    compile_with_runtime,
};
pub use diagnostic_fmt::{
    MAX_DIAGNOSTICS, diagnostic_summary, format_diagnostic, serialize_diagnostics,
    truncate_diagnostics,
};
pub use dot::DotOptions;
pub use emit::{EmitFormat, EmitOptions, emit};
pub use error::EmitterError;
pub use exit_code::{compute_exit_code, compute_exit_code_strict};
pub use json::{SCHEMA_VERSION, field_map_to_json, field_value_to_json};
pub use plan::{PlanGap, PlanGapKind, PlanValidationResult, serialize_plan_result, validate_plan};
pub use query::query;
pub use schema::{
    GraphProtocolSchema, SchemaCacheEntry, SchemaCompatibility, SchemaEdgeType, SchemaEntityKind,
    SchemaExtensionInfo, SchemaField, SchemaMigration, SchemaMigrationChange, SchemaVersion,
    SchemaVersionError, compute_schema_version, detect_breaking_with_diagnostics, diff_schemas,
    diff_schemas_optional, emit_schema, emit_schema_for_kind, generate_schema, load_schema_cache,
    negotiate_version, persist_schema_cache, publish_json_schema_format,
};
pub use stats::{
    ProjectStats, compute_stats, compute_stats_with_diagnostics, compute_stats_with_testable,
};
pub use trace::{
    TraceChain, TraceLink, detect_trace_gaps, serialize_trace, serialize_trace_all, trace,
    trace_all,
};

// --- Legacy API (kept for backwards compatibility with tests) ---
// Prefer `emit(graph, &EmitOptions { format, scope, schema, .. })` instead.
pub use brief::emit_brief;
pub use budget::{emit_json_with_budget, emit_json_with_budget_strategy};
pub use context::emit_context;
pub use dot::emit_dot;
pub use json::{emit_json, emit_json as emit_graph};
pub use schema::{
    emit_brief_scoped_with_schema, emit_brief_with_schema, emit_context_scoped_with_schema,
    emit_context_with_schema, emit_json_scoped_with_schema, emit_json_with_schema,
};
pub use scope::{emit_context_scoped, emit_json_scoped};
