//! Registry compilation: the loaded declarations in, everything the
//! compiler derives from them out (ADR 0012). [`build_registries`] is
//! the one entry point: it owns the step order, and it is what every
//! test calls. The steps (declaration checks, populate, rules) are
//! private and free to change. The checks over a built graph's records
//! are `RegistryBuild::check`'s.

mod build;
mod declaration;
mod populate;
mod provider;
mod validate;

pub use build::{CHECK_PHASE, DeclaredPass, RegistryBuild, build_registries};
// The `providers` specforge.json configures, and their schemes.
pub use provider::{
    ProviderConfig, ProviderSchemeRegistry, ProviderStatus, SchemeRegistryEntry,
    load_provider_configurations, register_provider_schemes, register_provider_schemes_with_status,
};
