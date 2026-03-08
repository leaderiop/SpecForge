// Roadmap — delivery phases
//
// Rewritten from scratch with clear priority ordering based on:
// 1. Entity dependency DAG — foundational entities before dependent ones
// 2. Compilation pipeline order — parse > resolve > graph > validate > export
// 3. Vision horizons — H1 (individual value) > H2 (ecosystem) > H3 (standard)
// 4. Each phase ships something usable — no setup-only phases
//
// Phase dependencies:
// P1 > P2 > P3 > P4 > P5
//                P4 > P6 > P7
//                P4 > P8
//        ── H1/H2 boundary ──
//                P8 > P9 > P10 > P11
//                P4 > P12 > P13
//                P11 > P14

use behaviors/parsing
use behaviors/resolution
use behaviors/graph
use behaviors/validation
use behaviors/error-reporting
use behaviors/init
use behaviors/output
use behaviors/output-schema
use behaviors/incremental
use behaviors/lsp
use behaviors/formatting
use behaviors/extensions
use behaviors/wasm-extensions
use behaviors/wasm-host-functions
use behaviors/wasm-lifecycle
use behaviors/wasm-sandbox
use behaviors/wasm-authoring
use behaviors/mcp-server
use behaviors/mcp-tools
use behaviors/mcp-operations
use behaviors/mcp-prompts
use behaviors/migration
use behaviors/zero-entity-registries
use behaviors/zero-entity-validation
use behaviors/zero-entity-lsp
use behaviors/surface-contributions
use features/parsing
use features/validation
use features/output
use features/project-init
use features/incremental
use features/lsp
use features/formatting
use features/zero-entity-core
use features/extensions
use features/wasm
use features/mcp
use features/migration
use product/libraries

// ════════════════════════════════════════════════════════════════
// H1: Individual Value — install to first validated output in <60s
// ════════════════════════════════════════════════════════════════

roadmap structural_parsing "Phase 1: Structural Parsing" {
  status     done
  behaviors  [
    // 1.1 Core Parsing
    parse_spec_file_to_ast, recover_from_syntax_errors, parse_use_imports,
    parse_all_block_types, parse_triple_quoted_strings,
    // 1.2 Structural Constructs
    parse_verify_statements, parse_ref_blocks, parse_define_blocks,
    // 1.3 Editor Queries
    provide_syntax_highlighting_queries, provide_code_folding_queries,
    provide_indentation_queries,
  ]
  features   [spec_file_parsing, error_recovery_during_parsing, editor_query_files]
  libraries  [tree_sitter_specforge, specforge_parser]

  criteria [
    "Tree-sitter grammar parses any keyword name { fields } block",
    "Multi-error recovery: N syntax errors produce N diagnostics, not 1",
    "Editor query files (highlights.scm, folds.scm, indents.scm) work in Neovim/VS Code",
    "Generic entity_block rule produces clean AST nodes for any keyword",
  ]
}

roadmap resolution_and_graph "Phase 2: Resolution & Graph Construction" {
  status     in_progress
  behaviors  [
    // 2.1 Import Resolution
    resolve_use_imports, detect_import_cycles, resolve_external_ref_declarations,
    // 2.2 Reference Linking
    link_entity_references, resolve_soft_cross_extension_references,
    // 2.3 Graph Building
    build_in_memory_graph, maintain_mutable_graph, compute_subgraph_for_invalidation,
  ]
  features   [reference_resolution, graph_construction]
  libraries  [specforge_resolver, specforge_graph]

  criteria [
    "All intra-project references linked across files",
    "Import cycles detected and reported as E003",
    "Cross-extension refs produce I004 info if extension not installed",
    "Graph has one node per entity, one edge per reference",
    "Mutable graph supports incremental updates",
  ]
}

