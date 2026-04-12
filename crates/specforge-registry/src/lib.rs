// Module groups
mod registries;
mod manifest;
pub mod compilation;
pub mod client;

#[cfg(test)]
mod invariants;

// --- Core registries ---
pub use registries::{
    EdgeRegistry, EdgeRegistryEntry,
    FieldRegistry, FieldRegistryEntry, ManifestFieldType,
    KindRegistry, KindRegistryEntry,
};

// --- Manifest types ---
pub use manifest::types::{
    validate_manifest, validate_manifest_consistency, BodyParserContribution,
    CollectorAutoDetect, CollectorContribution, ExtensionContributions, FieldConstraint,
    FieldEnhancement, GrammarContribution, ManifestEdgeType, ManifestEntityKind, ManifestField,
    ManifestV2, ManifestValidationRule, PeerDependency, SandboxPolicy,
};
pub use manifest::surface::{
    register_surface_contributions, CommandArg, CommandArgType, CommandContribution,
    McpResourceContribution, McpToolContribution, SurfaceContributions, SurfaceRegistryEntry,
    SurfaceSandboxOverride, SurfaceType,
};

// --- Compilation / extension logic ---
pub use compilation::{
    // contributions
    register_body_parser_contributions, register_grammar_contributions, GrammarConflictPolicy,
    RegisteredBodyParser, RegisteredGrammar,
    // detection
    detect_mistyped_references, detect_unknown_entity_kinds, detect_unknown_entity_fields,
    EntityRefInfo,
    // populate
    apply_entity_enhancements, populate_registries,
    // validate
    detect_duplicate_entity_kinds, register_validation_rules, register_verify_kinds,
    detect_circular_peer_dependencies, validate_extension_testability,
    validate_host_api_versions, validate_peer_dependencies,
    validate_registered_entity_fields, HOST_API_VERSION,
    // populate/validate (above)
    generate_keyword_extension_index,
    // provider
    load_extension_manifests, load_provider_configurations, register_extension_entity_types,
    register_provider_schemes, validate_provider_ref, validate_ref_target_format,
    validate_provider_kinds, ProviderConfig, ProviderSchemeRegistry, SchemeRegistryEntry,
};

// --- Registry client ---
pub use client::{
    authenticate_with_retry, logout_registry, resolve_credential, sanitize_token,
    validate_credentials,
    RegistryClient, RegistryError, RegistryResponse, RegistrySearchResult, RetryPolicy,
    find_registry_for_specifier, parse_registries_from_config, AuthMethod, RegistryConfig,
    RegistryCredential, TrustLevel,
    assign_trust_level, publish_to_registry, resolve_from_registry, search_registries,
    verify_registry_integrity,
};

// Backward-compatible module path aliases for external code that uses
// `specforge_registry::validation_engine::` or `specforge_registry::registry_config::` etc.
pub use compilation::validation_engine;
pub use compilation::detection as compilation_detection;
pub use compilation::keyword_index;
pub use compilation::provider;
pub use compilation::define;
pub use compilation::contributions;
pub use client::auth;
pub use client::registry_client;
pub use client::registry_config;
pub use client::registry_ops;
pub use manifest::surface;
