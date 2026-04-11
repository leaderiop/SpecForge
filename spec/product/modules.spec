// Modules — code packages that implement features

use "extensions/coverage/features"
use "features/extensions"
use "features/formatting"
use "features/incremental"
use "features/lsp"
use "features/mcp"
use "features/output"
use "features/parsing"
use "features/project-init"
use "features/validation"
use "features/wasm"
use "ports/inbound"
use "ports/outbound"

module tree_sitter_specforge "tree-sitter-specforge" {
  family      core
  description "Tree-sitter grammar and editor query files (highlights, folds, indents) for the .spec DSL"
  features    [spec_file_parsing, error_recovery_during_parsing, editor_query_files]
  tags        ["grammar", "parser"]
}

module specforge_common "specforge-common" {
  family      core
  description "Shared types, diagnostics, spans, and project discovery used across all crates"
  features    [spec_file_parsing, error_recovery_during_parsing]
  tags        ["shared", "foundation"]
}

module specforge_parser "specforge-parser" {
  family      core
  description "Wraps tree-sitter grammar into typed AST with error recovery"
  features    [spec_file_parsing, error_recovery_during_parsing]
  depends_on  [tree_sitter_specforge, specforge_common]
  tags        ["parser", "pipeline"]
}

module specforge_resolver "specforge-resolver" {
  family      core
  description "Import resolution and cross-file reference linking with cycle detection"
  features    [reference_resolution, graph_construction]
  depends_on  [specforge_parser]
  tags        ["resolver", "pipeline"]
}

module specforge_graph "specforge-graph" {
  family      core
  description "Mutable entity graph with petgraph backend supporting incremental updates"
  features    [incremental_compilation]
  depends_on  [specforge_resolver]
  tags        ["graph", "pipeline"]
}

module specforge_validator "specforge-validator" {
  family      core
  description "Structural and semantic validation with diagnostic reporting"
  features    [structural_validation, diagnostic_reporting]
  depends_on  [specforge_graph]
  tags        ["validation", "pipeline"]
}

module specforge_emitter "specforge-emitter" {
  family      core
  description "Serializes graph to JSON, DOT, and agent-optimized formats"
  features [
    json_and_dot_render,
    traceability_serialization,
    agent_export,
    self_describing_graph_protocol,
  ]
  depends_on  [specforge_graph]
  tags        ["emitter", "pipeline"]
}

module specforge_watch "specforge-watch" {
  family      core
  description "File watching and incremental rebuild pipeline with debouncing"
  features    [incremental_compilation]
  depends_on  [specforge_graph, specforge_validator]
  tags        ["watch", "incremental"]
}

module specforge_formatter "specforge-formatter" {
  family      core
  description "Idempotent code formatter with CST-preserving comment handling"
  features    [code_formatting, lsp_formatting]
  depends_on  [tree_sitter_specforge, specforge_parser]
  tags        ["formatter", "tooling"]
}

module specforge_wasm "specforge-wasm" {
  family      core
  description "Wasm/Extism extension runtime with AOT caching"
  features    [wasm_extension_runtime, wasm_extension_authoring, wasm_grammar_contributions]
  depends_on  [specforge_graph]
  tags        ["wasm", "runtime"]
}

module specforge_cli "specforge-cli" {
  family      platform
  description "CLI binary: init, check, export, format, watch, trace, stats, and extension management"
  features    [project_initialization, ci_integration]
  depends_on [
    specforge_parser,
    specforge_resolver,
    specforge_graph,
    specforge_validator,
    specforge_emitter,
    specforge_watch,
    specforge_wasm,
  ]
  tags        ["cli", "platform"]
}

module specforge_lsp "specforge-lsp" {
  family      platform
  description "LSP server: navigation, completion, diagnostics, refactoring"
  features [
    go_to_definition_and_references,
    hover_and_autocomplete,
    rename_refactoring,
    live_diagnostics,
    code_actions,
    outline_and_symbol_search,
  ]
  depends_on [
    specforge_parser,
    specforge_resolver,
    specforge_graph,
    specforge_validator,
    specforge_watch,
  ]
  tags        ["lsp", "platform"]
}

module specforge_mcp "specforge-mcp" {
  family      platform
  description "MCP server: resources, tools, notifications, prompts for AI agent integration"
  features [
    mcp_resource_exposure,
    mcp_core_tools,
    mcp_navigation_tools,
    mcp_mutation_tools,
    mcp_project_management_tools,
    mcp_delta_notifications,
    mcp_prompts,
  ]
  depends_on [
    specforge_graph,
    specforge_validator,
    specforge_emitter,
    specforge_watch,
    specforge_wasm,
  ]
  tags        ["mcp", "platform"]
}

module specforge_package_software "specforge-package-software" {
  family      extension
  description "Wasm package for @specforge/software: behavior, invariant, event, type, port entities"
  features    [extension_management, se_core_entity_kinds, se_validation_suite, se_gherkin_bridge]
  depends_on  [specforge_validator, specforge_wasm]
  tags        ["extension", "domain"]
}

module specforge_package_formal "specforge-package-formal" {
  family      extension
  description "Wasm package for @specforge/formal: condition, property, axiom, protocol, refinement, process entities"
  features    [extension_management]
  depends_on  [specforge_validator, specforge_wasm]
  tags        ["extension", "domain"]
}

module specforge_package_product "specforge-package-product" {
  family      extension
  description "Wasm package for @specforge/product: journey, deliverable, milestone, module, term, feature, persona, channel entities"
  features    [extension_management]
  depends_on  [specforge_validator, specforge_wasm]
  tags        ["extension", "domain"]
}

module specforge_package_governance "specforge-package-governance" {
  family      extension
  description "Wasm package for @specforge/governance: decision, constraint, failure_mode entities"
  features    [extension_management]
  depends_on  [specforge_validator, specforge_wasm]
  tags        ["extension", "domain"]
}

module specforge_provider_gh "specforge-provider-gh" {
  family      extension
  description "GitHub ref provider: validates gh.issue, gh.pr, gh.discussion schemes via GitHub API"
  features    [provider_based_ref_validation]
  depends_on  [specforge_validator, specforge_wasm]
  tags        ["extension", "provider"]
}

module specforge_coverage "specforge-coverage" {
  family      core
  description "Test coverage reporting and traceability proof ingestion"
  features    [test_coverage_reporting]
  depends_on  [specforge_graph]
  tags        ["coverage", "traceability"]
}

module specforge_collect_rust "specforge-collect-rust" {
  family      extension
  description "Rust test collector: parses JUnit XML and maps tests to entity IDs via specforge-report.json"
  features    [rust_test_collection]
  depends_on  [specforge_graph]
  tags        ["extension", "traceability"]
}

module specforge_test_lib "specforge-test" {
  family      extension
  description "Rust test library: drop guard reporter and atexit handler for specforge-test proc macro"
  features    [rust_proc_macro_annotation]
  tags        ["extension", "traceability"]
}

module specforge_test_macros_lib "specforge-test-macros" {
  family      extension
  description "Rust proc macro crate: #[specforge::test(\"entity_id\")] attribute for test-to-entity mapping"
  features    [rust_proc_macro_annotation]
  depends_on  [specforge_test_lib]
  tags        ["extension", "traceability"]
}
