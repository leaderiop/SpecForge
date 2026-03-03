// Roadmap — delivery phases

use features/parsing
use features/validation
use features/project-init
use features/incremental
use features/output
use features/codegen
use features/coverage
use features/rust-codegen
use features/rust-collection
use features/migration
use features/lsp
use features/extensions
use features/wasm
use invariants/wasm
use features/formatting
use product/libraries
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
use behaviors/migration
use behaviors/lsp
use behaviors/extensions
use behaviors/wasm
use behaviors/error-reporting
use behaviors/formatting

roadmap core_compiler "Phase 1: Core Compiler" {
  status     completed
  behaviors  [
    scaffold_new_project, interactive_plugin_selection, add_plugin_to_existing_project,
    parse_spec_file_to_ast, recover_from_syntax_errors, parse_use_imports, parse_all_block_types, parse_triple_quoted_strings, parse_scenario_blocks, parse_generic_entity_blocks,
    provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries,
    resolve_use_imports, detect_import_cycles, link_entity_references, resolve_soft_cross_plugin_references, resolve_external_ref_declarations,
    build_in_memory_graph, maintain_mutable_graph, compute_subgraph_for_invalidation,
    detect_dangling_references, detect_duplicate_entity_ids, detect_import_cycles, detect_orphan_behaviors,
    detect_unused_invariants, detect_unverified_behaviors, detect_orphan_events, validate_event_triggers,
    validate_persona_references, validate_surface_references,
    validate_ref_target_format, validate_provider_kinds, detect_orphan_refs,
    detect_orphan_features, detect_library_cycles, validate_behavior_ranges_in_roadmaps,
    validate_rpn_arithmetic, detect_unmitigated_high_risk_invariants, detect_orphan_capabilities, detect_features_with_empty_behaviors,
    validate_import_cycles, validate_empty_scenario, validate_duplicate_scenario_titles, validate_scenario_steps,
  ]
  features   [project_initialization, spec_file_parsing, error_recovery_during_parsing, graph_validation, editor_query_files, scenario_declaration]
  libraries  [tree_sitter_specforge, specforge_parser, specforge_resolver, specforge_graph, specforge_validator, specforge_cli]

  criteria [
    "Tree-sitter grammar parses all 16 block types",
    "Resolver links all references and detects cycles",
    "Validator emits all E-level and W-level diagnostics",
    "specforge check passes on SpecForge's own .spec files",
    "Snapshot tests cover all diagnostic formats",
    "Query files provide syntax highlighting in Neovim and other editors",
    "Generic entity blocks produce clean AST nodes for plugin entities",
  ]
}

roadmap cli_and_watch_mode "Phase 2: CLI and Watch Mode" {
  status     completed
  behaviors  [
    watch_file_system_for_changes, invalidate_changed_files, rebuild_affected_subgraph, emit_incremental_diagnostics,
    render_markdown_documentation, render_json_graph, render_dot_visualization, compute_traceability_chain,
    compute_project_statistics, print_diagnostics_in_rustc_style, exit_code_reflects_diagnostic_severity,
    render_traceability_report, render_index_files, selective_render_by_entity_type, deterministic_output, check_mode_for_ci,
    detect_deliverables_with_no_capabilities, detect_orphan_libraries, detect_constraints_with_no_protected_invariants, detect_unused_glossary_terms,
  ]
  features   [incremental_compilation, markdown_documentation_generation, json_and_dot_export, traceability_reports, ci_integration, format_version_migration]
  libraries  [specforge_emitter, specforge_watch]

  criteria [
    "specforge watch delivers diagnostics within 100ms of file change",
    "Markdown, JSON, and DOT renderers produce correct output",
    "specforge trace prints full traceability chains",
    "specforge stats reports accurate entity counts",
    "specforge migrate handles version 1.0 format",
  ]
}

