#![allow(clippy::result_large_err)]

mod cache;
mod contributions;
mod discovery;
mod engine_pool;
mod host_functions;
mod install;
mod integrity;
mod lifecycle;
mod lock_file;
pub mod runtime;
mod query_extensions;
mod sandbox;
mod surface;
mod toposort;
mod manifest_bridge;
mod trap;
mod uninstall;
mod upgrade;

#[cfg(test)]
mod invariants;
#[cfg(test)]
pub(crate) mod test_helpers;

pub use cache::{
    aot_compile, cache_grammar_artifact, cache_path_for_hash, grammar_cache_key,
    has_cached_artifact, has_cached_grammar, invalidate_cache, invalidate_entry, AotCacheEntry,
    InvalidationReason,
};
pub use contributions::{
    auto_detect_collector, compose_grammar_injections, detect_grammar_contribution_conflicts,
    dispatch_collector, dispatch_contribution_exports, ingest_collector_report,
    is_contribution_disabled, register_collector_contributions, register_entity_enhancements,
    reject_reserved_entity_kind, required_contribution_exports, resolve_enhancement_conflicts,
    validate_collector_output, validate_contribution_exports, ContributionToggle, CoverageMetadata,
    EnhancementConflict, EnhancementOverride, EnhancementPolicy, GrammarConflictPolicy,
    IngestedReport, RegisteredCollector,
};
pub use discovery::{
    discover_extensions, parse_extension_specifier, ExtensionSource, ExtensionSpecifier,
    ResolvedExtension,
};
pub use engine_pool::{EnginePool, WarmEngineConfig, WarmInstance};
pub use host_functions::{
    compute_extension_query_scope, filter_graph_by_query_scope, host_add_graph_edge_check,
    host_add_graph_node_check, host_emit_diagnostic, host_emit_file_check, host_http_get_check,
    host_read_file_check, is_host_function_allowed, CallSite, QueryScope,
};
pub use integrity::{hex_sha256, verify_wasm_integrity, verify_wasm_integrity_or_skip};
pub use lifecycle::{
    call_extension_validators, dispatch_body_parser, initialize_extension, load_extension_grammar,
    load_wasm_module, validate_extension_peer_dependencies, validate_grammar_wasm,
    GrammarLoadResult,
};
pub use lock_file::{
    read_lock_file, refresh_lock_file, run_doctor_check, write_lock_file, DoctorStatus, LockFile,
    LockFileEntry,
};
pub use runtime::{
    ExtensionLifecycleState, LoadedModule, WasmCallResult, WasmRuntime, WasmTrapInfo,
};
pub use sandbox::{
    configure_sandbox_policy, default_sandbox_policy, is_domain_allowed, is_output_extension_allowed,
    is_path_allowed, validate_total_memory,
};
pub use toposort::topological_sort_extensions;
pub use manifest_bridge::{
    detect_entity_kind_collision, load_extension_manifest_from_path, validate_extension_manifest,
};
pub use query_extensions::{
    compose_query_files, validate_query_extensions, QueryExtension, QueryFileKind,
    RawQueryExtension,
};
pub use surface::{
    auto_promote_commands_to_mcp_tools, dispatch_surface_command, dispatch_surface_mcp_resource,
    dispatch_surface_mcp_tool, enforce_resource_sandbox, enforce_surface_sandbox,
    toggle_surface_contribution, validate_command_arg_types, validate_mcp_tool_schemas,
    validate_surface_exports, AutoPromotedMcpTool, CommandOutput, EffectiveSandbox, SurfaceEntry,
    SurfaceEntryType, SurfaceSandboxOverrideValues,
};
pub use trap::{handle_wasm_trap, should_skip_extension};
pub use install::{install_extension, install_from_local, installed_wasm_path, InstallResult};
pub use uninstall::{check_dependents, uninstall_extension, UninstallResult};
pub use upgrade::{check_newer_version, upgrade_extension, UpgradeResult};