roadmap validation_and_errors "Phase 3: Validation & Error Reporting" {
  status     planned
  behaviors  [
    // 3.1 Structural Validation
    detect_dangling_references, detect_duplicate_entity_ids,
    detect_orphan_refs, validate_file_reference_paths,
    // 3.2 Error Reporting
    format_diagnostics_with_source_context,
    provide_did_you_mean_suggestions, aggregate_diagnostic_summary,
    // 3.3 CI Integration
    print_diagnostics_structured, exit_code_reflects_diagnostic_severity,
    check_mode_for_ci,
  ]
  features   [structural_validation, diagnostic_reporting, ci_integration]
  libraries  [specforge_validator, specforge_cli]

  criteria [
    "specforge check passes on SpecForge's own .spec files",
    "Diagnostics include source context with line/column spans",
    "Did-you-mean suggestions for misspelled entity IDs (Levenshtein <= 2)",
    "Exit code 0 on clean, 1 on errors; --strict promotes warnings to errors",
    "Structured output (JSON) available for CI parsers",
  ]
}

roadmap output_and_export "Phase 4: Output & Agent Export" {
  status     planned
  behaviors  [
    // 4.1 Core Serialization
    serialize_json_graph, serialize_dot_visualization,
    deterministic_output, export_diagnostics_as_json,
    // 4.2 Agent Formats
    export_agent_context_format, export_agent_brief_format,
    export_agent_graph_format, enforce_token_budget,
    // 4.3 Traceability & Query
    compute_traceability_chain, serialize_traceability_data,
    validate_agent_plan, query_graph_multi_resolution,
    // 4.4 Statistics
    compute_project_statistics,
  ]
  features   [json_and_dot_render, traceability_serialization, agent_export]
  libraries  [specforge_emitter]

  criteria [
    "Graph Protocol JSON schema published and stable",
    "specforge export --format=context produces token-optimized output",
    "specforge export --format=graph produces complete entity graph JSON",
    "specforge export --format=brief produces minimal IDs + contracts",
    "Multi-resolution queries work with --scope and --hop flags",
    "specforge trace prints full traceability chains",
    "specforge stats reports accurate entity/edge/orphan counts",
    "Output is deterministic: same input always produces same bytes",
  ]
}

roadmap project_init "Phase 5: Project Initialization" {
  status     planned
  behaviors  [
    // 5.1 Scaffolding
    scaffold_new_project, scaffold_starter_spec_file, find_project_root,
    // 5.2 Extension Selection
    interactive_extension_selection, non_interactive_init,
    // 5.3 Zero-Extension Support
    graceful_zero_extension_init, add_extension_to_existing_project,
  ]
  features   [project_initialization]
  libraries  [specforge_cli]

  criteria [
    "specforge init creates specforge.json and starter .spec file",
    "Full init > check > export pipeline completes in under 60 seconds",
    "Zero-extension project is valid and produces a graph",
    "Non-interactive mode works for CI: --name and --extensions flags",
    "specforge add @specforge/software adds extension to existing project",
  ]
}

roadmap incremental_compilation "Phase 6: Incremental Compilation" {
  status     planned
  behaviors  [
    // 6.1 File Watching
    watch_file_system_for_changes, debounce_file_changes,
    track_import_dag_incrementally,
    // 6.2 Incremental Rebuild
    invalidate_changed_files, rebuild_affected_subgraph,
    validate_delta_correctness,
    // 6.3 Delta & Notification
    compute_graph_delta, dispatch_incremental_validators,
    emit_incremental_diagnostics, notify_delta_subscribers,
  ]
  features   [incremental_compilation, incremental_graph_deltas]
  libraries  [specforge_watch]

  criteria [
    "specforge watch delivers diagnostics within 100ms of file change",
    "Incremental rebuild matches cold rebuild (validated by property tests)",
    "Graph delta contains only added/removed/modified nodes and edges",
    "File change debouncing prevents redundant rebuilds",
    "Import DAG tracked incrementally for minimal invalidation",
  ]
}

