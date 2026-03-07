// Core constraints — non-functional requirements for the compiler engine
//
// Extension-specific constraints live in their respective extension directories
// under spec/extensions/.

use invariants/core
use invariants/validation
use behaviors/init
use behaviors/parsing
use behaviors/resolution
use behaviors/graph
use behaviors/validation
use behaviors/zero-entity-registries
use behaviors/zero-entity-validation
use behaviors/incremental
use behaviors/output
use behaviors/output-schema
use extensions/coverage/behaviors
use behaviors/lsp
use behaviors/extensions
use behaviors/wasm-authoring
use behaviors/wasm-extensions
use behaviors/wasm-host-functions
use behaviors/wasm-lifecycle
use behaviors/wasm-sandbox
use behaviors/error-reporting
use behaviors/formatting
use invariants/formatting
use invariants/wasm

constraint incremental_compilation_latency "Incremental Compilation Latency" {
  category    performance
  priority    must

  metric """
    file_change_to_diagnostics < 100ms
    with up to 500 .spec files in the project
  """

  constrains [watch_file_system_for_changes, invalidate_changed_files, rebuild_affected_subgraph, emit_incremental_diagnostics]
  protects [incremental_correctness]

  verify load "benchmark incremental recompile with 500 files, assert < 100ms"

}

constraint memory_usage "Memory Usage" {
  category    performance
  priority    must

  metric """
    peak_memory < 50MB for a project with 500 .spec files
    and approximately 2000 entities
  """

  constrains [build_in_memory_graph, maintain_mutable_graph]
  protects [string_interning_consistency]

  verify load "compile 500-file project, measure peak RSS < 50MB"

}

constraint cross_platform_compatibility "Cross-Platform Compatibility" {
  category    compatibility
  priority    must

  metric """
    The CLI and LSP binaries MUST build and run on:
    Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64).
    No platform-specific file path handling.
  """

  constrains [print_diagnostics_structured, watch_file_system_for_changes]
  protects [diagnostic_determinism]

  verify integration "CI matrix tests on all 5 platform targets"

}

constraint diagnostic_quality "Diagnostic Quality" {
  category    usability
  priority    must

  metric """
    Every error diagnostic MUST include: file path, line number,
    column number, context snippet, and at least one actionable
    suggestion when a close match exists.
  """

  constrains [format_diagnostics_with_source_context, provide_did_you_mean_suggestions, aggregate_diagnostic_summary]
  protects [diagnostic_determinism]

  verify unit "all error diagnostics include required fields"

}

constraint zero_runtime_dependencies "Zero Runtime Dependencies" {
  category    portability
  priority    must

  metric """
    The specforge-cli and specforge-lsp binaries MUST be statically
    linked with zero runtime dependencies beyond the OS. No Node.js,
    Python, or JVM required at runtime. The Extism runtime is statically
    linked into the binary.
  """

  constrains [check_mode_for_ci, shared_incremental_pipeline]
  protects [multi_error_collection]

  verify integration "binary runs on clean OS install without additional packages"

}

constraint backward_compatibility "Backward Compatibility" {
  category    compatibility
  priority    should

  metric """
    The compiler SHOULD parse .spec files from the previous format version
    without errors. Deprecation warnings are acceptable.
  """

  constrains [parse_spec_file_to_ast, recover_from_syntax_errors]
  protects [multi_error_collection, spec_root_singleton]

  verify integration "previous version files compile with current compiler"

}

constraint cli_scaffolding_robustness "CLI Scaffolding Robustness" {
  category    usability
  priority    should

  metric """
    init/add commands produce valid, parseable specforge.json and a
    starter .spec file; existing files never overwritten without
    consent; non-interactive mode produces identical output to
    interactive mode for the same inputs
  """

  constrains [scaffold_new_project, scaffold_starter_spec_file, interactive_extension_selection, non_interactive_init, add_extension_to_existing_project, graceful_zero_extension_init]
  protects [spec_root_singleton, entity_id_uniqueness, init_config_validity]

  verify integration "scaffold and add commands produce valid spec files"
  verify integration "non-interactive init produces same output as interactive for same inputs"

}

constraint parser_correctness "Parser Correctness" {
  category    reliability
  priority    must

  metric """
    parser produces correct AST for all valid inputs and recovers
    from all syntax errors without crashing
  """

  constrains [parse_spec_file_to_ast, recover_from_syntax_errors, parse_use_imports, parse_all_block_types, parse_triple_quoted_strings, parse_gherkin_statements]
  protects [multi_error_collection]

  verify unit "parser handles all valid inputs and recovers from errors"

}

constraint editor_integration_quality "Editor Integration Quality" {
  category    usability
  priority    should

  metric """
    highlights.scm, folds.scm, indents.scm load without errors
    in Tree-sitter editors; extension entities receive syntax
    highlighting via generic entity_block rule and extension
    query extensions
  """

  constrains [provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries, parse_all_block_types, provide_extension_query_extensions, compose_query_files_from_extensions]
  protects [multi_error_collection]

  verify integration "all query files load in Tree-sitter editors"
  verify integration "extension entities receive highlighting via generic entity_block"

}

