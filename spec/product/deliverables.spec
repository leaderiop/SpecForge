// Deliverables — shippable artifacts

use "product/journeys"

deliverable specforge_cli_deliverable "specforge-cli" {
  description "The primary CLI binary for SpecForge. Parses, validates, exports, formats, and manages spec files."
  journeys [
    initialize_a_new_spec_project,
    initialize_project_non_interactively,
    initialize_project_as_agent,
    validate_spec_files,
    watch_for_changes,
    generate_documentation,
    trace_requirements,
    check_test_coverage,
    trace_test_coverage,
    run_spec_validation_in_ci,
    gate_on_coverage_in_ci,
    export_graph_as_json,
    export_graph_for_agents,
    export_graph_formats,
    export_scoped_context_for_agent,
    view_project_statistics,
    review_full_traceability,
    visualize_spec_graph,
    define_custom_entity_types,
    configure_ref_providers,
    manage_extensions,
    install_domain_extensions,
    migrate_spec_files,
    review_term_glossary,
    j_format_spec_files,
    check_formatting_in_ci,
    validate_graph_in_ci,
  ]
}

deliverable specforge_lsp_deliverable "specforge-lsp" {
  description "The LSP server binary for IDE integration. Provides diagnostics, navigation, completions, and formatting."
  journeys [
    see_live_errors_while_typing,
    navigate_to_entity_definitions,
    explore_entity_references,
    get_inline_help,
    rename_entities_safely,
    browse_file_structure,
    suggest_test_declarations_from_ide,
    format_on_save,
    get_syntax_highlighting_without_lsp,
  ]
}

deliverable specforge_mcp_deliverable "specforge-mcp" {
  description "The MCP server for AI agent integration. Exposes graph queries, tools, resources, and prompts via JSON-RPC over stdio."
  journeys [
    consume_graph_via_mcp,
    navigate_spec_graph_via_mcp,
    manage_spec_project_via_mcp,
    mutate_spec_project_via_mcp,
    receive_delta_notifications_via_mcp,
    use_guided_prompts_via_mcp,
    j_query_graph_multi_resolution,
    j_validate_agent_plan,
  ]
}

deliverable specforge_core "specforge/core" {
  description "Core compiler libraries: parser, resolver, graph, validator, emitter, watch."
}

deliverable specforge_product "specforge/product" {
  description "The @specforge/product Wasm extension package providing 9 product entity kinds."
  journeys [
    review_deliverable_scope,
    review_milestone_progress,
    review_persona_channel_landscape,
  ]
}

deliverable specforge_governance "specforge/governance" {
  description "The @specforge/governance Wasm extension package providing decision, constraint, and failure_mode kinds."
}

deliverable specforge_software "specforge/software" {
  description "The @specforge/software Wasm extension package providing behavior, invariant, event, type, and port kinds."
}

deliverable specforge_formal "specforge/formal" {
  description "The @specforge/formal Wasm extension package providing condition, property, axiom, protocol, refinement, and process kinds."
}

deliverable specforge_gh "specforge/gh" {
  description "The @specforge/gh provider extension for GitHub reference validation."
}

deliverable specforge_rust_traceability_deliverable "specforge/rust-traceability" {
  description "Rust test traceability toolkit: proc macro, test guard, JUnit XML collector."
  journeys [
    j_collect_rust_test_results,
    annotate_tests_with_proc_macro,
  ]
}

deliverable specforge_wasm_runtime_deliverable "specforge-wasm" {
  description "The Wasm extension runtime: loading, sandboxing, host functions, and AOT caching."
  journeys [
    author_a_domain_extension,
    author_a_custom_provider,
    scaffold_wasm_extension,
    diagnose_extension_issues,
    author_custom_grammar_extension,
    j_publish_wasm_extension,
    test_wasm_extension_locally,
  ]
}

deliverable tree_sitter_specforge_deliverable "tree-sitter-specforge" {
  description "Tree-sitter grammar for .spec files with editor query files."
}
