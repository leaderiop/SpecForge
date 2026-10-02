#![allow(clippy::result_large_err)]

mod contributions;
mod discovery;
mod install;
mod integrity;
mod lifecycle;
mod lock_file;
mod manifest_bridge;
pub mod protocol;
mod query_extensions;
pub mod runtime;
pub mod sandbox;
mod surface;
mod toposort;
mod trap;
mod uninstall;
mod upgrade;

#[cfg(test)]
mod invariants;
#[cfg(test)]
pub(crate) mod test_helpers;

pub use contributions::{
    ContributionToggle, EnhancementConflict, EnhancementOverride, EnhancementPolicy,
    is_contribution_disabled, register_entity_enhancements, reject_reserved_entity_kind,
    required_contribution_exports, resolve_enhancement_conflicts, validate_contribution_exports,
};
pub use discovery::{
    ExtensionSource, ExtensionSpecifier, ResolvedExtension, discover_extensions,
    parse_extension_specifier,
};
pub use install::{InstallResult, install_extension, install_from_local, installed_wasm_path};
pub use integrity::{hex_sha256, verify_wasm_integrity, verify_wasm_integrity_or_skip};
pub use lifecycle::{
    call_extension_validators, initialize_extension, load_wasm_module,
    validate_extension_peer_dependencies,
};
pub use lock_file::{
    DoctorStatus, LockFile, LockFileEntry, collect_peer_requirers, read_lock_file,
    refresh_lock_file, run_doctor_check, write_lock_file,
};
pub use manifest_bridge::{
    detect_entity_kind_collision, load_extension_manifest_from_path, validate_extension_manifest,
};
pub use query_extensions::{
    QueryExtension, QueryFileKind, RawQueryExtension, compose_query_files,
    validate_query_extensions,
};
pub use runtime::{
    ExtensionLifecycleState, LoadedModule, WasmCallResult, WasmRuntime, WasmTrapInfo,
};
pub use sandbox::{
    configure_sandbox_policy, default_sandbox_policy, is_domain_allowed,
    is_output_extension_allowed, is_path_allowed, validate_total_memory,
};
pub use surface::{
    AutoPromotedMcpTool, CommandOutput, EffectiveSandbox, SurfaceEntry, SurfaceEntryType,
    SurfaceSandboxOverrideValues, auto_promote_commands_to_mcp_tools, dispatch_surface_command,
    dispatch_surface_mcp_resource, dispatch_surface_mcp_tool, enforce_resource_sandbox,
    enforce_surface_sandbox, toggle_surface_contribution, validate_command_arg_types,
    validate_mcp_tool_schemas, validate_surface_exports,
};
pub use toposort::topological_sort_extensions;
pub use trap::{handle_wasm_trap, should_skip_extension};
pub use uninstall::{UninstallResult, check_dependents, uninstall_extension};
pub use upgrade::{UpgradeResult, check_newer_version, upgrade_extension};
