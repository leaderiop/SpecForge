//! Registry compilation: the loaded declarations in, everything the
//! compiler derives from them out (ADR 0012). [`build_registries`] is
//! the one entry point: it owns the step order, and it is what every
//! test calls. The steps (declaration checks, populate, rules) are
//! private and free to change. The checks a built graph runs take
//! [`EntityView`]s.

mod build;
mod declaration;
mod detection;
mod populate;
mod provider;
mod validate;
pub mod validation_engine;

pub use build::{CHECK_PHASE, DeclaredPass, RegistryBuild, build_registries};
// The registry checks `check_graph` runs over a built graph.
pub use detection::{
    EntityView, KeywordExtensionIndex, detect_identifier_length_violations,
    detect_mistyped_references, detect_reserved_entity_ids, detect_unknown_entity_fields,
    detect_unknown_entity_kinds,
};
// The `providers` specforge.json configures, and their schemes.
pub use provider::{
    ProviderConfig, ProviderSchemeRegistry, ProviderStatus, SchemeRegistryEntry,
    load_provider_configurations, register_provider_schemes, register_provider_schemes_with_status,
};

// The rule engine's tests, until plan 02 moves them to tests/.
#[cfg(test)]
mod tests;
