// Deliverables — shippable artifacts

use "product/journeys"
use "product/modules"

deliverable specforge_cli_deliverable "specforge-cli" {
  artifact_type cli
  description   "The primary CLI binary for SpecForge. Parses, validates, exports, formats, and manages spec files."
  modules       [specforge_cli]
  journeys      [
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
  artifact_type service
  description   "The LSP server binary for IDE integration. Provides diagnostics, navigation, completions, and formatting."
  modules       [specforge_lsp]
  journeys      [
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
  artifact_type service
  description   "The MCP server for AI agent integration. Exposes graph queries, tools, resources, and prompts via JSON-RPC over stdio."
  modules       [specforge_mcp]
  journeys      [
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
  artifact_type library
  description   "Core compiler libraries: parser, resolver, graph, validator, emitter, watch."
  journeys      [
    validate_spec_files,
    watch_for_changes,
    export_graph_formats,
    trace_requirements,
    j_format_spec_files,
  ]
  modules       [
    specforge_common,
    specforge_parser,
    specforge_resolver,
    specforge_graph,
    specforge_validator,
    specforge_emitter,
    specforge_watch,
    specforge_formatter,
  ]
}

deliverable specforge_product "specforge/product" {
  artifact_type extension
  description   "The @specforge/product Wasm extension package providing 9 product entity kinds."
  modules       [specforge_package_product]
  journeys      [review_deliverable_scope, review_milestone_progress, review_persona_channel_landscape]
}

deliverable specforge_governance "specforge/governance" {
  artifact_type extension
  description   "The @specforge/governance Wasm extension package providing decision, constraint, and failure_mode kinds."
  journeys      [install_domain_extensions]
  modules       [specforge_package_governance]
}

deliverable specforge_software "specforge/software" {
  artifact_type extension
  description   "The @specforge/software Wasm extension package providing behavior, invariant, event, type, and port kinds."
  journeys      [install_domain_extensions, trace_requirements]
  modules       [specforge_package_software]
}

deliverable specforge_formal "specforge/formal" {
  artifact_type extension
  description   "The @specforge/formal Wasm extension package providing property, axiom, protocol, refinement, and process kinds."
  journeys      [install_domain_extensions]
  modules       [specforge_package_formal]
}

deliverable specforge_gh "specforge/gh" {
  artifact_type extension
  description   "The @specforge/gh provider extension for GitHub reference validation."
  journeys      [configure_ref_providers]
  modules       [specforge_provider_gh]
}

deliverable specforge_rust_traceability_deliverable "specforge/rust-traceability" {
  artifact_type library
  description   "Rust test traceability toolkit: the #[specforge_test] attribute, test guard, and the @specforge/cargo-test collector."
  modules       [specforge_test_lib, specforge_test_macros_lib, specforge_package_cargo_test]
  journeys      [j_collect_rust_test_results, annotate_tests_with_proc_macro]
}

deliverable specforge_wasm_runtime_deliverable "specforge-wasm" {
  artifact_type library
  description   "The Wasm extension runtime: loading, sandboxing, host functions, and compile caching."
  modules       [specforge_wasm]
  journeys      [
    author_a_domain_extension,
    author_a_custom_provider,
    scaffold_wasm_extension,
    diagnose_extension_issues,
    j_publish_wasm_extension,
    test_wasm_extension_locally,
  ]
}

deliverable tree_sitter_specforge_deliverable "tree-sitter-specforge" {
  artifact_type library
  description   "Tree-sitter grammar for .spec files with editor query files."
  journeys      [get_syntax_highlighting_without_lsp]
  modules       [tree_sitter_specforge]
}
