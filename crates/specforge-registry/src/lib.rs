// Module groups
pub mod compilation;
mod manifest;
mod registries;

#[cfg(test)]
mod invariants;

// Tests inside the crate name it as its users do.
#[cfg(test)]
extern crate self as specforge_registry;

// --- Core registries ---
pub use registries::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistry, FieldRegistryEntry, KindRegistry,
    KindRegistryEntry, ManifestFieldType, ProofRole,
};

// --- Extension vocabulary (shared with the SDK through the protocol types) ---
pub use specforge_protocol_types::{CheckKind, ConstraintKind, FieldType};

// --- Manifest types ---
pub use manifest::surface::{
    CommandArg, CommandArgType, CommandContribution, McpResourceContribution, McpToolContribution,
    SurfaceContributions, SurfaceRegistryEntry, SurfaceSandboxOverride, SurfaceType,
    refuse_malformed_tool_schemas, register_surface_contributions,
};
pub use manifest::types::{
    AnalyzerContribution, CollectorAutoDetect, CollectorContribution, ExtensionContributions,
    FieldConstraint, FieldEnhancement, ManifestEdgeType, ManifestEntityKind, ManifestField,
    ManifestV2, ManifestValidationRule, PeerDependency, SandboxPolicy, unknown_manifest_fields,
    validate_manifest, validate_manifest_consistency, validate_manifest_consistency_with_peers,
};

// --- Registry compilation (plan 05): one build, and the graph checks ---
pub use compilation::{
    CHECK_PHASE, DeclaredPass, EntityView, ProviderConfig, ProviderSchemeRegistry, ProviderStatus,
    RegistryBuild, SchemeRegistryEntry, build_registries, detect_identifier_length_violations,
    detect_mistyped_references, detect_reserved_entity_ids, detect_unknown_entity_fields,
    detect_unknown_entity_kinds, load_provider_configurations, populate_registries,
    register_provider_schemes, register_provider_schemes_with_status, validate_peer_dependencies,
    validate_peer_dependencies_of,
};

// Module paths external code names directly
// (`specforge_registry::validation_engine::`, `specforge_registry::surface::`).
pub use compilation::validation_engine;
pub use manifest::surface;
