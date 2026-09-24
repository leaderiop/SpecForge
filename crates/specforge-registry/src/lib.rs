// Module groups
pub mod client;
pub mod compilation;
mod manifest;
mod registries;
pub mod signing;

#[cfg(test)]
mod invariants;

// --- Core registries ---
pub use registries::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistry, FieldRegistryEntry, KindRegistry,
    KindRegistryEntry, ManifestFieldType,
};

// --- Manifest types ---
pub use manifest::surface::{
    CommandArg, CommandArgType, CommandContribution, McpResourceContribution, McpToolContribution,
    SurfaceContributions, SurfaceRegistryEntry, SurfaceSandboxOverride, SurfaceType,
    register_surface_contributions,
};
pub use manifest::types::{
    AnalyzerContribution, BodyParserContribution, CollectorAutoDetect, CollectorContribution,
    ExtensionContributions, FieldConstraint, FieldEnhancement, GrammarContribution,
    ManifestEdgeType, ManifestEntityKind, ManifestField, ManifestV2, ManifestValidationRule,
    PeerDependency, SandboxPolicy, validate_manifest, validate_manifest_consistency,
};

// --- Compilation / extension logic ---
pub use compilation::{
    EntityRefInfo,
    GrammarConflictPolicy,
    HOST_API_VERSION,
    ProviderConfig,
    ProviderSchemeRegistry,
    RegisteredBodyParser,
    RegisteredGrammar,
    SchemeRegistryEntry,
    // populate
    apply_entity_enhancements,
    detect_circular_peer_dependencies,
    // validate
    detect_duplicate_entity_kinds,
    // detection
    detect_mistyped_references,
    detect_unknown_entity_fields,
    detect_unknown_entity_kinds,
    // populate/validate (above)
    generate_keyword_extension_index,
    generate_required_field_rules,
    // provider
    load_extension_manifests,
    load_provider_configurations,
    populate_registries,
    // contributions
    register_body_parser_contributions,
    register_extension_entity_types,
    register_grammar_contributions,
    register_provider_schemes,
    register_validation_rules,
    register_verify_kinds,
    validate_extension_testability,
    validate_host_api_versions,
    validate_peer_dependencies,
    validate_provider_kinds,
    validate_provider_ref,
    validate_ref_target_format,
    validate_registered_entity_fields,
};
pub use signing::{
    PackageSignature, SigningKey, load_or_create_signing_key, signing_key_path, verify_signature,
};

// --- Registry client ---
pub use client::trust::{KnownKeys, known_keys_path};
pub use client::{
    AuthMethod, CredentialStore, HttpRegistryClient, RegistryClient, RegistryConfig,
    RegistryCredential, RegistryError, RegistryResponse, RegistrySearchResult, RetryPolicy,
    TrustCheck, authenticate_with_retry, find_registry_for_specifier, load_known_keys,
    logout_registry, parse_registries_from_config, publish_to_registry, resolve_credential,
    resolve_from_registry, resolve_version, sanitize_token, save_known_keys, search_registries,
    validate_credentials, verify_package_signature, verify_registry_integrity,
};

// Backward-compatible module path aliases for external code that uses
// `specforge_registry::validation_engine::` or `specforge_registry::registry_config::` etc.
pub use client::auth;
pub use client::credentials;
pub use client::http_client;
pub use client::registry_client;
pub use client::registry_config;
pub use client::registry_ops;
pub use client::resolver;
pub use compilation::contributions;
pub use compilation::define;
pub use compilation::detection as compilation_detection;
pub use compilation::keyword_index;
pub use compilation::provider;
pub use compilation::validation_engine;
pub use manifest::surface;