roadmap lsp_server "Phase 7: LSP Server" {
  status     planned
  behaviors  [
    // 7.1 Lifecycle
    lsp_initialize, lsp_shutdown, document_open_close,
    handle_text_document_change,
    // 7.2 Navigation
    go_to_definition, find_all_references, hover_information,
    goto_import_definition, incremental_document_sync,
    // 7.3 Intelligence
    autocomplete_entity_ids, complete_field_names, complete_keywords,
    provide_semantic_tokens, shared_incremental_pipeline,
    // 7.4 Refactoring
    prepare_rename, rename_entity_id, code_actions_for_missing_verify,
    code_action_add_missing_import, code_action_create_entity_stub,
    // 7.5 Views
    outline_view, workspace_symbol_search, live_diagnostics,
  ]
  features   [lsp_lifecycle, go_to_definition_and_references, hover_and_autocomplete, rename_refactoring, live_diagnostics, semantic_tokens, code_actions, outline_and_symbol_search]
  libraries  [specforge_lsp]

  criteria [
    "Go-to-definition and find-references work across files",
    "Hover shows entity details, contract text, and reference count",
    "Autocomplete suggests entity IDs, field names, and keywords",
    "Rename updates declaration and all references atomically",
    "Live diagnostics appear within 100ms via shared incremental pipeline",
    "Semantic tokens classify entity keywords from extensions and define blocks",
    "Code actions: add missing import, create entity stub, add verify",
    "Outline view and workspace symbol search work for all entity types",
  ]
}

roadmap code_formatting "Phase 8: Code Formatting" {
  status     planned
  behaviors  [
    // 8.1 Core Formatting
    format_spec_files, preserve_comments, apply_format_rules,
    maintain_format_idempotency, format_with_parse_errors,
    // 8.2 CLI Modes
    check_formatting, show_formatting_diff, format_from_stdin,
    discover_format_targets,
    // 8.3 Configuration
    load_format_config,
    // 8.4 LSP Formatting
    lsp_format_document, lsp_format_range, lsp_respect_editor_config,
  ]
  features   [code_formatting, lsp_formatting]
  libraries  [specforge_formatter]

  criteria [
    "format(format(x)) == format(x) verified by property tests",
    "All comments preserved after formatting",
    "Single file formatted in under 50ms",
    "specforge format --check exits 1 on unformatted files",
    "LSP textDocument/formatting produces same result as CLI",
    "Range formatting matches full formatting for affected blocks",
    "Files with parse errors are partially formatted without data loss",
  ]
}

// ════════════════════════════════════════════════════════════════
// H2: Ecosystem — zero domain knowledge in core, extensions for all
// ════════════════════════════════════════════════════════════════

