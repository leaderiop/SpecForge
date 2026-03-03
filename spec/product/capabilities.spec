// Capabilities — UX flows mapping personas to features

use features/project-init
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
use features/wasm
use features/formatting

// ── Developer + CLI ──────────────────────────────────────────

capability initialize_a_new_spec_project "Initialize a New Spec Project" {
  persona  developer
  surface  [cli]
  features [project_initialization]

  flow """
    1. Developer runs specforge init in an empty directory
    2. System prompts for project name and version
    3. System offers interactive plugin selection
    4. Developer selects desired plugins
    5. System generates specforge.json with configuration
    6. Success: project is ready for .spec file authoring
  """

  scenario "developer initializes a new project" {
    given "an empty directory with no specforge.json file"
    when "developer runs specforge init"
    then "a specforge.json file is generated with project name, version, and selected plugins"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability validate_spec_files "Validate Spec Files" {
  persona  developer
  surface  [cli]
  features [spec_file_parsing, error_recovery_during_parsing, graph_validation, scenario_declaration]

  flow """
    1. Developer runs specforge check
    2. System parses all .spec files
    3. System resolves imports and links references
    4. System validates the graph
    5. Success: diagnostics printed, exit code 0 if clean
    6. Failure: errors printed in rustc style with suggestions
  """

  scenario "developer validates spec files" {
    given "a project with valid .spec files and no broken references"
    when "developer runs specforge check"
    then "exit code is 0 and no error diagnostics are printed"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability watch_for_changes "Watch for Changes" {
  persona  developer
  surface  [cli]
  features [incremental_compilation]

  flow """
    1. Developer runs specforge watch
    2. System performs initial full compilation
    3. Developer edits a .spec file
    4. System detects change within 100ms
    5. System incrementally recompiles affected files
    6. Updated diagnostics are printed
    7. Cycle repeats on each file change
  """

  scenario "developer watches for file changes" {
    given "specforge watch is running after initial compilation"
    when "developer edits a .spec file"
    then "the system incrementally recompiles and prints updated diagnostics"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability generate_documentation "Generate Documentation" {
  persona  developer
  surface  [cli]
  features [markdown_documentation_generation, json_and_dot_export, traceability_reports]

  flow """
    1. Developer runs specforge render markdown ./docs/
    2. System traverses the graph
    3. System generates .md files with cross-reference links
    4. Success: documentation written to output directory
    5. Developer optionally filters by entity type
  """

  scenario "developer generates markdown documentation" {
    given "a project with valid .spec files"
    when "developer runs specforge render markdown ./docs/"
    then "markdown files with cross-reference links are written to the output directory"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability trace_requirements "Trace Requirements" {
  persona  developer
  surface  [cli]
  features [traceability_reports]

  flow """
    1. Developer runs specforge trace scaffold_new_project
    2. System traverses upstream to features, capabilities, deliverables
    3. System traverses downstream to invariants
    4. Trace chain is printed showing all links
    5. Missing links are flagged
  """

  scenario "developer traces requirements for an entity" {
    given "a project with linked entities across features, capabilities, and invariants"
    when "developer runs specforge trace scaffold_new_project"
    then "the full traceability chain is printed with any missing links flagged"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability generate_code_from_spec "Generate Code from Spec" {
  persona  developer
  surface  [cli]
  features [type_and_port_code_generation, test_stub_generation_and_drift_detection]

  flow """
    1. Developer configures gen block in specforge.json
    2. Developer runs specforge gen typescript
    3. System reads type and port blocks from the graph
    4. System generates interfaces, stubs, and test skeletons
    5. Success: generated files written to output directory
    6. Developer runs specforge gen --check in CI for drift detection
  """

  scenario "developer generates code from spec" {
    given "a gen block configured in specforge.json for typescript"
    when "developer runs specforge gen typescript"
    then "interfaces and test stubs are generated in the output directory"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability check_test_coverage "Check Test Coverage" {
  persona  developer
  surface  [cli]
  features [test_coverage_reporting, test_traceability]

  flow """
    1. Developer runs tests with framework plugin (e.g., vitest)
    2. Plugin emits specforge-report.json
    3. Developer runs specforge coverage
    4. System merges reports and computes statistics
    5. Success: coverage summary printed
    6. Developer sets --min threshold for CI gating
  """

  scenario "developer checks test coverage" {
    given "a specforge-report.json emitted by a test framework plugin"
    when "developer runs specforge coverage"
    then "a coverage summary is printed with statistics for all testable entities"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability trace_test_coverage "Trace Test Coverage" {
  persona  developer
  surface  [cli]
  features [test_traceability, test_coverage_reporting]

  flow """
    1. Developer runs specforge trace --test-results
    2. System parses specforge-report.json files
    3. System computes three-layer coverage (declared/linked/executed/passing)
    4. System renders traceability matrix showing each testable entity
    5. Developer reviews matrix to identify gaps
    6. Developer fills gaps by adding tests and updating tests field
  """

  scenario "developer traces fully covered entity" {
    given "a behavior with verify statements and a tests field pointing to an existing test file"
    given "a specforge-report.json with passing results for that behavior"
    when "developer runs specforge trace --test-results"
    then "the traceability matrix shows the behavior as passing at all four levels"
  }

  scenario "developer finds untested entity" {
    given "a behavior with verify statements but no tests field"
    given "no specforge-report.json entry for that behavior"
    when "developer runs specforge trace --test-results"
    then "the traceability matrix shows the behavior as declared only"
    then "a W018 warning is emitted for missing tests field"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability migrate_spec_format "Migrate Spec Format" {
  persona  developer
  surface  [cli]
  features [format_version_migration]

  flow """
    1. Developer upgrades specforge CLI
    2. Compiler detects older format version (I003)
    3. Developer runs specforge migrate --from=1.0 --to=2.0
    4. System transforms all .spec files to new format
    5. Developer runs specforge check to verify
    6. Success: all files use new format syntax
  """

  scenario "developer migrates spec format" {
    given "a project with .spec files in format version 1.0"
    when "developer runs specforge migrate --from=1.0 --to=2.0"
    then "all .spec files are transformed to format 2.0 and pass specforge check"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability diagnose_plugin_issues "Diagnose Plugin Issues" {
  persona  developer
  surface  [cli]
  features [entity_enhancement, plugin_management, wasm_package_runtime]

  flow """
    1. Developer runs specforge doctor
    2. System loads all plugin manifests
    3. System builds FieldRegistry with all enhancements
    4. System detects enhancement conflicts
    5. System checks AOT cache health
    6. System produces a report listing installed plugins, enhancements,
       conflicts with actionable resolution suggestions, and cache status
    7. Developer resolves issues based on report
  """

  scenario "developer diagnoses enhancement conflicts" {
    given "two installed plugins that register the same field name on the same entity kind"
    when "developer runs specforge doctor"
    then "a report lists the conflict with both plugin names and suggests resolution via enhancement_policy or explicit overrides"
  }

  scenario "developer diagnoses stale AOT cache" {
    given "an AOT cache with entries compiled by a previous Extism runtime version"
    when "developer runs specforge doctor"
    then "a report flags the stale cache entries and suggests running specforge cache clear"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability manage_plugins "Manage Plugins" {
  persona  developer
  surface  [cli]
  features [plugin_management, wasm_package_runtime, entity_enhancement]

  flow """
    1. Developer runs specforge add @specforge/governance
    2. System downloads the .wasm plugin binary
    3. System AOT compiles and caches in .specforge/cache/
    4. System adds plugin to specforge.json
    5. New entity types become available
    6. Developer runs specforge plugins to verify
    7. Developer can later run specforge remove to uninstall
  """

  scenario "developer manages plugins" {
    given "a project with no governance plugin installed"
    when "developer runs specforge add @specforge/governance"
    then "the plugin is listed in specforge plugins output and governance entities are available"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability define_custom_entity_types "Define Custom Entity Types" {
  persona  developer
  surface  [cli]
  features [plugin_management]

  flow """
    1. Developer adds a define block to specforge.json
    2. Define block declares id_prefix, required fields, optional fields, and reference targets
    3. Developer creates .spec files using the custom entity type
    4. Compiler registers the custom type alongside built-in and plugin types
    5. Custom entities participate in reference resolution and orphan detection
    6. Success: specforge check validates custom entities like built-in ones
    7. Failure: malformed define blocks produce diagnostics
  """

  scenario "developer defines a custom entity type" {
    given "a define block in specforge.json declaring a custom entity type"
    when "developer runs specforge check"
    then "custom entities are validated like built-in entities"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability configure_ref_providers "Configure Ref Providers" {
  persona  developer
  surface  [cli]
  features [provider_based_ref_validation]

  flow """
    1. Developer adds a providers block to specforge.json
    2. Developer configures one or more provider instances with aliases
    3. Developer writes ref entities using registered schemes (e.g., gh.issue:42)
    4. Compiler delegates validation to the matching provider instance
    5. Developer runs specforge providers to verify configured instances
    6. Success: valid refs are resolved, invalid identifiers produce diagnostics
    7. Developer can configure multiple instances of the same provider for different repos
  """

  scenario "developer configures ref providers" {
    given "a providers block in specforge.json with a configured provider instance"
    when "developer runs specforge check with ref entities using the registered scheme"
    then "valid refs are resolved and invalid identifiers produce diagnostics"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability view_project_statistics "View Project Statistics" {
  persona  developer
  surface  [cli]
  features [ci_integration]

  flow """
    1. Developer runs specforge stats
    2. System computes entity counts, coverage %, and orphan count
    3. Summary table is printed
    4. Developer uses this to track project health
  """

  scenario "developer views project statistics" {
    given "a project with multiple entities and test coverage data"
    when "developer runs specforge stats"
    then "a summary table with entity counts, coverage percentage, and orphan count is printed"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability verify_adapter_implementations_cap "Verify Adapter Implementations" {
  persona  developer
  surface  [cli]
  features [test_stub_generation_and_drift_detection]

  flow """
    1. Developer writes adapter implementing a generated port interface
    2. Developer runs specforge verify typescript
    3. System checks adapter implements all port methods
    4. Success: adapter verified
    5. Failure: missing methods are reported
  """

  scenario "developer verifies adapter implementations" {
    given "an adapter implementing a generated port interface"
    when "developer runs specforge verify typescript"
    then "the system confirms all port methods are implemented or reports missing methods"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability format_spec_files_cap "Format Spec Files" {
  persona  developer
  surface  [cli]
  features [code_formatting]

  flow """
    1. Developer runs specforge format
    2. System discovers all .spec files in the project
    3. System parses each file into a CST via tree-sitter
    4. System applies formatting rules and writes formatted output
    5. Success: formatted file names and summary count printed
    6. Developer optionally runs specforge format --diff to preview changes
  """

  scenario "developer formats spec files" {
    given "a project with .spec files containing inconsistent formatting"
    when "developer runs specforge format"
    then "all files are formatted to canonical style and a summary is printed"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability check_formatting_in_ci_cap "Check Formatting in CI" {
  persona  ci
  surface  [ci_surface]
  features [code_formatting]

  flow """
    1. CI pipeline runs specforge format --check
    2. System compares formatted output against existing files
    3. No changes needed: exit 0
    4. Changes needed: exit 1 with list of unformatted files
    5. Developer must run specforge format and commit
  """

  scenario "CI checks formatting compliance" {
    given "a CI pipeline configured with specforge format --check"
    when "spec files contain formatting inconsistencies"
    then "the pipeline fails with exit code 1 and lists unformatted files"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability format_on_save_cap "Format on Save in IDE" {
  persona  developer
  surface  [ide]
  features [lsp_format_on_save]

  flow """
    1. Developer enables format-on-save in their editor
    2. Developer edits a .spec file
    3. Developer saves the file
    4. LSP receives textDocument/formatting request
    5. LSP formats the document and returns TextEdit operations
    6. Editor applies edits atomically
    7. File is saved with canonical formatting
  """

  scenario "developer formats on save" {
    given "a .spec file open in an IDE with format-on-save enabled and the LSP server running"
    when "developer saves the file"
    then "the file is formatted to canonical style before being written to disk"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Developer + IDE ──────────────────────────────────────────

capability navigate_to_entity_definitions "Navigate to Entity Definitions" {
  persona  developer
  surface  [ide]
  features [go_to_definition_and_references]

  flow """
    1. Developer Ctrl+clicks on an entity ID in a .spec file
    2. LSP finds the entity's declaration
    3. Editor navigates to the declaration file and line
    4. Developer Ctrl+clicks on references to navigate further
  """

  scenario "developer navigates to entity definition" {
    given "a .spec file open in an IDE with the LSP server running"
    when "developer Ctrl+clicks on an entity ID"
    then "the editor navigates to the entity declaration file and line"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

capability explore_entity_references "Explore Entity References" {
  persona  developer
  surface  [ide]
  features [go_to_definition_and_references]

  flow """
    1. Developer right-clicks an entity ID
    2. Developer selects "Find All References"
    3. LSP returns all reference sites across workspace
    4. Developer clicks on results to navigate
  """

  scenario "developer explores entity references" {
    given "a .spec file open in an IDE with the LSP server running"
    when "developer selects Find All References on an entity ID"
    then "all reference sites across the workspace are returned"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

capability get_inline_help "Get Inline Help" {
  persona  developer
  surface  [ide]
  features [hover_and_autocomplete]

  flow """
    1. Developer hovers over an entity ID
    2. Popup shows entity title, contract/guarantee, reference count
    3. Developer types in reference list [INV-
    4. Autocomplete suggests matching invariants with titles
    5. Developer selects from suggestions
  """

  scenario "developer gets inline help" {
    given "a .spec file open in an IDE with the LSP server running"
    when "developer hovers over an entity ID"
    then "a popup shows the entity title, contract or guarantee, and reference count"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

capability rename_entities_safely "Rename Entities Safely" {
  persona  developer
  surface  [ide]
  features [rename_refactoring]

  flow """
    1. Developer triggers rename on an entity ID (F2)
    2. Developer types new ID
    3. LSP updates declaration and all references atomically
    4. All .spec files reflect the new name
  """

  scenario "developer renames an entity safely" {
    given "a .spec file open in an IDE with the LSP server running"
    when "developer triggers rename on an entity ID and types a new name"
    then "the declaration and all references are updated atomically across all files"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

capability see_live_errors_while_typing "See Live Errors While Typing" {
  persona  developer
  surface  [ide]
  features [live_diagnostics_feature]

  flow """
    1. Developer edits a .spec file
    2. LSP incrementally recompiles within 100ms
    3. Red squiggles appear on broken references
    4. Yellow squiggles appear on orphans
    5. Semantic tokens color entity IDs by type and highlight keywords
    6. Diagnostics update in real time as user types
  """

  scenario "developer sees live errors while typing" {
    given "a .spec file open in an IDE with the LSP server running"
    when "developer introduces a broken reference in the file"
    then "red squiggles appear on the broken reference and diagnostics update in real time"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

capability generate_test_stubs_from_ide "Generate Test Stubs from IDE" {
  persona  developer
  surface  [ide]
  features [code_actions]

  flow """
    1. Developer opens a .spec file with untested behaviors
    2. LSP shows a lightbulb code action on behaviors lacking test coverage
    3. Developer clicks the code action
    4. Developer selects target language (e.g., TypeScript, Python, Go)
    5. System generates a test stub file with behavior ID and verify descriptions pre-filled
    6. Success: test file is created and opened in the editor
    7. Failure: missing gen configuration produces a diagnostic
  """

  scenario "developer generates test stubs from IDE" {
    given "a .spec file with an untested behavior open in an IDE with the LSP server running"
    when "developer clicks the lightbulb code action and selects a target language"
    then "a test stub file is generated with the behavior ID and verify descriptions pre-filled"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

capability browse_file_structure "Browse File Structure" {
  persona  developer
  surface  [ide]
  features [outline_and_symbol_search]

  flow """
    1. Developer opens outline panel in IDE
    2. LSP shows tree of entities in the current file
    3. Developer clicks on entity to jump to its location
    4. Developer uses workspace symbol search to find entities across files
  """

  scenario "developer browses file structure" {
    given "a .spec file open in an IDE with the LSP server running"
    when "developer opens the outline panel"
    then "a tree of entities in the current file is shown and clicking navigates to each entity"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-lsp/tests/integration.rs"]
}

capability get_syntax_highlighting_without_lsp "Get Syntax Highlighting Without LSP" {
  persona  developer
  surface  [ide]
  features [editor_query_files]

  flow """
    1. Developer installs tree-sitter-specforge grammar in their editor
    2. Editor loads highlights.scm, folds.scm, and indents.scm
    3. Developer opens a .spec file
    4. Keywords, strings, entity IDs, and types are syntax-highlighted
    5. Brace-delimited blocks are foldable
    6. Indentation adjusts automatically on new lines after braces
    7. No LSP server required
  """

  scenario "developer gets syntax highlighting without LSP" {
    given "a tree-sitter-specforge grammar installed in the editor"
    when "developer opens a .spec file"
    then "keywords, strings, entity IDs, and types are syntax-highlighted without an LSP server"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-parser/tests/snapshot_tests.rs"]
}

// ── Architect + CLI ──────────────────────────────────────────

capability export_graph_as_json "Export Graph as JSON" {
  persona  architect
  surface  [cli]
  features [json_and_dot_export]

  flow """
    1. Architect runs specforge render json ./output/
    2. System traverses the full graph
    3. System serializes all entities, edges, and metadata to JSON
    4. Success: JSON file written to output directory
    5. Architect feeds JSON to external dashboards, analyzers, or custom tooling
  """

  scenario "architect exports graph as JSON" {
    given "a project with valid .spec files"
    when "architect runs specforge render json ./output/"
    then "a JSON file with all entities, edges, and metadata is written to the output directory"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability review_full_traceability "Review Full Traceability" {
  persona  architect
  surface  [cli]
  features [traceability_reports]

  flow """
    1. Architect runs specforge trace (no arguments)
    2. System prints full traceability from deliverables to invariants
    3. Gaps in the chain are highlighted
    4. Architect reviews chain completeness
    5. Architect addresses gaps by adding missing links
  """

  scenario "architect reviews full traceability" {
    given "a project with entities linked from deliverables to invariants"
    when "architect runs specforge trace"
    then "the full traceability chain is printed with gaps highlighted"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability visualize_spec_graph "Visualize Spec Graph" {
  persona  architect
  surface  [cli]
  features [json_and_dot_export]

  flow """
    1. Architect runs specforge graph | dot -Tsvg > spec.svg
    2. System emits DOT format graph
    3. Graphviz renders SVG visualization
    4. Architect reviews architecture and dependencies
  """

  scenario "architect visualizes the spec graph" {
    given "a project with valid .spec files"
    when "architect runs specforge graph piped to dot"
    then "an SVG visualization of the spec graph is produced"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Contributor + CLI ────────────────────────────────────────

capability author_a_custom_plugin "Author a Custom Plugin" {
  persona  contributor
  surface  [cli]
  features [plugin_management, wasm_package_authoring]

  flow """
    1. Contributor runs specforge plugin init to scaffold a Wasm plugin project
    2. Contributor implements initialize/validate/generate exports using the PDK
    3. Contributor runs specforge plugin build to compile to .wasm
    4. Contributor runs specforge plugin test to verify against fixtures
    5. Contributor installs locally via specforge add ./path/to/plugin.wasm
    6. Compiler loads .wasm, calls initialize(), and registers new entity types
    7. Contributor writes .spec files using the new entities
    8. Success: custom entities participate in resolution, validation, and orphan detection
    9. Failure: Wasm traps or manifest errors are reported as diagnostics
  """

  scenario "contributor authors a custom plugin" {
    given "a Wasm plugin built with specforge plugin build declaring custom entity types"
    when "contributor installs the plugin and runs specforge check"
    then "custom entities are validated and participate in resolution and orphan detection"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability author_a_custom_provider "Author a Custom Provider" {
  persona  contributor
  surface  [cli]
  features [provider_based_ref_validation]

  flow """
    1. Contributor creates a provider package implementing the ref validation interface
    2. Provider registers supported schemes and kinds
    3. Contributor configures a test instance in specforge.json providers block
    4. Developer writes ref entities using the provider's scheme
    5. Compiler delegates ref validation to the provider
    6. Success: valid refs pass, invalid identifiers produce diagnostics
    7. Failure: unknown schemes emit I005 if provider not installed
  """

  scenario "contributor authors a custom provider" {
    given "a provider package implementing the ref validation interface"
    when "contributor configures the provider and runs specforge check with ref entities"
    then "valid refs are resolved and invalid identifiers produce diagnostics"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability author_a_custom_generator "Author a Custom Generator" {
  persona  contributor
  surface  [cli]
  features [generator_plugin_protocol, wasm_package_authoring]

  flow """
    1. Contributor scaffolds a generator project with specforge plugin init --generator
    2. Contributor implements the generate() Wasm export using the PDK
    3. Generator accesses graph via specforge.query_graph host function
    4. Generator emits files via specforge.emit_file host function
    5. Generator emits diagnostics via specforge.emit_diagnostic host function
    6. Contributor compiles to .wasm with specforge plugin build
    7. Contributor configures gen block in specforge.json with wasmPath
    8. Developer runs specforge gen <name>
    9. Success: generated files are written to the output directory
    10. Failure: Wasm traps forwarded as compiler diagnostics
  """

  scenario "contributor authors a custom generator" {
    given "a generator .wasm binary configured in a gen block in specforge.json"
    when "developer runs specforge gen with the generator name"
    then "generated files are written to the output directory"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability scaffold_wasm_plugin "Scaffold a Wasm Plugin" {
  persona  contributor
  surface  [cli]
  features [wasm_package_authoring]

  flow """
    1. Contributor runs specforge plugin init
    2. System prompts for plugin name and type (entity plugin, provider, generator)
    3. System scaffolds project with manifest, src/ skeleton, and build config
    4. Skeleton implements initialize/validate/generate exports
    5. README includes PDK documentation and examples
    6. Success: project is ready for development
  """

  scenario "contributor scaffolds a Wasm plugin" {
    given "an empty directory"
    when "contributor runs specforge plugin init"
    then "a Wasm plugin project is scaffolded with manifest, source skeleton, and build config"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability test_wasm_plugin_locally_cap "Test a Wasm Plugin Locally" {
  persona  contributor
  surface  [cli]
  features [wasm_package_authoring]

  flow """
    1. Contributor runs specforge plugin test
    2. System builds .wasm binary
    3. System loads binary in production sandbox
    4. System runs against test fixtures
    5. Success: test results printed
    6. Failure: sandbox violations or Wasm traps reported
  """

  scenario "contributor tests a Wasm plugin locally" {
    given "a Wasm plugin project with test fixtures"
    when "contributor runs specforge plugin test"
    then "the plugin is built, loaded in the sandbox, and test results are printed"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability publish_wasm_plugin_cap "Publish a Wasm Plugin" {
  persona  contributor
  surface  [cli]
  features [wasm_package_authoring]

  flow """
    1. Contributor runs specforge plugin publish
    2. System validates manifest and .wasm binary
    3. System packages binary and manifest
    4. System publishes to configured registry (npm, OCI, or GitHub Releases)
    5. Success: package URL printed
    6. Failure: validation errors reported
  """

  scenario "contributor publishes a Wasm plugin" {
    given "a Wasm plugin project with a valid manifest and built .wasm binary"
    when "contributor runs specforge plugin publish"
    then "the plugin is packaged and published to the configured registry"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Developer + CLI (Rust) ───────────────────────────────────

capability generate_rust_code_from_spec "Generate Rust Code from Spec" {
  persona  developer
  surface  [cli]
  features [rust_type_and_port_generation, rust_test_stub_generation]

  flow """
    1. Developer configures gen rust block in specforge.json
    2. Developer runs specforge gen rust
    3. System reads type and port blocks from the graph
    4. System generates Rust structs, traits, and test stubs
    5. Success: generated files written to output directory with checksum headers
    6. Developer implements test bodies replacing todo!() placeholders
  """

  scenario "developer generates Rust code from spec" {
    given "a gen rust block configured in specforge.json with out and test_out directories"
    when "developer runs specforge gen rust"
    then "Rust structs, traits, and test stubs are generated in the output directories"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability collect_rust_test_results_cap "Collect Rust Test Results" {
  persona  developer
  surface  [cli]
  features [rust_test_collection]

  flow """
    1. Developer runs cargo nextest run --profile ci
    2. nextest produces JUnit XML in target/nextest/ci/
    3. Developer runs specforge collect rust --format=junit target/nextest/ci/junit.xml
    4. System parses JUnit XML and maps tests to entity IDs
    5. System emits specforge-report.json
    6. Developer runs specforge coverage to see results
  """

  scenario "developer collects Rust test results from nextest" {
    given "JUnit XML produced by cargo nextest in target/nextest/ci/"
    when "developer runs specforge collect rust --format=junit target/nextest/ci/junit.xml"
    then "specforge-report.json is emitted with tests mapped to entity IDs"
  }

  scenario "developer collects Rust test results from cargo test" {
    given "cargo test output available on stdin"
    when "developer pipes cargo test output to specforge collect rust"
    then "specforge-report.json is emitted with tests mapped via naming convention"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability annotate_tests_with_proc_macro "Annotate Tests with Proc Macro" {
  persona  developer
  surface  [cli]
  features [rust_proc_macro_annotation]

  flow """
    1. Developer adds specforge-test dependency to Cargo.toml
    2. Developer annotates test with #[specforge::test("entity_id")]
    3. Developer runs cargo test
    4. Drop guard records pass/fail to target/specforge/
    5. Developer runs specforge collect rust to gather results
  """

  scenario "developer annotates a test with the proc macro" {
    given "specforge-test is a dependency and a test function exists"
    when "developer annotates the test with #[specforge::test(\"validate_input\")] and runs cargo test"
    then "a mapping file in target/specforge/ records the test result for validate_input"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability detect_rust_code_drift_in_ci "Detect Rust Code Drift in CI" {
  persona  ci
  surface  [ci_surface]
  features [rust_test_stub_generation]

  flow """
    1. CI pipeline runs specforge gen rust --check
    2. System compares SHA256 checksums in generated files against spec state
    3. No drift: exit 0
    4. Drift detected: exit 1 with stale file paths
    5. Developer must regenerate and commit
  """

  scenario "CI detects Rust code drift" {
    given "a CI pipeline configured with specforge gen rust --check"
    when "generated Rust files have stale checksums"
    then "the pipeline fails with exit code 1 and lists the stale file paths"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability gate_rust_coverage_in_ci "Gate on Rust Spec Coverage in CI" {
  persona  ci
  surface  [ci_surface]
  features [rust_test_collection]

  flow """
    1. CI pipeline runs cargo nextest and collects results
    2. CI pipeline runs specforge collect rust to emit report
    3. CI pipeline runs specforge coverage --min=90
    4. Coverage above threshold: exit 0
    5. Coverage below threshold: exit 1 with uncovered entities
  """

  scenario "CI gates on Rust spec coverage" {
    given "a CI pipeline that runs specforge collect rust then specforge coverage --min=90"
    when "Rust spec coverage is below the 90 percent threshold"
    then "the pipeline fails with exit code 1 and lists uncovered entities"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── CI + CI Surface ──────────────────────────────────────────

capability run_spec_validation_in_ci "Run Spec Validation in CI" {
  persona  ci
  surface  [ci_surface]
  features [ci_integration]

  flow """
    1. CI pipeline runs specforge check --strict
    2. System validates all .spec files
    3. Exit code 0: pipeline continues
    4. Exit code 1: pipeline fails with error details
    5. Warnings are treated as errors in strict mode
  """

  scenario "CI runs spec validation in strict mode" {
    given "a CI pipeline configured with specforge check --strict"
    when "warnings exist in the spec files"
    then "the pipeline fails with exit code 1 and warning details"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability gate_on_coverage_in_ci "Gate on Coverage in CI" {
  persona  ci
  surface  [ci_surface]
  features [test_coverage_reporting]

  flow """
    1. CI pipeline runs test suite with framework plugin
    2. CI pipeline runs specforge coverage --min=90
    3. Coverage above threshold: exit 0
    4. Coverage below threshold: exit 1 with summary
  """

  scenario "CI gates on coverage threshold" {
    given "a CI pipeline configured with specforge coverage --min=90"
    when "test coverage is below the 90 percent threshold"
    then "the pipeline fails with exit code 1 and a coverage summary"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

capability detect_code_drift_in_ci "Detect Code Drift in CI" {
  persona  ci
  surface  [ci_surface]
  features [test_stub_generation_and_drift_detection]

  flow """
    1. CI pipeline runs specforge gen typescript --check
    2. System compares generated output against committed files
    3. No drift: exit 0
    4. Drift detected: exit 1 with differing file paths
    5. Developer must regenerate and commit
  """

  scenario "CI detects code drift" {
    given "a CI pipeline configured with specforge gen typescript --check"
    when "generated output differs from committed files"
    then "the pipeline fails with exit code 1 and lists the differing file paths"
  }

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
