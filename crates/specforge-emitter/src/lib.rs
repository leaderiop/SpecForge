mod brief;
mod budget;
pub mod builtins;
pub mod compile;
mod context;
pub mod scanner_dispatch;
mod diagnostic_fmt;
mod dot;
mod emit;
mod error;
mod exit_code;
mod json;
pub mod model;
pub mod outline;
mod plan;
mod query;
pub mod schema;
mod scope;
mod stats;
mod trace;

// --- Primary API (use these) ---
pub use error::EmitterError;
pub use emit::{emit, EmitFormat, EmitOptions};
pub use compile::{build_validation_entities, compile, compile_simple, compile_with_runtime, CompilationContext};
pub use exit_code::{compute_exit_code, compute_exit_code_strict};
pub use stats::{compute_stats, compute_stats_with_diagnostics, compute_stats_with_testable, ProjectStats};
pub use query::query;
pub use diagnostic_fmt::{format_diagnostic, serialize_diagnostics, truncate_diagnostics, diagnostic_summary, MAX_DIAGNOSTICS};
pub use plan::{serialize_plan_result, validate_plan, PlanValidationResult};
pub use trace::{detect_trace_gaps, serialize_trace, serialize_trace_all, trace, trace_all, TraceChain, TraceLink};
pub use schema::{
    compute_schema_version, detect_breaking_with_diagnostics, diff_schemas,
    diff_schemas_optional, emit_schema, emit_schema_for_kind, generate_schema,
    negotiate_version, negotiate_version_or_latest,
    persist_schema_cache, load_schema_cache, publish_json_schema,
    GraphProtocolSchema, SchemaCompatibility, SchemaEdgeType, SchemaEntityKind,
    SchemaExtensionInfo, SchemaField, SchemaCacheEntry, SchemaMigration,
    SchemaMigrationChange, SchemaVersion, SchemaVersionError,
};

// --- Legacy API (kept for backwards compatibility with tests) ---
// Prefer `emit(graph, &EmitOptions { format, scope, schema, .. })` instead.
pub use brief::emit_brief;
pub use context::emit_context;
pub use dot::emit_dot;
pub use json::{emit_json, emit_json as emit_graph};
pub use scope::{emit_context_scoped, emit_json_scoped};
pub use budget::{emit_json_with_budget, emit_json_with_budget_strategy};
pub use schema::{
    emit_brief_scoped_with_schema, emit_brief_with_schema,
    emit_context_scoped_with_schema, emit_context_with_schema,
    emit_json_scoped_with_schema, emit_json_with_schema,
};