roadmap zero_entity_core "Phase 9: Zero-Entity Core Architecture" {
  status     planned
  behaviors  [
    // 9.1 Empty Registries
    boot_empty_kind_registry, boot_empty_field_registry,
    boot_empty_edge_registry,
    // 9.2 Manifest V2
    validate_manifest_v2_schema, validate_extension_manifest_consistency,
    // 9.3 Registry Population
    register_entity_kinds_from_manifest, register_edge_types_from_manifest,
    register_validation_rules_from_manifest, register_verify_kinds_from_manifest,
    populate_kind_registry_from_extensions, populate_field_registry_from_extensions,
    populate_edge_registry_from_extensions,
    // 9.4 Post-Registration Validation
    detect_unknown_entity_kinds, validate_registered_entity_fields,
    detect_unknown_entity_fields, detect_duplicate_entity_kinds,
    validate_peer_dependencies,
    // 9.5 Declarative Validation Engine
    parse_validation_rule_pattern, execute_validation_pattern,
    emit_diagnostic_from_pattern, register_extension_validation_rules,
    register_custom_validation_patterns, validate_extension_testability,
    // 9.6 Two-Phase Compilation
    two_phase_parse_structural, two_phase_validate_semantic,
    collapse_grammar_to_generic_entity_block, suggest_missing_extensions,
    graceful_degradation_without_extensions, handle_all_extensions_failed_to_load,
    custom_entity_types_via_define,
    delegate_body_parsing_to_extension,
    // 9.7 Extension-Driven Visualization
    render_extension_defined_dot_shapes, render_extension_defined_edge_styles,
    // 9.8 Extension-Driven LSP
    complete_extension_defined_keywords, provide_extension_entity_semantic_tokens,
    provide_extension_entity_hover, provide_extension_defined_lsp_icons,
    // 9.9 Entity Enhancement
    register_entity_enhancements, detect_enhancement_conflicts,
    resolve_enhancement_conflicts,
    // 9.10 Grammar & Body Parser Registration
    register_grammar_contributions, register_body_parser_contributions,
  ]
  features   [declarative_validation_rules, extension_manifest, dynamic_entity_registration, extension_driven_lsp, extension_driven_visualization, zero_entity_bootstrap, zero_entity_validation, entity_enhancement]
  libraries  [specforge_wasm]

  criteria [
    "Core compiler has zero hardcoded entity types — all from extensions",
    "KindRegistry boots empty and is populated exclusively from manifests",
    "Extension manifest v2 declares entity_kinds, edge_types, validation_rules, testability",
    "@specforge/software + product + governance reproduce today's 14 domain entities",
    "Two-phase compilation separates structural parsing from semantic validation",
    "E024 diagnostics suggest which extension provides unknown keywords",
    "Graceful degradation with zero extensions produces I002 info",
    "Declarative validation patterns interpreted by core, not hardcoded passes",
    "LSP highlights, completes, and navigates extension-defined entity types",
    "Entity enhancements from multiple extensions compose without conflicts",
    "Third-party domain extensions work end-to-end",
  ]
}

roadmap wasm_runtime "Phase 10: Wasm Extension Runtime" {
  status     planned
  behaviors  [
    // 10.1 Loading & Lifecycle
    load_extension_manifests, register_extension_entity_types,
    load_extension_manifest, load_wasm_module, initialize_wasm_extension,
    validate_extension_manifest,
    // 10.2 Dependencies
    validate_extension_peer_dependencies, topological_sort_extensions,
    call_extension_validators,
    // 10.3 Host Functions
    compute_extension_query_scope, provide_host_function_query_graph,
    provide_host_function_emit_diagnostic, provide_host_function_add_graph_node,
    provide_host_function_add_graph_edge, provide_host_function_read_file,
    provide_host_function_emit_file, provide_host_function_http_get,
    // 10.4 Sandbox & AOT
    enforce_wasm_sandbox, aot_compile_wasm_module, cache_aot_artifacts,
    warm_wasm_engine_instance, evict_warm_engine_instance,
    configure_sandbox_policy,
    // 10.5 Error Recovery
    handle_wasm_trap, invalidate_aot_cache,
    // 10.6 Conflict Prevention
    reject_reserved_entity_kind, detect_entity_kind_collision,
    // 10.7 Provider System
    load_provider_configurations, register_provider_schemes,
    validate_provider_refs, validate_ref_target_format, validate_provider_kinds,
    // 10.8 Contributions
    dispatch_contribution_exports, enforce_per_call_site_permissions,
    validate_contribution_exports, toggle_extension_contributions,
    provide_extension_query_extensions, compose_query_files_from_extensions,
    // 10.9 Surface Contributions — Registration & Validation
    register_surface_contributions, validate_surface_exports,
    validate_mcp_tool_schemas, validate_command_arg_types,
    auto_promote_commands_to_mcp_tools,
  ]
  features   [wasm_extension_runtime, wasm_host_function_api, wasm_performance_optimization, entity_kind_conflict_prevention, provider_based_ref_validation, contribution_based_extensions, extension_query_contributions, surface_contributions]
  libraries  [specforge_wasm]

  criteria [
    "Wasm extensions load, initialize, and validate without errors",
    "All 8 host functions work correctly (query, diagnostic, node, edge, file, http)",
    "AOT compilation reduces cold start to <50ms per extension",
    "Sandbox enforcement blocks unauthorized filesystem and network access",
    "Peer dependency validation catches missing or incompatible extensions",
    "Wasm traps produce structured diagnostics without crashing the compiler",
    "Entity kind conflicts between extensions detected and reported",
    "Provider-based ref validation catches malformed identifiers",
    "Contribution-based dispatch routes to correct exports per contribution type",
    "Per-call-site permissions enforce least-privilege for each export",
    "Surface contributions registered from manifest surfaces field",
    "CLI commands auto-promoted to MCP tools with matching schemas",
  ]
}

