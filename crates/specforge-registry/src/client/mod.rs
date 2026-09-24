pub mod auth;
pub mod credentials;
pub mod http_client;
pub mod registry_client;
pub mod registry_config;
pub mod registry_ops;
pub mod resolver;
pub mod trust;

pub use auth::{
    authenticate_with_retry, logout_registry, resolve_credential, sanitize_token,
    validate_credentials,
};
pub use credentials::{CredentialStore, read_credentials, write_credentials};
pub use http_client::HttpRegistryClient;
pub use registry_client::{
    RegistryClient, RegistryError, RegistryResponse, RegistrySearchResult, RetryPolicy,
};
pub use registry_config::{
    AuthMethod, RegistryConfig, RegistryCredential, TrustLevel, find_registry_for_specifier,
    parse_registries_from_config,
};
pub use registry_ops::{
    TrustCheck, assign_trust_level, publish_to_registry, resolve_from_registry, search_registries,
    verify_package_signature, verify_registry_integrity,
};
pub use resolver::resolve_version;
pub use trust::{
    KnownKeys, known_keys_path, load_known_keys, load_known_keys_at, save_known_keys,
    save_known_keys_at,
};
