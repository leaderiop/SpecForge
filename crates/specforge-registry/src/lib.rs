// Module groups
pub mod compilation;
pub mod entity;
mod registries;
pub mod rules;

#[cfg(test)]
mod invariants;

// --- Core registries ---
pub use registries::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistry, FieldRegistryEntry, KindRegistry,
    KindRegistryEntry, ManifestFieldType,
};

// --- Extension vocabulary (shared with the SDK through the protocol types) ---
pub use specforge_protocol_types::{CheckKind, ConstraintKind, FieldType, ProofRole};
// --- What a registry entry embeds: the descriptor its extension declared ---
pub use specforge_protocol_types::{EdgeTypeDescriptor, EntityKindDescriptor, FieldDescriptor};

// --- What an extension declares, as the registry build reads it (ADR 0012) ---
pub use specforge_protocol_types::PeerDependency;

// --- The surfaces registry ---
pub use surface::{
    CommandArgType, SurfaceRegistryEntry, SurfaceType, refuse_malformed_tool_schemas,
    register_surface_contributions,
};

// --- What the checks after the graph build read about an entity (ADR 0019) ---
pub use entity::{
    Direction, EdgeCounts, EdgeRecord, EntityRecord, Exemption, FieldRecord, MethodRecord,
    ObligationRecord, ParamRecord, RuleInput,
};

// --- Registry compilation (plan 05): one build, and the graph checks ---
pub use compilation::{
    CHECK_PHASE, DeclaredPass, ProviderConfig, ProviderSchemeRegistry, ProviderStatus,
    RegistryBuild, SchemeRegistryEntry, build_registries, detect_identifier_length_violations,
    detect_mistyped_references, detect_reserved_entity_ids, detect_unknown_entity_fields,
    detect_unknown_entity_kinds, load_provider_configurations, register_provider_schemes,
    register_provider_schemes_with_status,
};

// Module paths external code names directly (`specforge_registry::surface::`).
pub mod surface;
