// Validation-specific invariants

use behaviors/resolution
use behaviors/validation
use behaviors/error-reporting
use behaviors/output
use behaviors/output-schema
use behaviors/incremental
use behaviors/extensions
use behaviors/zero-entity-registries
use behaviors/zero-entity-validation
use behaviors/lsp
use behaviors/mcp-operations
use behaviors/mcp-prompts
use behaviors/mcp-server
use behaviors/mcp-tools
use behaviors/migration

invariant reference_resolution_completeness "Reference Resolution Completeness" {
  guarantee """
    Every entity ID in a reference list MUST resolve to a declared entity.
    The compiler MUST emit E001 for unresolvable hard references and I004
    for unresolvable soft references (cross-extension). No reference MUST be
    silently ignored.
  """
  enforced_by [link_entity_references, detect_dangling_references, resolve_use_imports, resolve_soft_cross_extension_references, resolve_external_ref_declarations, validate_provider_refs, validate_ref_target_format, validate_provider_kinds, provide_did_you_mean_suggestions, load_provider_configurations, register_provider_schemes, register_extension_entity_types, remove_extension, custom_entity_types_via_define, code_action_add_missing_import, validate_file_reference_paths, go_to_definition, find_all_references, goto_import_definition, provide_mcp_trace_tool, hover_information, autocomplete_entity_ids, outline_view, workspace_symbol_search]
  risk high

  verify property "every entity ID in a reference list resolves to a declared entity or emits a diagnostic"
  verify unit "E001 is emitted for broken hard references and I004 for broken soft references"

}

invariant diagnostic_determinism "Diagnostic Determinism" {
  guarantee """
    Given identical .spec source files, the compiler MUST produce identical
    diagnostics in the same order. No diagnostic MUST depend on filesystem
    iteration order, hashmap ordering, or wall-clock time.
  """
  enforced_by [format_diagnostics_with_source_context, aggregate_diagnostic_summary, emit_incremental_diagnostics, live_diagnostics, deterministic_output, print_diagnostics_structured, export_diagnostics_as_json, serialize_json_graph, serialize_dot_visualization, detect_orphan_refs, compute_project_statistics, export_agent_context_format, export_agent_brief_format, export_agent_graph_format, query_graph_multi_resolution, check_mode_for_ci, serialize_traceability_data, compute_traceability_chain, validate_agent_plan, serve_graph_resource, list_installed_extensions, list_configured_providers, search_registry, configure_registries, embed_schema_in_export, serve_schema_resource, notify_delta_subscribers, register_provider_schemes, dispatch_incremental_validators, generate_migration_diff, expose_graph_as_mcp_resource, expose_schema_as_mcp_resource, provide_mcp_query_tool, provide_mcp_validate_tool, provide_mcp_export_tool, provide_mcp_trace_tool, validate_registry_credentials, notify_graph_delta_via_mcp, debounce_file_changes, enforce_token_budget, negotiate_schema_version, detect_breaking_schema_changes, publish_schema_specification, expose_context_as_mcp_resource, expose_brief_as_mcp_resource, expose_diagnostics_as_mcp_resource, expose_entity_as_mcp_resource, notify_diagnostics_delta_via_mcp, provide_mcp_search_tool, provide_mcp_schema_tool, provide_mcp_coverage_tool, provide_mcp_stats_tool, provide_mcp_inspect_tool, provide_mcp_find_definition_tool, provide_mcp_find_references_tool, provide_mcp_outline_tool, provide_mcp_suggest_fixes_tool, provide_mcp_format_tool, provide_mcp_rename_tool, provide_mcp_init_tool, provide_mcp_add_extension_tool, provide_mcp_remove_extension_tool, provide_mcp_migrate_tool, provide_mcp_extensions_tool, provide_mcp_providers_tool, provide_mcp_doctor_tool, provide_mcp_collect_tool, provide_mcp_render_tool, provide_mcp_implement_prompt, provide_mcp_review_prompt, provide_mcp_trace_prompt, provide_mcp_explore_prompt]
  risk medium

  verify property "identical source files produce identical diagnostics in the same order"
  verify unit "diagnostic output does not depend on filesystem iteration order or hashmap ordering"

}

// testable_entity_classification is defined in invariants/zero-entity-core.spec
// (merged from here to eliminate E002 duplicate entity ID)

invariant validation_pipeline_ordering "Validation Phase Ordering" {
  guarantee """
    Post-graph-built structural validators MUST complete before the diagnostic
    summary is aggregated. The pipeline order MUST be: graph_built →
    core structural validators → extension declarative validators →
    aggregate_diagnostic_summary → validation_complete.
  """
  enforced_by [detect_dangling_references, detect_orphan_refs, validate_file_reference_paths, aggregate_diagnostic_summary, execute_validation_pattern]
  risk medium

  verify integration "core structural validators complete before declarative_validation_executed fires"
  verify property "pipeline phases execute in declared order"
}