constraint reference_resolution_correctness "Reference Resolution Correctness" {
  category    reliability
  priority    must

  metric """
    every valid reference resolves exactly once; every invalid
    reference produces exactly one diagnostic
  """

  constrains [resolve_use_imports, detect_import_cycles, link_entity_references, resolve_soft_cross_extension_references, resolve_external_ref_declarations, compute_subgraph_for_invalidation]
  protects [import_dag, reference_resolution_completeness]

  verify unit "references resolve correctly with no false positives or negatives"

}

constraint validation_pass_correctness "Validation Pass Correctness" {
  category    reliability
  priority    must

  metric """
    each validation code fires if and only if the specified condition
    holds; zero false positives, zero false negatives
  """

  constrains [detect_dangling_references, detect_duplicate_entity_ids, detect_import_cycles, detect_orphan_refs, validate_file_reference_paths]
  protects [reference_resolution_completeness]

  verify unit "each validation code fires iff its condition holds"

}

constraint extension_validation_correctness "Extension Validation Correctness" {
  category    reliability
  priority    must

  metric """
    extension-specific validation rules run only when the owning extension is
    installed; no false positives when extension absent; the core declarative
    validation engine executes ValidationRulePattern entries from extension
    manifests without knowledge of domain-specific entity types
  """

  constrains [execute_validation_pattern, call_extension_validators, validate_extension_testability]
  protects [reference_resolution_completeness]

  verify unit "extension checks activate only when extension is installed"
  verify unit "uninstalled extension rules do not fire"

}

constraint output_format_correctness "Output Format Correctness" {
  category    reliability
  priority    must

  metric """
    every emitter produces syntactically valid output in its format
    (JSON, DOT); deterministic across runs
  """

  constrains [serialize_json_graph, serialize_dot_visualization, compute_traceability_chain, compute_project_statistics, exit_code_reflects_diagnostic_severity, serialize_traceability_data, deterministic_output, export_agent_context_format, export_agent_brief_format, export_agent_graph_format, query_graph_multi_resolution]
  protects [diagnostic_determinism]

  verify unit "all emitters produce syntactically valid, deterministic output"

}



constraint lsp_responsiveness "LSP Responsiveness" {
  category    usability
  priority    should

  metric """
    go-to-def, hover, completion respond within 50ms
    on a 500-file project
  """

  constrains [go_to_definition, find_all_references, hover_information, autocomplete_entity_ids, rename_entity_id, live_diagnostics, code_actions_for_missing_tests, outline_view, workspace_symbol_search, shared_incremental_pipeline, provide_semantic_tokens, complete_field_names, complete_keywords, goto_import_definition, code_action_add_missing_import, code_action_create_entity_stub, incremental_document_sync]
  protects [incremental_correctness]

  verify load "LSP features respond within 50ms on 500-file project"

}

constraint traceability_completeness "Traceability Completeness" {
  category    reliability
  priority    must

  metric """
    specforge trace MUST detect all broken links in the traceability
    chain with zero false negatives: missing tests field on testable
    entities with verify/gherkin, non-existent test file paths,
    missing report results for linked entities.
  """

  constrains [consume_specforge_report, compute_four_level_coverage, render_test_traceability_matrix, validate_file_reference_paths]
  protects [traceability_chain_integrity]

  verify unit "all broken traceability links are detected with zero false negatives"

}

constraint extension_system_integrity "Extension System Integrity" {
  category    reliability
  priority    must

  metric """
    extension install/remove never corrupts specforge.json; Wasm sandbox
    contains all extension execution; Wasm traps produce diagnostics, not
    crashes; provider registration survives reload; timeout enforcement
    prevents runaway extensions
  """

  constrains [load_extension_manifests, register_extension_entity_types, load_provider_configurations, validate_provider_refs, remove_extension, list_installed_extensions, custom_entity_types_via_define, list_configured_providers, validate_ref_target_format, validate_provider_kinds, load_wasm_module, initialize_wasm_extension, enforce_wasm_sandbox, validate_extension_peer_dependencies, call_extension_validators, provide_host_function_query_graph, provide_host_function_emit_diagnostic, provide_host_function_add_graph_node, provide_host_function_add_graph_edge, warm_wasm_engine_instance, topological_sort_extensions, load_extension_manifest, register_entity_enhancements, detect_enhancement_conflicts, resolve_enhancement_conflicts, run_doctor_check, scaffold_wasm_extension_project, build_wasm_extension, validate_wasm_extension_locally, publish_wasm_extension, reject_reserved_entity_kind, detect_entity_kind_collision, resolve_entity_kind_conflict_via_config, qualify_entity_kind_inline, upgrade_wasm_extension, validate_extension_manifest, handle_wasm_trap, discover_extensions, parse_extension_specifier, resolve_extension_source, write_lock_file, read_lock_file, verify_wasm_integrity, dispatch_contribution_exports, validate_contribution_exports, toggle_extension_contributions]
  protects [spec_root_singleton, reference_resolution_completeness, wasm_sandbox_integrity]

  verify integration "extension operations never corrupt state or crash"
  verify integration "Wasm traps produce diagnostics without crashing the compiler"

}



