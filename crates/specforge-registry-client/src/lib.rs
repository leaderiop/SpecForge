//! The package-registry client: talking to a SpecForge package registry
//! over HTTP (search, resolve, publish), registry credentials in the OS
//! keyring, publisher trust (TOFU key pinning) and ed25519 package signing.
//!
//! It is not the Registry build (`specforge-registry`), which is pure and
//! which this crate does not depend on. It reads the extension declaration
//! and the package name from `specforge-protocol-types` and diagnostics from
//! `specforge-common`.

pub mod auth;
pub mod credential_health;
pub mod credentials;
pub mod http_client;
pub mod registry_client;
pub mod registry_config;
pub mod registry_ops;
pub mod secrets;
pub mod signing;
pub mod trust;
pub mod trust_flow;

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
    AuthMethod, RegistryConfig, RegistryCredential, parse_registries_from_config,
};
pub use registry_ops::{
    TrustCheck, publish_to_registry, resolve_from_registry, search_registries,
    verify_package_signature, verify_registry_integrity,
};
pub use signing::{
    PackageSignature, SigningKey, load_or_create_signing_key, signing_key_path, verify_signature,
};
pub use trust::{
    KnownKeys, known_keys_path, load_known_keys, load_known_keys_at, save_known_keys,
    save_known_keys_at,
};
