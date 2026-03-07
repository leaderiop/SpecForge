// Roadmap — delivery phases

use features/parsing
use features/validation
use features/project-init
use features/incremental
use features/output
use extensions/coverage/features
use extensions/rust/features
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
use behaviors/incremental
use behaviors/output
use extensions/coverage/behaviors
use extensions/rust/behaviors
use behaviors/lsp
use behaviors/extensions
use behaviors/wasm-authoring
use behaviors/wasm-extensions
use behaviors/wasm-host-functions
use behaviors/wasm-lifecycle
use behaviors/wasm-sandbox
use behaviors/error-reporting
use behaviors/formatting
use features/zero-entity-core
use behaviors/zero-entity-lsp
use behaviors/zero-entity-registries
use behaviors/zero-entity-validation
use invariants/zero-entity-core
use types/zero-entity-core
use features/mcp
use behaviors/mcp-server
use behaviors/mcp-tools

roadmap core_compiler "Phase 1: Core Compiler" {
  status     completed
  behaviors  [
    scaffold_new_project, scaffold_starter_spec_file, interactive_extension_selection, non_interactive_init, add_extension_to_existing_project, graceful_zero_extension_init,
    parse_spec_file_to_ast, recover_from_syntax_errors, parse_use_imports, parse_all_block_types, parse_triple_quoted_strings, parse_gherkin_statements,
    provide_syntax_highlighting_queries, provide_code_folding_queries, provide_indentation_queries,
    resolve_use_imports, detect_import_cycles, link_entity_references, resolve_soft_cross_extension_references, resolve_external_ref_declarations,
    build_in_memory_graph, maintain_mutable_graph, compute_subgraph_for_invalidation,
    detect_dangling_references, detect_duplicate_entity_ids,
    validate_ref_target_format, validate_provider_kinds, detect_orphan_refs,
    detect_library_cycles, validate_behavior_ranges_in_roadmaps,
    validate_rpn_arithmetic, validate_file_reference_paths,
    // Domain-specific validations (orphan detection, testability checks, etc.)
    // are declared in extension manifests and executed via execute_validation_pattern
    // in Phase 8. See spec/extensions/*/validation-rules.spec.
  ]
  features   [project_initialization, spec_file_parsing, error_recovery_during_parsing, reference_resolution, graph_construction, structural_validation, diagnostic_reporting, editor_query_files, gherkin_bridge]
  libraries  [tree_sitter_specforge, specforge_parser, specforge_resolver, specforge_graph, specforge_validator, specforge_cli]

  criteria [
    "Tree-sitter grammar parses any keyword via generic entity_block rule",
    "Resolver links all references and detects cycles",
    "Validator emits all E-level and W-level diagnostics",
    "specforge check passes on SpecForge's own .spec files",
    "Snapshot tests cover all diagnostic formats",
    "Query files provide syntax highlighting in Neovim and other editors",
    "Generic entity blocks produce clean AST nodes for extension entities",
    "specforge init creates specforge.json and starter .spec file in under 60 seconds",
  ]
}

roadmap cli_and_watch_mode "Phase 2: CLI and Watch Mode" {
  status     completed
  behaviors  [
    watch_file_system_for_changes, invalidate_changed_files, rebuild_affected_subgraph, emit_incremental_diagnostics,
    serialize_json_graph, serialize_dot_visualization, compute_traceability_chain,
    compute_project_statistics, print_diagnostics_structured, exit_code_reflects_diagnostic_severity,
    serialize_traceability_data, deterministic_output, check_mode_for_ci,
    detect_orphan_libraries,
    // Domain-specific validations moved to extension manifests (Phase 8).
  ]
  features   [incremental_compilation, json_and_dot_export, traceability_serialization, ci_integration]
  libraries  [specforge_emitter, specforge_watch]

  criteria [
    "specforge watch delivers diagnostics within 100ms of file change",
    "Markdown, JSON, and DOT renderers produce correct output",
    "specforge trace prints full traceability chains",
    "specforge stats reports accurate entity counts",
  ]
}

roadmap lsp_server "Phase 3: LSP Server" {
  status     completed
  behaviors  [
    go_to_definition, find_all_references, hover_information, autocomplete_entity_ids,
    rename_entity_id, live_diagnostics, code_actions_for_missing_tests, outline_view,
    workspace_symbol_search, shared_incremental_pipeline, provide_semantic_tokens,
    complete_field_names, complete_keywords, goto_import_definition,
    code_action_add_missing_import, code_action_create_entity_stub, incremental_document_sync,
  ]
  features   [go_to_definition_and_references, hover_and_autocomplete, rename_refactoring, live_diagnostics, code_actions, outline_and_symbol_search]
  libraries  [specforge_lsp]

  criteria [
    "Go-to-definition and find-references work across files",
    "Hover shows entity details and contract text",
    "Autocomplete suggests entity IDs in reference lists",
    "Field name completion suggests context-appropriate fields inside blocks",
    "Keyword completion suggests entity keywords at top level with snippet templates",
    "Rename updates all references atomically",
    "Live diagnostics appear within 100ms",
    "LSP shares incremental pipeline with watch mode",
    "Semantic tokens classify entity keywords from extensions and define blocks",
    "Go-to-definition on use imports navigates to the target file",
    "Code action adds missing use imports for cross-file references",
    "Code action creates entity stubs for unresolved references",
    "Incremental document sync applies range-based changes correctly",
  ]
}