roadmap extension_ecosystem "Phase 11: Extension Ecosystem" {
  status     planned
  behaviors  [
    // 11.1 Lifecycle
    install_wasm_extension, upgrade_wasm_extension, uninstall_wasm_extension,
    // 11.2 CLI Commands
    remove_extension, list_installed_extensions, list_configured_providers,
    // 11.3 Source & Lock
    parse_extension_specifier, resolve_extension_source,
    write_lock_file, read_lock_file, verify_wasm_integrity,
    // 11.4 Registry
    configure_registries, resolve_registry_source, search_registry,
    publish_to_registry, verify_registry_integrity,
    authenticate_registry_request, retry_registry_request,
    validate_registry_credentials, logout_registry, support_private_registries,
    // 11.5 Authoring
    scaffold_wasm_extension_project, build_wasm_extension,
    validate_wasm_extension_locally, publish_wasm_extension,
    // 11.6 Collectors
    register_collector_contributions, auto_detect_collector,
    dispatch_collector, validate_collector_output, ingest_collector_report,
    // 11.7 Grammar Contributions
    load_extension_grammar, validate_grammar_wasm,
    compose_grammar_injections, dispatch_body_parser,
    cache_grammar_artifacts,
    // 11.8 Updates & Doctor
    discover_extensions, update_all_extensions, refresh_lock_file,
    run_doctor_check, generate_keyword_extension_index,
    // 11.9 LSP Grammar Integration
    load_extension_grammars_for_highlighting,
    // 11.10 Surface Contributions — Dispatch & Sandbox
    dispatch_surface_command, dispatch_surface_mcp_tool,
    dispatch_surface_mcp_resource, enforce_surface_sandbox,
    toggle_surface_contributions,
  ]
  features   [extension_management, wasm_extension_installation, wasm_lock_management, wasm_extension_maintenance, wasm_extension_authoring, extension_registry, registry_authentication, test_result_collection, wasm_grammar_contributions, extension_body_parsing]

  criteria [
    "Full install/upgrade/remove lifecycle for Wasm extensions",
    "specforge.lock pins exact versions with SHA256 integrity hashes",
    "Extension authoring: init > build > test > publish works e2e",
    "Collectors produce specforge-report.json from test frameworks",
    "Registry search/publish works with npm, OCI, and GitHub sources",
    "Grammar contributions load and body parsers produce structured fields",
    "specforge doctor reports conflicts, cache health, and extension status",
    "Private registry authentication with token refresh and retry",
    "LSP loads extension grammars for syntax highlighting",
    "Surface commands dispatched via cmd__{id} Wasm exports with sandbox enforcement",
    "Surface MCP tools/resources dispatched via mcp__{name} Wasm exports",
  ]
}

roadmap schema_versioning "Phase 12: Graph Protocol Schema Versioning" {
  status     planned
  behaviors  [
    // 12.1 Schema Lifecycle
    generate_schema_from_registries, embed_schema_in_export,
    persist_schema_cache, serve_schema_resource, serve_graph_resource,
    // 12.2 Versioning
    compute_schema_version, detect_breaking_schema_changes,
    negotiate_schema_version, publish_schema_specification,
  ]
  features   [self_describing_graph_protocol, graph_protocol_versioning]
  libraries  [specforge_emitter]

  criteria [
    "Self-describing schema embedded in every graph export",
    "Schema version auto-computed from registry contents",
    "Breaking changes detected when entity kinds or edge types are removed",
    "Schema negotiation allows consumers to request specific versions",
    "JSON Schema specification published alongside graph exports",
  ]
}

