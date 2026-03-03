// Libraries — code packages that implement features

use features/parsing
use features/validation
use features/incremental
use features/output
use features/codegen
use features/coverage
use features/rust-codegen
use features/rust-collection
use features/migration
use features/lsp
use features/extensions
use features/project-init
use features/formatting
use ports/outbound
use ports/inbound

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
  features     [graph_validation]
  depends_on   [specforge_parser]
}

library specforge_graph "specforge-graph" {
  family       core
  features     [incremental_compilation]
  depends_on   [specforge_resolver]
}

library specforge_validator "specforge-validator" {
  family       core
  features     [graph_validation]
  depends_on   [specforge_graph]
  ports_defined [RefValidator]
}

library specforge_emitter "specforge-emitter" {
  family       core
  features     [markdown_documentation_generation, json_and_dot_export, traceability_reports]
  depends_on   [specforge_graph]
  ports_defined [OutputRenderer]
}

library specforge_watch "specforge-watch" {
  family       core
  features     [incremental_compilation]
  depends_on   [specforge_graph, specforge_validator]
  ports_defined [FileSystem]
}

library specforge_formatter "specforge-formatter" {
  family       core
  features     [code_formatting, lsp_format_on_save]
  depends_on   [tree_sitter_specforge, specforge_parser]
}

library specforge_cli "specforge-cli" {
  family       platform
  features     [project_initialization, ci_integration, format_version_migration, generator_plugin_protocol]
  depends_on   [specforge_parser, specforge_resolver, specforge_graph, specforge_validator, specforge_emitter, specforge_watch]
  ports_defined [CompilerApi]
}

library specforge_lsp "specforge-lsp" {
  family       platform
  features     [go_to_definition_and_references, hover_and_autocomplete, rename_refactoring, live_diagnostics_feature, code_actions, outline_and_symbol_search]
  depends_on   [specforge_parser, specforge_resolver, specforge_graph, specforge_validator, specforge_watch]
  ports_defined [LspProtocol]
}

library specforge_plugin_product "specforge-plugin-product" {
  family       plugin
  features     [plugin_management]
  depends_on   [specforge_validator]
}

library specforge_plugin_governance "specforge-plugin-governance" {
  family       plugin
  features     [plugin_management]
  depends_on   [specforge_validator]
}

library specforge_provider_gh "specforge-provider-gh" {
  family       plugin
  features     [provider_based_ref_validation]
  depends_on   [specforge_validator]
}

library specforge_gen_typescript "specforge-gen-typescript" {
  family       plugin
  features     [type_and_port_code_generation, test_stub_generation_and_drift_detection]
  depends_on   [specforge_graph]
}

library specforge_coverage "specforge-coverage" {
  family       core
  features     [test_coverage_reporting]
  depends_on   [specforge_graph]
  ports_defined [TestReporter]
}

library specforge_gen_rust "specforge-gen-rust" {
  family       plugin
  features     [rust_type_and_port_generation, rust_test_stub_generation, rust_test_collection]
  depends_on   [specforge_graph]
}

library specforge_test_lib "specforge-test" {
  family       plugin
  features     [rust_proc_macro_annotation]
}

library specforge_test_macros_lib "specforge-test-macros" {
  family       plugin
  features     [rust_proc_macro_annotation]
  depends_on   [specforge_test_lib]
}
