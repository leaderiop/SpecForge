mod build;
pub mod detection;
mod populate;
pub mod provider;
mod validate;
pub mod validation_engine;

pub use build::{RegistryBuild, build_registries};
pub use detection::{
    EntityView, KeywordExtensionIndex, detect_identifier_length_violations,
    detect_mistyped_references, detect_reserved_entity_ids, detect_unknown_entity_fields,
    detect_unknown_entity_kinds, generate_required_field_rules, reserved_entity_id_words,
};
pub use populate::{apply_entity_enhancements, populate_registries};
pub use provider::{
    ProviderConfig, ProviderSchemeRegistry, ProviderStatus, SchemeRegistryEntry,
    load_provider_configurations, register_provider_schemes, register_provider_schemes_with_status,
};
pub use validate::{
    detect_duplicate_entity_kinds, register_validation_rules, validate_extension_testability,
    validate_peer_dependencies, validate_registered_entity_fields,
};
