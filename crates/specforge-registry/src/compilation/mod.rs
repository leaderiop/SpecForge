//! Registry compilation (architecture plan 05): loaded manifests in,
//! everything the compiler derives from them out. [`build_registries`] is
//! the one entry point and owns the step order; the checks a built graph
//! runs take [`EntityView`]s. The steps themselves are private.

mod build;
mod detection;
mod populate;
mod provider;
mod validate;
pub mod validation_engine;

pub use build::{RegistryBuild, build_registries};
// The registry checks `check_graph` runs over a built graph.
pub use detection::{
    EntityView, KeywordExtensionIndex, detect_identifier_length_violations,
    detect_mistyped_references, detect_reserved_entity_ids, detect_unknown_entity_fields,
    detect_unknown_entity_kinds,
};
// The three registries alone: `build_registries` runs this first.
pub use populate::populate_registries;
// The `providers` specforge.json configures, and their schemes.
pub use provider::{
    ProviderConfig, ProviderSchemeRegistry, ProviderStatus, SchemeRegistryEntry,
    load_provider_configurations, register_provider_schemes, register_provider_schemes_with_status,
};
// Peer dependencies (compile reports them; `add` checks them first), and the
// kind collisions the Wasm manifest bridge reports.
pub use validate::{
    detect_duplicate_entity_kinds, validate_peer_dependencies, validate_peer_dependencies_of,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use populate::apply_entity_enhancements;
#[cfg(test)]
pub(crate) use validate::{
    register_validation_rules, validate_extension_testability, validate_registered_entity_fields,
};
