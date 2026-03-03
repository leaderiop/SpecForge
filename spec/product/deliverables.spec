// Deliverables — shippable artifacts

use product/capabilities
use product/libraries

deliverable specforge_cli_deliverable "specforge-cli" {
  type         cli
  personas     [developer, architect, ci, contributor]
  capabilities [
    initialize_a_new_spec_project, validate_spec_files, watch_for_changes, generate_documentation, trace_requirements,
    generate_code_from_spec, check_test_coverage, trace_test_coverage, migrate_spec_format, manage_plugins, view_project_statistics,
    verify_adapter_implementations_cap, review_full_traceability, visualize_spec_graph,
    format_spec_files_cap, check_formatting_in_ci_cap,
    run_spec_validation_in_ci, gate_on_coverage_in_ci, detect_code_drift_in_ci,
    author_a_custom_plugin, author_a_custom_provider, author_a_custom_generator,
    define_custom_entity_types, configure_ref_providers, export_graph_as_json,
    scaffold_wasm_plugin, test_wasm_plugin_locally_cap, publish_wasm_plugin_cap,
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
    see_live_errors_while_typing, browse_file_structure, generate_test_stubs_from_ide,
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
  capabilities [manage_plugins]
  libraries    [specforge_plugin_product]
}

deliverable specforge_governance "specforge/governance" {
  type         library
  personas     [developer, architect]
  capabilities [manage_plugins]
  libraries    [specforge_plugin_governance]
}

deliverable specforge_gh "specforge/gh" {
  type         library
  personas     [developer]
  capabilities [manage_plugins]
  libraries    [specforge_provider_gh]
}

deliverable specforge_gen_typescript_deliverable "specforge/gen-typescript" {
  type         library
  personas     [developer]
  capabilities [generate_code_from_spec, verify_adapter_implementations_cap, detect_code_drift_in_ci]
  libraries    [specforge_gen_typescript]
}

deliverable specforge_gen_rust_deliverable "specforge/gen-rust" {
  type         library
  personas     [developer, ci]
  capabilities [generate_rust_code_from_spec, collect_rust_test_results_cap, annotate_tests_with_proc_macro, detect_rust_code_drift_in_ci, gate_rust_coverage_in_ci]
  libraries    [specforge_gen_rust, specforge_test_lib, specforge_test_macros_lib]
}

deliverable specforge_wasm_runtime_deliverable "specforge-wasm" {
  type         library
  personas     [developer, contributor]
  capabilities [manage_plugins, scaffold_wasm_plugin, test_wasm_plugin_locally_cap, publish_wasm_plugin_cap]
  libraries    [specforge_wasm]
}

deliverable tree_sitter_specforge_deliverable "tree-sitter-specforge" {
  type         library
  personas     [developer, contributor]
  capabilities [validate_spec_files, get_syntax_highlighting_without_lsp]
  libraries    [tree_sitter_specforge]
}
