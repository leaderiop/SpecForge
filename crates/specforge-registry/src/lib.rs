// Module groups
pub mod client;
pub mod compilation;
mod manifest;
mod registries;
pub mod signing;

#[cfg(test)]
mod invariants;

// Tests inside the crate name it as its users do.
#[cfg(test)]
extern crate self as specforge_registry;

// --- Core registries ---
pub use registries::{
    EdgeRegistry, EdgeRegistryEntry, FieldRegistry, FieldRegistryEntry, KindRegistry,
    KindRegistryEntry, ManifestFieldType,
};

// --- Extension vocabulary (shared with the SDK through the protocol types) ---
pub use specforge_protocol_types::{CheckKind, FieldType};

// --- Manifest types ---
pub use manifest::surface::{
    CommandArg, CommandArgType, CommandContribution, McpResourceContribution, McpToolContribution,
    SurfaceContributions, SurfaceRegistryEntry, SurfaceSandboxOverride, SurfaceType,
    register_surface_contributions,
};
pub use manifest::types::{
    AnalyzerContribution, CollectorAutoDetect, CollectorContribution, ExtensionContributions,
    FieldConstraint, FieldEnhancement, ManifestEdgeType, ManifestEntityKind, ManifestField,
    ManifestV2, ManifestValidationRule, PeerDependency, SandboxPolicy, unknown_manifest_fields,
    validate_manifest, validate_manifest_consistency, validate_manifest_consistency_with_peers,
};

// --- Registry compilation (plan 05): one build, and the graph checks ---
pub use compilation::{
    EntityView, ProviderConfig, ProviderSchemeRegistry, ProviderStatus, RegistryBuild,
    SchemeRegistryEntry, build_registries, detect_identifier_length_violations,
    detect_mistyped_references, detect_reserved_entity_ids, detect_unknown_entity_fields,
    detect_unknown_entity_kinds, load_provider_configurations, populate_registries,
    register_provider_schemes, register_provider_schemes_with_status, validate_peer_dependencies,
    validate_peer_dependencies_of,
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
    resolve_diamond, resolve_from_registry, resolve_version, sanitize_token, save_known_keys,
    search_registries, validate_credentials, verify_package_signature, verify_registry_integrity,
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
pub use compilation::validation_engine;
pub use manifest::surface;
