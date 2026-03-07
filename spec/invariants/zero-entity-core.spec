// Zero-entity core architecture invariants

use behaviors/zero-entity-lsp
use behaviors/zero-entity-registries
use behaviors/zero-entity-validation
use behaviors/extensions
use behaviors/lsp
use behaviors/parsing
use behaviors/init
use behaviors/validation
use behaviors/output
use behaviors/output-schema
use behaviors/incremental

invariant zero_domain_knowledge_core "Zero Domain Knowledge Core" {
  guarantee """
    The core compiler MUST have zero hardcoded entity types. All domain
    vocabulary — entity kinds, edge types, field definitions, validation
    rules, and testability flags — MUST come exclusively from installed
    extensions. The core MUST only understand structural parsing of
    keyword name { fields } blocks, use imports, reference lists, string
    fields, verify/gherkin declarations, and define meta-blocks.
  """
  enforced_by [
    boot_empty_kind_registry, boot_empty_field_registry, boot_empty_edge_registry,
    populate_kind_registry_from_extensions, populate_field_registry_from_extensions,
    register_entity_kinds_from_manifest, register_verify_kinds_from_manifest,
    detect_unknown_entity_kinds, detect_unknown_entity_fields,
    validate_registered_entity_fields,
    parse_validation_rule_pattern, execute_validation_pattern,
    emit_diagnostic_from_pattern, register_extension_validation_rules,
    register_validation_rules_from_manifest, register_custom_validation_patterns,
    graceful_degradation_without_extensions,
    complete_extension_defined_keywords, provide_extension_entity_semantic_tokens,
    provide_extension_entity_hover, provide_extension_defined_lsp_icons,
    register_edge_types_from_manifest,
    generate_schema_from_registries, embed_schema_in_export, serve_schema_resource,
    render_extension_defined_dot_shapes, collapse_grammar_to_generic_entity_block,
    parse_all_block_types, parse_gherkin_statements, parse_verify_statements, parse_define_blocks,
    provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries,
    two_phase_validate_semantic, suggest_missing_extensions,
    validate_extension_manifest_consistency,
    autocomplete_entity_ids, complete_field_names, complete_keywords,
    hover_information, code_actions_for_missing_tests, code_action_add_missing_import, code_action_create_entity_stub,
    outline_view, workspace_symbol_search,
    recover_from_syntax_errors, parse_spec_file_to_ast, parse_use_imports, parse_triple_quoted_strings, parse_ref_blocks,
    populate_edge_registry_from_extensions, two_phase_parse_structural, provide_semantic_tokens,
    load_extension_manifests, register_extension_entity_types,
    list_installed_extensions, custom_entity_types_via_define,
    scaffold_new_project, scaffold_starter_spec_file, non_interactive_init,
    interactive_extension_selection, find_project_root, graceful_zero_extension_init,
    load_provider_configurations, register_provider_schemes,
    validate_provider_refs, validate_ref_target_format, validate_provider_kinds,
    lsp_initialize,
    list_configured_providers, configure_registries,
    compute_project_statistics,
    validate_manifest_v2_schema,
    detect_duplicate_entity_kinds, validate_peer_dependencies,
    rebuild_affected_subgraph,
    render_extension_defined_edge_styles,
    mcp_initialize,
    emit_incremental_diagnostics,
    dispatch_incremental_validators,
    remove_extension,
  ]
  risk high

  verify property "core with zero extensions installed has zero entity kinds in KindRegistry"
  verify unit "compiling a .spec file with no extensions produces only structural parse, no kind validation"

}

invariant registry_population_before_validation "Registry Population Before Validation" {
  guarantee """
    All registries (KindRegistry, FieldRegistry, edge type set) MUST be
    fully populated from all installed extension manifests before any semantic
    validation begins. The two-phase approach MUST guarantee that Phase 1
    (structural parsing) completes for all files before Phase 2 (semantic
    validation against registries) starts.
  """
  enforced_by [
    load_extension_manifests, validate_manifest_v2_schema, boot_empty_edge_registry,
    two_phase_parse_structural, two_phase_validate_semantic,
    populate_kind_registry_from_extensions, populate_field_registry_from_extensions,
    populate_edge_registry_from_extensions, validate_registered_entity_fields,
    detect_unknown_entity_kinds, detect_unknown_entity_fields,
    register_entity_kinds_from_manifest, register_edge_types_from_manifest,
    register_validation_rules_from_manifest, register_verify_kinds_from_manifest,
    register_extension_validation_rules, validate_extension_manifest_consistency,
    generate_schema_from_registries,
    detect_duplicate_entity_kinds, validate_peer_dependencies,
  ]
  risk high

  verify property "no validation diagnostic references a kind that was registered after validation started"
  verify unit "adding an extension that defines kind X makes X available in the validation phase"

}

invariant declarative_validation_determinism "Declarative Validation Determinism" {
  guarantee """
    Given the same set of installed extensions and the same .spec source files,
    the declarative validation engine MUST produce an identical set of
    diagnostics on every invocation. The order of diagnostic emission MUST
    be deterministic. No randomness or timing-dependent logic MUST influence
    which diagnostics are produced or their ordering.
  """
  enforced_by [
    parse_validation_rule_pattern, execute_validation_pattern,
    emit_diagnostic_from_pattern, register_extension_validation_rules,
    register_validation_rules_from_manifest, register_custom_validation_patterns,
  ]
  risk medium

  verify property "same extensions and sources produce identical diagnostics across 100 runs"
  verify unit "diagnostic ordering is deterministic regardless of extension load order"

}

invariant testable_entity_classification "Testable Entity Classification" {
  guarantee """
    Testability and gherkin support MUST be determined exclusively by
    the KindRegistry entries populated from extension manifests. An entity
    kind with testable=true MUST accept verify statements and participate
    in coverage calculations and code action suggestions. An entity kind
    with supportsGherkin=true MUST accept gherkin declarations referencing
    external .feature files. These flags MUST NOT be hardcoded — they come
    from the extension's ManifestEntityKind declarations. The core compiler
    MUST NOT assume any entity kind is testable or supports gherkin by default.
  """

  enforced_by [
    register_entity_kinds_from_manifest,
    compute_project_statistics,
    code_actions_for_missing_tests,
    validate_extension_testability,
  ]

  risk medium

  verify unit "entity kind with testable=true in manifest accepts verify statements"
  verify unit "entity kind with supportsGherkin=true in manifest accepts gherkin declarations"
  verify unit "testable=true entity counts toward coverage"
  verify unit "testable=false entity excluded from coverage"
  verify unit "no default testability assumed by core"

}
