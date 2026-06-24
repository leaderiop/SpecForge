pub mod contributions;
pub mod define;
pub mod detection;
pub mod keyword_index;
mod populate;
pub mod provider;
mod validate;
pub mod validation_engine;

pub use contributions::{
    register_body_parser_contributions, register_grammar_contributions, GrammarConflictPolicy,
    RegisteredBodyParser, RegisteredGrammar,
};
pub use detection::{
    check_graceful_degradation, detect_mistyped_references, detect_unknown_entity_fields,
    detect_unknown_entity_kinds, detect_unknown_verify_kinds, generate_required_field_rules,
    handle_all_extensions_failed,
    lsp_keywords_with_registry, EntityRefInfo, KeywordExtensionIndex,
};
pub use keyword_index::generate_keyword_extension_index;
pub use keyword_index::KeywordExtensionIndex as ManifestKeywordIndex;
pub use populate::{apply_entity_enhancements, populate_registries};
pub use provider::{
    load_extension_manifests, load_provider_configurations, register_extension_entity_types,
    register_provider_schemes, validate_provider_ref, validate_ref_target_format,
    validate_provider_kinds, ProviderConfig, ProviderSchemeRegistry, SchemeRegistryEntry,
};
pub use validate::{
    detect_circular_peer_dependencies, detect_duplicate_entity_kinds,
    register_validation_rules, register_verify_kinds,
    validate_extension_testability, validate_host_api_versions, validate_peer_dependencies,
    validate_registered_entity_fields, HOST_API_VERSION,
};