constraint wasm_cold_start_budget "Wasm Cold Start Budget" {
  category    performance
  priority    must

  metric """
    Each Wasm extension MUST load in under 50ms with AOT cache.
    First load (without cache) SHOULD complete AOT compilation
    in under 500ms for a typical extension (<1MB .wasm).
  """

  constrains [load_wasm_module, aot_compile_wasm_module, cache_aot_artifacts, warm_wasm_engine_instance]
  protects [extension_load_order_determinism]

  verify load "benchmark AOT-cached extension load, assert < 50ms"
  verify load "benchmark first-load AOT compilation for 1MB .wasm, assert < 500ms"

}

constraint wasm_memory_limit "Wasm Memory Limit" {
  category    performance
  priority    must

  metric """
    Each Wasm extension instance MUST be limited to 64MB of linear memory.
    Total memory across all loaded extensions MUST NOT exceed 256MB.
    Memory limit violations MUST trap the extension.
  """

  constrains [enforce_wasm_sandbox, load_wasm_module]
  protects [wasm_sandbox_integrity]

  verify unit "extension exceeding 64MB traps"
  verify unit "total memory exceeding 256MB prevents new extension load"

}

constraint wasm_sandbox_enforcement "Wasm Sandbox Enforcement" {
  category    security
  priority    must

  metric """
    Zero sandbox escapes in adversarial testing. Extensions MUST NOT
    access host filesystem, network, environment variables, or
    process control outside of designated host functions.
  """

  constrains [enforce_wasm_sandbox, provide_host_function_emit_file, provide_host_function_http_get, call_extension_validators, configure_sandbox_policy, enforce_per_call_site_permissions]
  protects [wasm_sandbox_integrity]

  verify integration "adversarial extension cannot escape sandbox"
  verify unit "direct filesystem access from Wasm is blocked"
  verify unit "direct network access from Wasm is blocked"

}

constraint wasm_binary_size_limit "Wasm Binary Size Limit" {
  category    portability
  priority    should

  metric """
    Each extension .wasm binary SHOULD be under 5MB. Binaries exceeding 10MB
    MUST produce a warning diagnostic at install time. This ensures
    reasonable download times and AOT compilation latency.
  """

  constrains [install_wasm_extension, build_wasm_extension, aot_compile_wasm_module]
  protects [extension_load_order_determinism]

  verify unit "extension under 5MB installs without warning"
  verify unit "extension over 10MB produces warning at install"

}

constraint extension_count_limit "Extension Count Limit" {
  category    performance
  priority    should

  metric """
    The system SHOULD support up to 20 extensions without performance
    degradation. 21-50 extensions MUST produce a warning diagnostic.
    More than 50 extensions MUST be refused with a hard error.
  """

  constrains [load_wasm_module, initialize_wasm_extension, topological_sort_extensions]
  protects [wasm_sandbox_integrity]

  verify unit "20 extensions load without warning"
  verify unit "21+ extensions produce warning"
  verify unit "51+ extensions produce hard error"

}

constraint aot_cache_size_limit "AOT Cache Size Limit" {
  category    portability
  priority    should

  metric """
    The AOT cache directory SHOULD stay under 500MB total. When the cache
    exceeds this limit, LRU eviction MUST remove the least recently used
    entries and an info diagnostic MUST be emitted.
  """

  constrains [aot_compile_wasm_module, cache_aot_artifacts, invalidate_aot_cache]
  protects [aot_cache_integrity]

  verify unit "cache under 500MB operates normally"
  verify unit "cache over 500MB triggers LRU eviction"
  verify unit "eviction emits info diagnostic"

}

constraint formatting_correctness "Formatting Correctness" {
  category    reliability
  priority    must

  metric """
    formatter output is idempotent (format(format(x)) == format(x));
    all comments preserved; formatted files produce identical compiler graph;
    single file formatted in under 50ms
  """

  constrains [format_spec_files, preserve_comments, check_formatting, show_formatting_diff, format_from_stdin, load_format_config, apply_format_rules, maintain_format_idempotency, lsp_format_document, lsp_format_range, lsp_respect_editor_config]
  protects [formatting_idempotency, comment_preservation, formatting_consistency]

  verify property "format(format(x)) == format(x) for all valid inputs"
  verify unit "all comments preserved after formatting round-trip"

}

constraint schema_publication_accessibility "Schema Publication Accessibility" {
  category    interoperability
  priority    must

  metric """
    The Graph Protocol schema MUST be publicly accessible as a standalone
    JSON Schema document. The published schema MUST be downloadable without
    authentication. The schema URL MUST be stable across patch versions.
  """

  constrains [publish_schema_specification, embed_schema_in_export, serve_schema_resource]
  protects [graph_schema_completeness, schema_version_backward_compatibility]

  verify integration "published schema is accessible without authentication"
  verify unit "schema URL is stable across patch versions"

}