roadmap mcp_server "Phase 13: MCP Server" {
  status     planned
  behaviors  [
    // 13.1 Lifecycle
    mcp_initialize, mcp_shutdown, list_mcp_resources, list_mcp_tools,
    list_mcp_prompts, handle_mcp_protocol_error,
    handle_mcp_request_cancellation, guard_mcp_reinitialization,
    // 13.2 Resources
    expose_graph_as_mcp_resource, expose_schema_as_mcp_resource,
    expose_context_as_mcp_resource, expose_brief_as_mcp_resource,
    expose_diagnostics_as_mcp_resource, expose_entity_as_mcp_resource,
    // 13.3 Core Tools
    provide_mcp_query_tool, provide_mcp_validate_tool,
    provide_mcp_export_tool, provide_mcp_trace_tool,
    provide_mcp_search_tool, provide_mcp_schema_tool,
    provide_mcp_coverage_tool, provide_mcp_stats_tool,
    // 13.4 Navigation Tools
    provide_mcp_inspect_tool, provide_mcp_find_definition_tool,
    provide_mcp_find_references_tool, provide_mcp_outline_tool,
    provide_mcp_suggest_fixes_tool,
    // 13.5 Mutation Tools
    provide_mcp_format_tool, provide_mcp_rename_tool,
    provide_mcp_init_tool, provide_mcp_add_extension_tool,
    provide_mcp_remove_extension_tool, provide_mcp_migrate_tool,
    // 13.6 Project Tools
    provide_mcp_extensions_tool, provide_mcp_providers_tool,
    provide_mcp_doctor_tool, provide_mcp_collect_tool,
    provide_mcp_render_tool,
    // 13.7 Notifications
    notify_graph_delta_via_mcp, notify_diagnostics_delta_via_mcp,
    // 13.8 Prompts
    provide_mcp_context_prompt, provide_mcp_review_prompt,
    provide_mcp_trace_prompt, provide_mcp_explore_prompt,
  ]
  features   [mcp_lifecycle, mcp_resource_exposure, mcp_core_tools, mcp_navigation_tools, mcp_mutation_tools, mcp_project_management_tools, mcp_delta_notifications, mcp_prompts, mcp_protocol_compliance, mcp_discovery]

  criteria [
    "MCP server initializes and shuts down cleanly per protocol spec",
    "All 6 resources registered and return current graph state",
    "All 13 core+navigation tools respond with correct results",
    "All 11 mutation+project tools execute operations successfully",
    "Delta notifications sent to subscribed clients after incremental rebuild",
    "All 4 prompts return pre-composed context-rich workflows",
    "Protocol errors produce JSON-RPC error responses, not crashes",
    "Agents consume graph without CLI invocation",
  ]
}

roadmap migration "Phase 14: Migration" {
  status     planned
  behaviors  [
    // 14.1 Detection
    detect_format_version_mismatch, generate_migration_diff,
    // 14.2 Execution
    migrate_spec_files_in_place, rollback_failed_migration,
    // 14.3 Post-Migration
    validate_post_migration_integrity,
    capture_pre_migration_schema_snapshot,
    verify_graph_protocol_compatibility_after_migration,
    // 14.4 Extension Hooks
    invoke_extension_migration_hooks,
  ]
  features   [spec_file_migration]

  criteria [
    "specforge migrate --dry-run shows unified diff of all proposed changes",
    "Backup created before in-place transformation",
    "Post-migration validation confirms graph structural equivalence",
    "Rollback restores original files on migration failure",
    "Extension migration hooks invoked in topological order",
  ]
}