roadmap graph_protocol_and_export "Phase 4: Graph Protocol & Agent Export" {
  status     completed
  behaviors  [
    serialize_json_graph, serialize_dot_visualization, compute_traceability_chain,
    deterministic_output, check_mode_for_ci,
    export_agent_context_format, export_agent_brief_format, query_graph_multi_resolution,
  ]
  features   [json_and_dot_export, traceability_serialization, agent_export]
  libraries  [specforge_emitter]

  criteria [
    "Graph Protocol JSON schema is stable and published",
    "specforge export --format=context produces token-optimized output for AI agents",
    "specforge export --format=graph produces complete entity graph JSON",
    "specforge export --format=brief produces minimal entity IDs + contracts",
    "Multi-resolution queries work with --scope and --hop flags",
    "Wasm extension protocol works with renderers via host functions",
    "Any AI agent framework can consume the Graph Protocol output",
  ]
}

roadmap extensions_and_coverage "Phase 5: Extensions and Coverage" {
  status     completed
  behaviors  [
    merge_coverage_reports, compute_coverage_summary, gate_on_coverage_threshold, validate_test_ids_against_spec, consume_specforge_report, compute_four_level_coverage, render_test_traceability_matrix,
    load_extension_manifests, register_extension_entity_types, load_provider_configurations, validate_provider_refs,
    remove_extension, list_installed_extensions, custom_entity_types_via_define,
    list_configured_providers,
    validate_file_reference_paths, validate_extension_testability,
    format_diagnostics_with_source_context, provide_did_you_mean_suggestions, aggregate_diagnostic_summary,
    load_wasm_module, initialize_wasm_extension, call_extension_validators,
    provide_host_function_query_graph, provide_host_function_emit_diagnostic,
    provide_host_function_add_graph_node, provide_host_function_add_graph_edge,
    provide_host_function_emit_file, provide_host_function_http_get,
    enforce_wasm_sandbox, aot_compile_wasm_module, cache_aot_artifacts, warm_wasm_engine_instance,
    validate_extension_peer_dependencies, topological_sort_extensions,
    provide_extension_query_extensions, compose_query_files_from_extensions,
    install_wasm_extension, validate_extension_manifest, handle_wasm_trap,
    invalidate_aot_cache, discover_extensions, configure_sandbox_policy, upgrade_wasm_extension,
    reject_reserved_entity_kind, detect_entity_kind_collision,
    resolve_entity_kind_conflict_via_config, qualify_entity_kind_inline,
    parse_extension_specifier, resolve_extension_source, write_lock_file, read_lock_file, verify_wasm_integrity,
    dispatch_contribution_exports, enforce_per_call_site_permissions, validate_contribution_exports,
    toggle_extension_contributions,
    generate_keyword_extension_index,
  ]
  features   [test_coverage_reporting, test_traceability, extension_management, provider_based_ref_validation, wasm_extension_runtime, extension_query_contributions, wasm_extension_installation, wasm_lock_management, wasm_extension_maintenance, entity_kind_conflict_prevention, contribution_based_extensions, extension_source_resolution, extension_version_management, extension_registry]
  libraries  [specforge_package_product, specforge_package_governance, specforge_provider_gh, specforge_coverage, specforge_wasm]

  criteria [
    "specforge coverage merges reports and gates on threshold",
    "Extension add/remove works without breaking existing spec files",
    "Provider-based ref validation catches malformed identifiers",
    "Wasm extension runtime loads, initializes, and validates extensions",
    "Host functions (query_graph, emit_diagnostic, emit_file, http_get) work correctly",
    "AOT compilation reduces cold start to <50ms per extension",
    "Sandbox enforcement blocks unauthorized filesystem and network access",
    "Peer dependency validation catches missing or incompatible extensions",
    "Custom entity types via define blocks participate in validation",
    "Extension query extensions compose with base queries for editor delivery",
    "Extension install/upgrade resolves from registry, local, and git sources",
    "Wasm traps produce structured diagnostics without crashing the compiler",
    "AOT cache self-heals on corruption and invalidates on runtime upgrade",
    "Entity kind conflicts between extensions are detected and resolved via config",
    "Contribution-based dispatch routes to correct exports per contribution type",
    "Per-call-site permissions enforce least-privilege for each export",
    "specforge.lock pins exact versions with SHA256 integrity hashes",
  ]
}

