// Zero-entity core architecture invariants

use "behaviors/zero-entity-lsp"
use "behaviors/zero-entity-registries"
use "behaviors/zero-entity-validation"
use "behaviors/extensions"
use "behaviors/lsp"
use "behaviors/parsing"
use "behaviors/init"
use "behaviors/validation"
use "behaviors/output"
use "behaviors/output-schema"
use "behaviors/incremental"
use "behaviors/mcp-operations"
use "behaviors/wasm-host-functions"
use "behaviors/migration"
use "behaviors/resolution"
invariant zero_domain_knowledge_core "Zero Domain Knowledge Core" {
  guarantee """
    The core compiler MUST have zero hardcoded entity types. All domain
    vocabulary — entity kinds, edge types, field definitions, validation
    rules, and testability flags — MUST come exclusively from installed
    extensions. The core MUST only understand structural parsing of
    keyword name { fields } blocks, use imports, reference lists, string
    fields, verify declarations, and define meta-blocks.
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
    generate_schema_from_registries, embed_schema_in_export, persist_schema_cache, serve_schema_resource,
    serialize_json_graph, serialize_dot_visualization,
    render_extension_defined_dot_shapes, collapse_grammar_to_generic_entity_block,
    parse_all_block_types, parse_verify_statements, parse_define_blocks,
    provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries,
    two_phase_validate_semantic, suggest_missing_extensions,
    validate_extension_manifest_consistency,
    autocomplete_entity_ids, complete_field_names, complete_keywords,
    hover_information, code_actions_for_missing_verify, code_action_add_missing_import, code_action_create_entity_stub,
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
    go_to_definition, find_all_references, rename_entity_id, prepare_rename,
    shared_incremental_pipeline,
    compute_traceability_chain, serialize_traceability_data, serve_graph_resource,
    validate_agent_plan, print_diagnostics_structured, exit_code_reflects_diagnostic_severity,
    check_mode_for_ci, export_diagnostics_as_json,
    add_extension_to_existing_project, provide_mcp_init_tool,
    provide_host_function_add_graph_node, provide_host_function_add_graph_edge,
    validate_extension_testability,
    deterministic_output,
    export_agent_context_format,
    export_agent_brief_format,
    export_agent_graph_format,
    query_graph_multi_resolution,
    enforce_token_budget,
    format_diagnostics_with_source_context,
    provide_did_you_mean_suggestions,
    aggregate_diagnostic_summary,
    live_diagnostics,
    goto_import_definition,
    detect_format_version_mismatch,
    migrate_spec_files_in_place,
    generate_migration_diff,
    validate_post_migration_integrity,
    capture_pre_migration_schema_snapshot,
    verify_graph_protocol_compatibility_after_migration,
    rollback_failed_migration,
    register_grammar_contributions,
    register_body_parser_contributions,
    persist_schema_cache,
    negotiate_schema_version,
    detect_breaking_schema_changes,
    compute_schema_version,
    publish_schema_specification,
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
    mcp_initialize,
    register_grammar_contributions,
    register_body_parser_contributions,
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
    Testability MUST be determined exclusively by the KindRegistry entries
    populated from extension manifests. An entity kind with testable=true
    MUST accept verify statements and participate in coverage calculations
    and code action suggestions. These flags MUST NOT be hardcoded — they
    come from the extension's ManifestEntityKind declarations. The core
    compiler MUST NOT assume any entity kind is testable by default.

    P2 justification: The testable flag is structural dispatch, not domain
    knowledge. The core routes verify acceptance and coverage counting based
    on a boolean flag without knowing what "testable" means in any domain.
    This parallels Terraform providers declaring CRUD capabilities — the
    core routes based on capability flags without knowing what the resource
    is. The core never interprets why an entity is testable, only that the
    extension declared it so.
  """

  enforced_by [
    register_entity_kinds_from_manifest,
    compute_project_statistics,
    code_actions_for_missing_verify,
    validate_extension_testability,
    provide_mcp_coverage_tool,
  ]

  risk medium

  verify unit "entity kind with testable=true in manifest accepts verify statements"
  verify unit "testable=true entity counts toward coverage"
  verify unit "testable=false entity excluded from coverage"
  verify unit "no default testability assumed by core"

}

invariant define_extension_kind_uniqueness "Define-Extension Kind Uniqueness" {
  guarantee """
    A define block MUST NOT register a kind name that is already registered
    by an installed extension. If a define block declares a kind name that
    collides with an extension-provided kind, the compiler MUST emit an
    E-level diagnostic identifying both the define block and the owning
    extension. Extension-registered kinds always take precedence over
    define blocks — define blocks are project-local overrides for kinds
    NOT provided by any extension.
  """
  enforced_by [custom_entity_types_via_define, populate_kind_registry_from_extensions]
  risk medium

  verify unit "define block with kind name matching an extension kind produces E-level diagnostic"
  verify unit "define block with unique kind name succeeds"

}

invariant compilation_pipeline_ordering "Compilation Pipeline Ordering" {
  guarantee """
    The compilation pipeline MUST execute events in strict order:
    all_files_parsed → extension_manifests_loaded → registries_populated →
    define_blocks_registered → validation_complete. No phase MAY begin
    before all prior phases have completed.
  """
  enforced_by [
    two_phase_parse_structural, load_extension_manifests,
    populate_kind_registry_from_extensions, populate_field_registry_from_extensions,
    populate_edge_registry_from_extensions, custom_entity_types_via_define,
    two_phase_validate_semantic, resolve_use_imports,
  ]
  risk critical

  verify property "pipeline events fire in declared order"

}