roadmap lsp_server "Phase 3: LSP Server" {
  status     planned
  behaviors  [
    go_to_definition, find_all_references, hover_information, autocomplete_entity_ids,
    rename_entity_id, live_diagnostics, code_actions_for_missing_tests, outline_view,
    workspace_symbol_search, shared_incremental_pipeline, provide_semantic_tokens,
  ]
  features   [go_to_definition_and_references, hover_and_autocomplete, rename_refactoring, live_diagnostics_feature, code_actions, outline_and_symbol_search]
  libraries  [specforge_lsp]

  criteria [
    "Go-to-definition and find-references work across files",
    "Hover shows entity details and contract text",
    "Autocomplete suggests entity IDs in reference lists",
    "Rename updates all references atomically",
    "Live diagnostics appear within 100ms",
    "LSP shares incremental pipeline with watch mode",
    "Semantic tokens classify custom entity keywords from plugins and define blocks",
  ]
}

roadmap code_generation "Phase 4: Code Generation" {
  status     planned
  behaviors  [
    generate_typescript_interfaces_from_types, generate_port_interfaces, generate_test_stubs,
    detect_generated_code_drift, verify_adapter_implementations, generate_json_schema_from_types,
    respect_naming_conventions, generate_readonly_fields, generate_unique_constraints,
    call_package_generators, plugin_wasm_protocol, incremental_code_generation, support_multiple_languages,
  ]
  features   [type_and_port_code_generation, test_stub_generation_and_drift_detection]
  libraries  [specforge_gen_typescript]

  criteria [
    "TypeScript interfaces generated from type and port blocks",
    "Test stubs generated from verify statements",
    "Drift detection catches stale generated code",
    "Adapter verification confirms port implementations",
    "Wasm plugin protocol works with external generators via host functions",
  ]
}

roadmap extensions_and_coverage "Phase 5: Extensions and Coverage" {
  status     planned
  behaviors  [
    merge_coverage_reports, compute_coverage_summary, gate_on_coverage_threshold, validate_test_ids_against_spec, consume_specforge_report, compute_three_layer_coverage, render_test_traceability_matrix,
    migrate_spec_files_between_versions, detect_format_version_mismatch,
    load_plugin_manifests, register_plugin_entity_types, load_provider_configurations, validate_provider_refs,
    execute_generator_plugins, remove_plugin, list_installed_plugins, custom_entity_types_via_define,
    validate_generator_configuration, list_configured_providers,
    validate_tests_field_references, validate_plugin_testability,
    format_diagnostics_with_source_context, provide_did_you_mean_suggestions, aggregate_diagnostic_summary,
    load_wasm_module, initialize_wasm_package, call_package_validators,
    provide_host_function_query_graph, provide_host_function_emit_diagnostic,
    provide_host_function_register_entity, provide_host_function_register_edge,
    provide_host_function_emit_file, provide_host_function_http_get,
    enforce_wasm_sandbox, aot_compile_wasm_module, cache_aot_artifacts, warm_wasm_engine_instance,
    validate_package_peer_dependencies, topological_sort_packages,
    provide_plugin_query_extensions, compose_query_files_from_plugins,
    install_wasm_package, validate_package_manifest, handle_wasm_trap,
    invalidate_aot_cache, discover_packages, configure_sandbox_policy, upgrade_wasm_package,
    reject_reserved_entity_kind, detect_entity_kind_collision,
    resolve_entity_kind_conflict_via_config, qualify_entity_kind_inline,
    parse_package_specifier, resolve_package_source, write_lock_file, read_lock_file, verify_wasm_integrity,
    dispatch_contribution_exports, enforce_per_call_site_permissions, validate_contribution_exports,
    toggle_package_contributions, migrate_v1_manifest,
  ]
  features   [test_coverage_reporting, test_traceability, plugin_management, provider_based_ref_validation, generator_plugin_protocol, wasm_package_runtime, package_syntax_extensions, wasm_package_lifecycle, entity_kind_conflict_prevention, contribution_based_extensions, package_source_resolution, package_version_management]
  libraries  [specforge_plugin_product, specforge_plugin_governance, specforge_provider_gh, specforge_coverage, specforge_wasm]

  criteria [
    "specforge coverage merges reports and gates on threshold",
    "Package add/remove works without breaking existing spec files",
    "Provider-based ref validation catches malformed identifiers",
    "Wasm package runtime loads, initializes, and validates packages",
    "Host functions (query_graph, emit_diagnostic, emit_file, http_get) work correctly",
    "AOT compilation reduces cold start to <50ms per package",
    "Sandbox enforcement blocks unauthorized filesystem and network access",
    "Peer dependency validation catches missing or incompatible packages",
    "Custom entity types via define blocks participate in validation",
    "Package query extensions compose with base queries for editor delivery",
    "Package install/upgrade resolves from registry, local, and git sources",
    "Wasm traps produce structured diagnostics without crashing the compiler",
    "AOT cache self-heals on corruption and invalidates on runtime upgrade",
    "Entity kind conflicts between packages are detected and resolved via config",
    "Contribution-based dispatch routes to correct exports per contribution type",
    "Per-call-site permissions enforce least-privilege for each export",
    "specforge.lock pins exact versions with SHA256 integrity hashes",
  ]
}

