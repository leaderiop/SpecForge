pub mod auth;
pub mod registry_client;
pub mod registry_config;
pub mod registry_ops;

pub use auth::{
    authenticate_with_retry, logout_registry, resolve_credential, sanitize_token,
    validate_credentials,
};
pub use registry_client::{
    RegistryClient, RegistryError, RegistryResponse, RegistrySearchResult, RetryPolicy,
};
pub use registry_config::{
    find_registry_for_specifier, parse_registries_from_config, AuthMethod, RegistryConfig,
    RegistryCredential, TrustLevel,
};
pub use registry_ops::{
    assign_trust_level, publish_to_registry, resolve_from_registry, search_registries,
    verify_registry_integrity,
};
