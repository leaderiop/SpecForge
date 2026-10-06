// Module groups
pub mod compilation;
pub mod entity;
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
// --- What a registry entry embeds: the descriptor its extension declared ---
pub use specforge_protocol_types::{EdgeTypeDescriptor, EntityKindDescriptor, FieldDescriptor};

// --- What an extension declares, as the registry build reads it (ADR 0012) ---
pub use specforge_protocol_types::PeerDependency;

// --- The surfaces registry ---
pub use surface::{
    CommandArgType, SurfaceRegistryEntry, SurfaceType, refuse_malformed_tool_schemas,
    register_surface_contributions,
};

// --- Registry compilation (plan 05): one build, and the graph checks ---
pub use compilation::{
    CHECK_PHASE, DeclaredPass, EntityView, ProviderConfig, ProviderSchemeRegistry, ProviderStatus,
    RegistryBuild, SchemeRegistryEntry, build_registries, detect_identifier_length_violations,
    detect_mistyped_references, detect_reserved_entity_ids, detect_unknown_entity_fields,
    detect_unknown_entity_kinds, load_provider_configurations, register_provider_schemes,
    register_provider_schemes_with_status,
};

// Module paths external code names directly
// (`specforge_registry::validation_engine::`, `specforge_registry::surface::`).
pub use compilation::validation_engine;
pub mod surface;