roadmap wasm_package_authoring_phase "Phase 5b: Wasm Package Authoring" {
  status     planned
  behaviors  [
    scaffold_wasm_package_project, build_wasm_package, test_wasm_package_locally, publish_wasm_package,
  ]
  features   [wasm_package_authoring]
  libraries  [specforge_wasm]

  criteria [
    "specforge package init scaffolds a working Wasm package project",
    "specforge package build produces a valid .wasm binary",
    "specforge package test runs against fixtures in production sandbox",
    "specforge package publish packages and uploads to npm/OCI/GitHub",
  ]
}

roadmap rust_integration "Phase 6: Rust Integration" {
  status     planned
  behaviors  [
    generate_rust_structs_from_types, generate_rust_traits_from_ports,
    generate_rust_test_stubs, generate_rust_bench_stubs, generate_rust_module_tree,
    slugify_verify_descriptions, detect_rust_code_drift, safe_rust_regeneration,
    collect_rust_test_results, parse_junit_xml, parse_libtest_json,
    resolve_entity_mapping, validate_rust_entity_ids, merge_workspace_reports,
    record_test_via_drop_guard, emit_specforge_report_from_rust,
  ]
  features   [rust_type_and_port_generation, rust_test_stub_generation, rust_test_collection, rust_proc_macro_annotation]
  libraries  [specforge_gen_rust, specforge_test_lib, specforge_test_macros_lib]

  criteria [
    "specforge gen rust produces compilable Rust structs, traits, and test stubs",
    "SHA256 checksum drift detection catches stale generated files",
    "specforge collect rust parses JUnit XML and emits valid specforge-report.json",
    "Three-level entity mapping resolves tests to entities correctly",
    "Proc macro Drop guard records pass/fail for annotated tests",
    "Workspace report merging handles multiple test binaries",
    "Convention-based Phase 1 works with zero user-side Rust dependencies",
  ]
}

roadmap code_formatting_phase "Phase 7: Code Formatting" {
  status     planned
  behaviors  [
    format_spec_files, preserve_comments, check_formatting, show_formatting_diff,
    format_from_stdin, load_format_config, apply_format_rules, maintain_format_idempotency,
    lsp_format_document, lsp_format_range, lsp_respect_editor_config,
  ]
  features   [code_formatting, lsp_format_on_save]
  libraries  [specforge_formatter]

  criteria [
    "specforge format runs on SpecForge's own spec/ directory",
    "format(format(x)) == format(x) verified by property tests",
    "All comments preserved after formatting",
    "Single file formatted in under 50ms",
    "specforge format --check exits 1 on unformatted files",
    "LSP textDocument/formatting produces same result as CLI",
    "Range formatting matches full formatting for affected blocks",
  ]
}
