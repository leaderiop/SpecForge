// Deliverables — shippable artifacts

use product/capabilities
use product/libraries

deliverable specforge_cli_deliverable "specforge-cli" {
  type         cli
  personas     [developer, architect, ci, contributor]
  capabilities [
    initialize_a_new_spec_project, validate_spec_files, watch_for_changes, generate_documentation, trace_requirements,
    check_test_coverage, trace_test_coverage, migrate_spec_files_cap, manage_extensions, view_project_statistics,
    review_full_traceability, visualize_spec_graph,
    format_spec_files_cap, check_formatting_in_ci_cap,
    run_spec_validation_in_ci, gate_on_coverage_in_ci,
    author_a_domain_extension, author_a_custom_provider,
    define_custom_entity_types, configure_ref_providers, export_graph_as_json,
    scaffold_wasm_extension, validate_wasm_extension_locally_cap, publish_wasm_extension_cap,
  ]
  libraries    [
    specforge_parser, specforge_resolver, specforge_graph, specforge_validator,
    specforge_emitter, specforge_watch, specforge_cli, specforge_coverage,
    specforge_formatter, specforge_wasm,
  ]
}

deliverable specforge_lsp_deliverable "specforge-lsp" {
  type         service
  personas     [developer]
  capabilities [
    navigate_to_entity_definitions, explore_entity_references, get_inline_help, rename_entities_safely,
    see_live_errors_while_typing, browse_file_structure, suggest_test_declarations_from_ide,
    format_on_save_cap,
  ]
  libraries    [
    specforge_parser, specforge_resolver, specforge_graph, specforge_validator,
    specforge_watch, specforge_lsp, specforge_formatter,
  ]
}

deliverable specforge_core "specforge/core" {
  type         library
  personas     [developer, contributor]
  capabilities [validate_spec_files, watch_for_changes, generate_documentation, trace_requirements]
  libraries    [specforge_parser, specforge_resolver, specforge_graph, specforge_validator, specforge_emitter, specforge_watch]
}

deliverable specforge_product "specforge/product" {
  type         library
  personas     [developer]
  capabilities [manage_extensions]
  libraries    [specforge_package_product]
}

deliverable specforge_governance "specforge/governance" {
  type         library
  personas     [developer, architect]
  capabilities [manage_extensions]
  libraries    [specforge_package_governance]
}

deliverable specforge_gh "specforge/gh" {
  type         library
  personas     [developer]
  capabilities [manage_extensions]
  libraries    [specforge_provider_gh]
}

deliverable specforge_rust_traceability_deliverable "specforge/rust-traceability" {
  type         library
  personas     [developer, ci]
  capabilities [collect_rust_test_results_cap, annotate_tests_with_proc_macro]
  libraries    [specforge_collect_rust, specforge_test_lib, specforge_test_macros_lib]
}

deliverable specforge_wasm_runtime_deliverable "specforge-wasm" {
  type         library
  personas     [developer, contributor]
  capabilities [manage_extensions, scaffold_wasm_extension, validate_wasm_extension_locally_cap, publish_wasm_extension_cap, diagnose_extension_issues]
  libraries    [specforge_wasm]
}

deliverable tree_sitter_specforge_deliverable "tree-sitter-specforge" {
  type         library
  personas     [developer, contributor]
  capabilities [validate_spec_files, get_syntax_highlighting_without_lsp]
  libraries    [tree_sitter_specforge]
}
