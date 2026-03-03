// Constraints — non-functional requirements

use invariants/core
use invariants/validation
use invariants/rust
use behaviors/init
use behaviors/parsing
use behaviors/resolution
use behaviors/graph
use behaviors/validation
use behaviors/validation-ext
use behaviors/incremental
use behaviors/output
use behaviors/codegen
use behaviors/coverage
use behaviors/rust-codegen
use behaviors/rust-collection
use behaviors/lsp
use behaviors/extensions
use behaviors/wasm
use behaviors/error-reporting
use behaviors/migration
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

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
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

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint cross_platform_compatibility "Cross-Platform Compatibility" {
  category    compatibility
  priority    must

  metric """
    The CLI and LSP binaries MUST build and run on:
    Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64).
    No platform-specific file path handling.
  """

  constrains [print_diagnostics_in_rustc_style, watch_file_system_for_changes]
  protects [diagnostic_determinism]

  verify integration "CI matrix tests on all 5 platform targets"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
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

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
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

  constrains [check_mode_for_ci, go_to_definition]
  protects [multi_error_collection]

  verify integration "binary runs on clean OS install without additional packages"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint backward_compatibility "Backward Compatibility" {
  category    compatibility
  priority    should

  metric """
    The compiler SHOULD parse .spec files from the previous format version
    without errors. Deprecation warnings are acceptable.
    Migration SHOULD be reversible.
  """

  constrains [migrate_spec_files_between_versions, detect_format_version_mismatch]
  protects [multi_error_collection, spec_root_singleton]

  verify integration "previous version files compile with current compiler"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint cli_scaffolding_robustness "CLI Scaffolding Robustness" {
  category    usability
  priority    should

  metric """
    init/add commands produce valid, parseable specforge.json;
    existing files never overwritten without consent
  """

  constrains [scaffold_new_project, interactive_plugin_selection, add_plugin_to_existing_project]
  protects [spec_root_singleton, entity_id_uniqueness]

  verify integration "scaffold and add commands produce valid spec files"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint parser_correctness "Parser Correctness" {
  category    reliability
  priority    must

  metric """
    parser produces correct AST for all valid inputs and recovers
    from all syntax errors without crashing
  """

  constrains [parse_spec_file_to_ast, recover_from_syntax_errors, parse_use_imports, parse_all_block_types, parse_triple_quoted_strings, parse_scenario_blocks]
  protects [multi_error_collection]

  verify unit "parser handles all valid inputs and recovers from errors"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-parser/tests/snapshot_tests.rs"]
}

constraint editor_integration_quality "Editor Integration Quality" {
  category    usability
  priority    should

  metric """
    highlights.scm, folds.scm, indents.scm load without errors
    in Tree-sitter editors; plugin entities receive syntax
    highlighting via generic_entity_block fallback and plugin
    query extensions
  """

  constrains [provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries, parse_generic_entity_blocks, provide_plugin_query_extensions, compose_query_files_from_plugins]
  protects [multi_error_collection]

  verify integration "all query files load in Tree-sitter editors"
  verify integration "plugin entities receive highlighting via generic_entity_block"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

constraint reference_resolution_correctness "Reference Resolution Correctness" {
  category    reliability
  priority    must

  metric """
    every valid reference resolves exactly once; every invalid
    reference produces exactly one diagnostic
  """

  constrains [resolve_use_imports, detect_import_cycles, validate_import_cycles, link_entity_references, resolve_soft_cross_plugin_references, resolve_external_ref_declarations, compute_subgraph_for_invalidation]
  protects [import_dag, reference_resolution_completeness]

  verify unit "references resolve correctly with no false positives or negatives"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

constraint validation_pass_correctness "Validation Pass Correctness" {
  category    reliability
  priority    must

  metric """
    each validation code fires if and only if the specified condition
    holds; zero false positives, zero false negatives
  """

  constrains [detect_dangling_references, detect_duplicate_entity_ids, detect_import_cycles, detect_orphan_behaviors, detect_unused_invariants, detect_unverified_behaviors, detect_orphan_events, validate_event_triggers, validate_persona_references, validate_surface_references, detect_orphan_refs, validate_empty_scenario, validate_duplicate_scenario_titles, validate_scenario_steps]
  protects [reference_resolution_completeness]

  verify unit "each validation code fires iff its condition holds"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

constraint plugin_validation_correctness "Plugin Validation Correctness" {
  category    reliability
  priority    must

  metric """
    plugin-specific checks run only when the owning plugin is
    installed; no false positives when plugin absent
  """

  constrains [detect_orphan_features, detect_library_cycles, validate_behavior_ranges_in_roadmaps, validate_rpn_arithmetic, detect_unmitigated_high_risk_invariants, detect_orphan_capabilities, detect_features_with_empty_behaviors, detect_deliverables_with_no_capabilities, detect_orphan_libraries, detect_constraints_with_no_protected_invariants, detect_unused_glossary_terms, validate_plugin_testability]
  protects [library_dag, rpn_arithmetic_integrity]

  verify unit "plugin checks activate only when plugin is installed"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint output_format_correctness "Output Format Correctness" {
  category    reliability
  priority    must

  metric """
    every emitter produces syntactically valid output in its format
    (Markdown, JSON, DOT); deterministic across runs
  """

  constrains [render_markdown_documentation, render_json_graph, render_dot_visualization, compute_traceability_chain, compute_project_statistics, exit_code_reflects_diagnostic_severity, render_traceability_report, render_index_files, selective_render_by_entity_type, deterministic_output]
  protects [diagnostic_determinism]

  verify unit "all emitters produce syntactically valid, deterministic output"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint code_generation_correctness "Code Generation Correctness" {
  category    reliability
  priority    must

  metric """
    generated TypeScript compiles with tsc --strict; generated JSON
    Schema validates with ajv; drift detection has no false positives
  """

  constrains [generate_typescript_interfaces_from_types, generate_port_interfaces, generate_test_stubs, detect_generated_code_drift, verify_adapter_implementations, generate_json_schema_from_types, respect_naming_conventions, generate_readonly_fields, generate_unique_constraints, plugin_wasm_protocol, incremental_code_generation, support_multiple_languages]
  protects [entity_id_uniqueness, diagnostic_determinism]

  verify unit "generated code compiles and validates correctly"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint test_coverage_accuracy "Test Coverage Accuracy" {
  category    reliability
  priority    must

  metric """
    coverage percentage matches actual verified/total ratio;
    merge produces correct deduplicated results
  """

  constrains [merge_coverage_reports, compute_coverage_summary, gate_on_coverage_threshold, validate_test_ids_against_spec]
  protects [testable_entity_classification, traceability_chain_integrity]

  verify unit "coverage percentage and merge are accurate"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
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

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

constraint traceability_completeness "Traceability Completeness" {
  category    reliability
  priority    must

  metric """
    specforge trace MUST detect all broken links in the traceability
    chain with zero false negatives: missing tests field on testable
    entities with verify/scenario, non-existent test file paths,
    missing report results for linked entities.
  """

  constrains [consume_specforge_report, compute_three_layer_coverage, render_test_traceability_matrix, validate_tests_field_references]
  protects [traceability_chain_integrity]

  verify unit "all broken traceability links are detected with zero false negatives"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint extension_system_integrity "Extension System Integrity" {
  category    reliability
  priority    must

  metric """
    plugin install/remove never corrupts specforge.json; Wasm sandbox
    contains all plugin execution; Wasm traps produce diagnostics, not
    crashes; provider registration survives reload; timeout enforcement
    prevents runaway plugins
  """

  constrains [load_plugin_manifests, register_plugin_entity_types, load_provider_configurations, validate_provider_refs, execute_generator_plugins, remove_plugin, list_installed_plugins, custom_entity_types_via_define, validate_generator_configuration, list_configured_providers, validate_ref_target_format, validate_provider_kinds, load_wasm_module, initialize_wasm_package, enforce_wasm_sandbox, validate_package_peer_dependencies, call_package_validators, call_package_generators, provide_host_function_query_graph, provide_host_function_emit_diagnostic, provide_host_function_register_entity, provide_host_function_register_edge, warm_wasm_engine_instance, topological_sort_packages, load_package_manifest, register_entity_enhancements, detect_enhancement_conflicts, resolve_enhancement_conflicts, run_doctor_check, scaffold_wasm_package_project, build_wasm_package, test_wasm_package_locally, publish_wasm_package, reject_reserved_entity_kind, detect_entity_kind_collision, resolve_entity_kind_conflict_via_config, qualify_entity_kind_inline, upgrade_wasm_package, validate_package_manifest, handle_wasm_trap, discover_packages, parse_package_specifier, resolve_package_source, write_lock_file, read_lock_file, verify_wasm_integrity, dispatch_contribution_exports, validate_contribution_exports, toggle_package_contributions, migrate_v1_manifest]
  protects [spec_root_singleton, reference_resolution_completeness, wasm_sandbox_integrity]

  verify integration "extension operations never corrupt state or crash"
  verify integration "Wasm traps produce diagnostics without crashing the compiler"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint rust_generation_correctness "Rust Generation Correctness" {
  category    reliability
  priority    must

  metric """
    generated Rust compiles with rustc; SHA256 checksums are accurate;
    drift detection has zero false positives and zero false negatives
  """

  constrains [generate_rust_structs_from_types, generate_rust_traits_from_ports, generate_rust_test_stubs, generate_rust_bench_stubs, generate_rust_module_tree, slugify_verify_descriptions, detect_rust_code_drift, safe_rust_regeneration]
  protects [deterministic_rust_generation, rust_drift_detection_accuracy]

  verify unit "generated Rust code compiles and checksums are accurate"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
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

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint rust_collection_accuracy "Rust Collection Accuracy" {
  category    reliability
  priority    must

  metric """
    entity mapping has zero false positives; specforge-report.json
    conforms to the SpecforgeReport schema; all entity IDs are validated
  """

  constrains [collect_rust_test_results, parse_junit_xml, parse_libtest_json, resolve_entity_mapping, validate_rust_entity_ids, merge_workspace_reports, emit_specforge_report_from_rust, record_test_via_drop_guard]
  protects [entity_mapping_precedence]

  verify unit "entity mapping and report generation are accurate"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint wasm_cold_start_budget "Wasm Cold Start Budget" {
  category    performance
  priority    must

  metric """
    Each Wasm plugin MUST load in under 50ms with AOT cache.
    First load (without cache) SHOULD complete AOT compilation
    in under 500ms for a typical plugin (<1MB .wasm).
  """

  constrains [load_wasm_module, aot_compile_wasm_module, cache_aot_artifacts, warm_wasm_engine_instance]
  protects [plugin_load_order_determinism]

  verify load "benchmark AOT-cached plugin load, assert < 50ms"
  verify load "benchmark first-load AOT compilation for 1MB .wasm, assert < 500ms"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint wasm_memory_limit "Wasm Memory Limit" {
  category    performance
  priority    must

  metric """
    Each Wasm plugin instance MUST be limited to 64MB of linear memory.
    Total memory across all loaded plugins MUST NOT exceed 256MB.
    Memory limit violations MUST trap the plugin.
  """

  constrains [enforce_wasm_sandbox, load_wasm_module]
  protects [wasm_sandbox_integrity]

  verify unit "plugin exceeding 64MB traps"
  verify unit "total memory exceeding 256MB prevents new plugin load"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint wasm_sandbox_enforcement "Wasm Sandbox Enforcement" {
  category    security
  priority    must

  metric """
    Zero sandbox escapes in adversarial testing. Plugins MUST NOT
    access host filesystem, network, environment variables, or
    process control outside of designated host functions.
  """

  constrains [enforce_wasm_sandbox, provide_host_function_emit_file, provide_host_function_http_get, call_package_validators, call_package_generators, configure_sandbox_policy, enforce_per_call_site_permissions]
  protects [wasm_sandbox_integrity]

  verify integration "adversarial plugin cannot escape sandbox"
  verify unit "direct filesystem access from Wasm is blocked"
  verify unit "direct network access from Wasm is blocked"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint wasm_binary_size_limit "Wasm Binary Size Limit" {
  category    portability
  priority    should

  metric """
    Each plugin .wasm binary SHOULD be under 5MB. Binaries exceeding 10MB
    MUST produce a warning diagnostic at install time. This ensures
    reasonable download times and AOT compilation latency.
  """

  constrains [install_wasm_package, build_wasm_package, aot_compile_wasm_module]
  protects [plugin_load_order_determinism]

  verify unit "plugin under 5MB installs without warning"
  verify unit "plugin over 10MB produces warning at install"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

constraint plugin_count_limit "Plugin Count Limit" {
  category    performance
  priority    should

  metric """
    The system SHOULD support up to 20 plugins without performance
    degradation. 21-50 plugins MUST produce a warning diagnostic.
    More than 50 plugins MUST be refused with a hard error.
  """

  constrains [load_wasm_module, initialize_wasm_package, topological_sort_packages]
  protects [wasm_sandbox_integrity]

  verify unit "20 plugins load without warning"
  verify unit "21+ plugins produce warning"
  verify unit "51+ plugins produce hard error"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
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

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