roadmap wasm_extension_authoring_phase "Phase 5b: Wasm Extension Authoring" {
  status     completed
  behaviors  [
    scaffold_wasm_extension_project, build_wasm_extension, validate_wasm_extension_locally, publish_wasm_extension,
  ]
  features   [wasm_extension_authoring]
  libraries  [specforge_wasm]

  criteria [
    "specforge extension init scaffolds a working Wasm extension project",
    "specforge extension build produces a valid .wasm binary",
    "specforge extension test runs against fixtures in production sandbox",
    "specforge extension publish uploads to npm/OCI/GitHub",
  ]
}

roadmap rust_integration "Phase 6: Rust Test Collection & Traceability" {
  status     completed
  behaviors  [
    collect_rust_test_results, parse_junit_xml, parse_libtest_json,
    resolve_entity_mapping, validate_rust_entity_ids, merge_workspace_reports,
    record_test_via_drop_guard, emit_specforge_report_from_rust,
  ]
  features   [rust_test_collection, rust_proc_macro_annotation]
  libraries  [specforge_test_lib, specforge_test_macros_lib]

  criteria [
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
  features   [code_formatting, lsp_formatting]
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

roadmap zero_entity_core_phase "Phase 8: Zero-Entity Core Architecture" {
  status     in_progress
  behaviors  [
    // Existing Phase 5 behaviors reused in Phase 8
    load_extension_manifests, register_extension_entity_types, custom_entity_types_via_define,
    validate_extension_manifest, validate_extension_peer_dependencies, topological_sort_extensions,
    reject_reserved_entity_kind, detect_entity_kind_collision,
    // Declarative Validation
    parse_validation_rule_pattern, execute_validation_pattern,
    emit_diagnostic_from_pattern, register_extension_validation_rules,
    // Extension Manifest V2
    validate_manifest_v2_schema, register_entity_kinds_from_manifest,
    register_edge_types_from_manifest, register_validation_rules_from_manifest,
    register_verify_kinds_from_manifest,
    // Dynamic Entity Registration
    boot_empty_kind_registry, boot_empty_field_registry,
    populate_kind_registry_from_extensions, populate_field_registry_from_extensions,
    populate_edge_registry_from_extensions, validate_registered_entity_fields,
    // Extension-Driven LSP and Tooling
    complete_extension_defined_keywords, provide_extension_entity_semantic_tokens,
    provide_extension_entity_hover, provide_extension_defined_lsp_icons,
    render_extension_defined_dot_shapes,
    // Zero-Entity Bootstrap
    collapse_grammar_to_generic_entity_block,
    two_phase_parse_structural, two_phase_validate_semantic,
    suggest_missing_extensions, detect_unknown_entity_kinds,
    graceful_degradation_without_extensions,
    // Extension Manifest Consistency
    validate_extension_manifest_consistency,
  ]
  features   [
    extension_management, wasm_extension_runtime, extension_query_contributions,
    declarative_validation_rules, extension_manifest, dynamic_entity_registration,
    extension_driven_lsp, zero_entity_bootstrap,
  ]
  libraries  [specforge_wasm]

  criteria [
    "Core compiler has zero hardcoded entity types — all from extensions",
    "Extension manifest v2 declares entity_kinds, edge_types, validation_rules, and testability",
    "@specforge/software + @specforge/product + @specforge/governance reproduce today's 14 domain entities (+ 2 structural keywords spec, ref = 16 total)",
    "specforge init prompts for domain extensions like Terraform prompts for providers",
    "Existing projects migrate by adding extensions to specforge.json",
    "Third-party domain extensions (atomic-design, compliance, api-design) work end-to-end",
    "Validation rules are declarative patterns interpreted by core, not hardcoded passes",
    "LSP highlights, completes, and navigates extension-defined entity types",
    "KindRegistry boots empty and is populated exclusively from extensions",
    "Two-phase compilation separates structural parsing from semantic validation",
    "E024 diagnostics suggest which extension provides unknown keywords",
    "Graceful degradation with zero extensions produces I002 info",
  ]
}

roadmap mcp_server_phase "Phase 9: MCP Server" {
  status     planned
  behaviors  [
    expose_graph_as_mcp_resource, expose_schema_as_mcp_resource,
    notify_graph_delta_via_mcp,
    provide_mcp_query_tool, provide_mcp_validate_tool, provide_mcp_export_tool,
  ]
  features   [mcp_resource_exposure, mcp_tool_integration, mcp_delta_notifications]

  criteria [
    "MCP server exposes specforge://graph and specforge://schema resources",
    "specforge.query, specforge.validate, and specforge.export tools registered",
    "Delta notifications sent to subscribed MCP clients after incremental rebuild",
    "Agents can consume graph without CLI invocation",
  ]
}

roadmap entity_embeddings_phase "Phase 10: Entity Embeddings" {
  status     future
  behaviors  []
  features   [entity_embedding_search]

  criteria [
    "Entity contracts and relationships embedded into vector space",
    "specforge search --semantic queries entities by natural language",
    "Integration with agent memory systems for persistent context",
  ]
}
