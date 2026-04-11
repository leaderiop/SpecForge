// Libraries — code crates that implement features

use "features/parsing"
use "features/validation"
use "features/incremental"
use "features/output"
use "extensions/coverage/features"
use "extensions/rust/features"
use "features/lsp"
use "features/extensions"
use "features/wasm"
use "features/project-init"
use "features/formatting"
use "ports/outbound"
use "ports/inbound"
library tree_sitter_specforge "tree-sitter-specforge" {
  family       core
  features     [spec_file_parsing, error_recovery_during_parsing, editor_query_files]
}

library specforge_parser "specforge-parser" {
  family       core
  features     [spec_file_parsing, error_recovery_during_parsing]
  depends_on   [tree_sitter_specforge]
  ports_defined [SourceParser]
}

library specforge_resolver "specforge-resolver" {
  family       core
  features     [reference_resolution, graph_construction]
  depends_on   [specforge_parser]
}

library specforge_graph "specforge-graph" {
  family       core
  features     [incremental_compilation]
  depends_on   [specforge_resolver]
}

library specforge_validator "specforge-validator" {
  family       core
  features     [structural_validation, diagnostic_reporting]
  depends_on   [specforge_graph]
  ports_defined [RefValidator]
}

library specforge_emitter "specforge-emitter" {
  family       core
  features     [json_and_dot_render, traceability_serialization, agent_export, self_describing_graph_protocol]
  depends_on   [specforge_graph]
  ports_defined [GraphSerializer]
}

library specforge_watch "specforge-watch" {
  family       core
  features     [incremental_compilation]
  depends_on   [specforge_graph, specforge_validator]
  ports_defined [FileSystem]
}

library specforge_formatter "specforge-formatter" {
  family       core
  features     [code_formatting, lsp_formatting]
  depends_on   [tree_sitter_specforge, specforge_parser]
}

library specforge_wasm "specforge-wasm" {
  family       core
  features     [wasm_extension_runtime, wasm_extension_authoring, wasm_grammar_contributions]
  depends_on   [specforge_graph]
  ports_defined [WasmRuntime]
}

library specforge_cli "specforge-cli" {
  family       platform
  features     [project_initialization, ci_integration]
  depends_on   [specforge_parser, specforge_resolver, specforge_graph, specforge_validator, specforge_emitter, specforge_watch, specforge_wasm]
  ports_defined [CompilerApi]
}

library specforge_lsp "specforge-lsp" {
  family       platform
  features     [go_to_definition_and_references, hover_and_autocomplete, rename_refactoring, live_diagnostics, code_actions, outline_and_symbol_search]
  depends_on   [specforge_parser, specforge_resolver, specforge_graph, specforge_validator, specforge_watch]
  ports_defined [LspProtocol]
}

library specforge_package_product "specforge-package-product" {
  family       extension
  features     [extension_management]
  depends_on   [specforge_validator, specforge_wasm]
}

library specforge_package_governance "specforge-package-governance" {
  family       extension
  features     [extension_management]
  depends_on   [specforge_validator, specforge_wasm]
}

library specforge_provider_gh "specforge-provider-gh" {
  family       extension
  features     [provider_based_ref_validation]
  depends_on   [specforge_validator, specforge_wasm]
}

library specforge_coverage "specforge-coverage" {
  family       core
  features     [test_coverage_reporting]
  depends_on   [specforge_graph]
  ports_defined [TestReporter]
}

library specforge_collect_rust "specforge-collect-rust" {
  family       extension
  features     [rust_test_collection]
  depends_on   [specforge_graph]
}

library specforge_test_lib "specforge-test" {
  family       extension
  features     [rust_proc_macro_annotation]
}

library specforge_test_macros_lib "specforge-test-macros" {
  family       extension
  features     [rust_proc_macro_annotation]
  depends_on   [specforge_test_lib]
}
