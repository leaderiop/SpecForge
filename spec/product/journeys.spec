// Journeys — UX flows mapping personas to features
// SpecForge is the structured context standard for AI agents.
// The graph is the product — consumed by any agent for any task.

use "extensions/cargo-test/features"
use "extensions/testing/features"
use "extensions/vitest/features"
use "features/extensions"
use "features/formatting"
use "features/incremental"
use "features/lsp"
use "features/mcp"
use "features/migration"
use "features/output"
use "features/parsing"
use "features/project-init"
use "features/validation"
use "features/wasm"
use "features/zero-entity-core"
use "product/channels"
use "product/features"
use "product/personas"

defaults journey {
  channels [cli]
}

// ── Developer + CLI ──────────────────────────────────────────

journey initialize_a_new_spec_project "Initialize a New Spec Project" {
  persona  developer
  priority critical
  tags     ["developer", "cli"]
  features [project_initialization]
  flow     [
    "Developer runs specforge init in an empty directory",
    "System prompts for project name (version defaults to 0.1.0)",
    "System offers interactive extension selection",
    "Developer selects desired extensions (or none — zero extensions is valid)",
    "System generates specforge.json with configuration",
    "System creates a starter .spec file using structural syntax (extensions MAY contribute templates, but core generates only generic entity blocks)",
    "Developer runs specforge check — validates immediately",
    "Developer runs specforge export — produces Graph Protocol JSON",
    "Success: project is ready for .spec file authoring in under 60 seconds via three commands (init, check, export)",
  ]
}

journey initialize_project_non_interactively "Initialize a Project Non-Interactively" {
  persona  ci
  priority high
  tags     ["ci", "cli"]
  features [project_initialization]
  flow     [
    "CI script runs specforge init --name myproject --extensions @specforge/software",
    "System creates specforge.json with specified configuration (no prompts)",
    "System creates a starter .spec file",
    "CI script runs specforge check — validates with zero errors",
    "CI script runs specforge export — produces Graph Protocol JSON",
    "Success: project is ready for automated spec validation via three commands (init, check, export)",
  ]
}

journey initialize_project_as_agent "Initialize a Project as an AI Agent" {
  persona  agent
  priority high
  tags     ["agent", "cli"]
  features [project_initialization]
  flow     [
    "AI agent runs specforge init --name myproject --extensions @specforge/software,@specforge/product",
    "System creates specforge.json with specified configuration (no prompts, no TTY required)",
    "System creates a starter .spec file using structural syntax",
    "Agent runs specforge check to validate the project",
    "Agent runs specforge export --format=context to get the initial graph",
    "Agent uses the graph as structured context for subsequent tasks",
    "Success: agent has a consumable graph in under 60 seconds",
  ]
}

journey validate_spec_files "Validate Spec Files" {
  persona  developer
  priority critical
  tags     ["developer", "cli"]
  features [
    spec_file_parsing,
    error_recovery_during_parsing,
    reference_resolution,
    graph_construction,
    structural_validation,
    diagnostic_reporting,
    declarative_validation_rules,
    zero_entity_bootstrap,
  ]
  flow     [
    "Developer runs specforge check",
    "System parses all .spec files",
    "System resolves imports and links references",
    "System validates the graph",
    "Success: diagnostics printed, exit code 0 if clean",
    "Failure: errors printed in rustc style with suggestions",
  ]
}

journey watch_for_changes "Watch for Changes" {
  persona  developer
  priority critical
  tags     ["developer", "cli"]
  features [incremental_compilation]
  flow     [
    "Developer runs specforge watch",
    "System performs initial full compilation",
    "Developer edits a .spec file",
    "System detects change within 100ms",
    "System incrementally recompiles affected files",
    "Updated diagnostics are printed",
    "Cycle repeats on each file change",
  ]
}

journey generate_documentation "Generate Documentation" {
  persona  developer
  priority high
  tags     ["developer", "cli"]
  features [json_and_dot_render]
  flow     [
    "Developer installs a renderer extension (e.g., @specforge/markdown-renderer)",
    "Developer runs specforge render markdown ./docs/",
    "Renderer extension traverses the graph via specforge.query_graph host function",
    "Renderer produces human-readable documentation files via specforge.emit_file",
    "Developer reviews generated documentation",
    "Success: documentation reflects current spec state",
    "Note: SpecForge renders documentation, not code — code generation is the agent's job",
  ]
}

journey export_graph_formats "Export Graph Formats" {
  persona  developer
  priority high
  tags     ["developer", "cli"]
  features [json_and_dot_render, traceability_serialization]
  flow     [
    "Developer runs specforge render json ./output/ or specforge graph",
    "System traverses the graph and serializes in the requested format",
    "JSON export produces full Graph Protocol output; DOT produces Graphviz visualization",
    "Success: machine-readable graph written to output directory",
  ]
}

journey trace_requirements "Trace Requirements" {
  persona  developer
  priority high
  tags     ["developer", "cli"]
  features [traceability_serialization]
  flow     [
    "Developer runs specforge trace scaffold_new_project",
    "System traverses upstream following all registered edge types",
    "System traverses downstream following all registered edge types",
    "Trace chain is printed showing all links",
    "Missing links are flagged",
  ]
}

// specforge export is a separate command from specforge render — export writes to stdout in agent-optimized formats, render writes files

journey export_graph_for_agents "Export Graph for AI Agents" {
  persona  developer
  channels [cli, ch_graph_protocol]
  priority critical
  tags     ["developer", "cli"]
  features [json_and_dot_render, agent_export]
  flow     [
    "Developer runs specforge export --format=context",
    "System compiles all .spec files into the entity graph",
    "System serializes graph in token-optimized format for AI agent consumption",
    "Developer feeds output to any AI agent (coding, PM, compliance, docs, security)",
    "Agent performs task correctly on first attempt using structured context",
    "Alternative formats: --format=graph (full JSON), --format=brief (IDs + contracts only)",
  ]
}

journey check_test_coverage "Check Test Coverage" {
  persona  developer
  priority high
  tags     ["developer", "cli"]
  features [test_result_collection, te_coverage_analysis]
  flow     [
    "Developer runs specforge collect",
    "System runs each detected runner extension's approved test command (e.g. vitest, cargo test) and records results in specforge-report.json",
    "Developer runs specforge analyze coverage",
    "System scores intent, enforcement and proof per entity",
    "Success: coverage findings and summary printed",
    "Developer sets --min threshold for CI gating",
  ]
}

journey trace_test_coverage "Trace Test Coverage" {
  persona  developer
  priority high
  tags     ["developer", "cli"]
  features [te_coverage_analysis]
  flow     [
    "Developer runs specforge analyze coverage after specforge collect",
    "System reads the recorded results from specforge-report.json",
    "System reports testable entities without obligations (A001), with failing tests (A014), and the discharge funnel of proven entities",
    "Developer reviews the findings to identify gaps",
    "Developer fills gaps by adding tests annotated with the entity they prove",
  ]
}

journey diagnose_extension_issues "Diagnose Extension Issues" {
  persona  developer
  priority medium
  tags     ["developer", "cli"]
  features [entity_enhancement, extension_management, wasm_extension_runtime, extension_manifest]
  flow     [
    "Developer runs specforge doctor",
    "System loads all extension manifests",
    "System builds KindRegistry and FieldRegistry from extension declarations",
    "System detects entity kind conflicts and enhancement conflicts",
    "System checks Wasm compile cache health",
    "System produces a report listing installed extensions, entity kinds, enhancements, conflicts with actionable resolution suggestions, and cache status",
    "Developer resolves issues based on report",
  ]
}

journey manage_extensions "Manage Extensions" {
  persona  developer
  priority medium
  tags     ["developer", "cli"]
  features [extension_management, wasm_extension_runtime, entity_enhancement, extension_manifest]
  flow     [
    "Developer runs specforge add @specforge/governance",
    "System downloads the .wasm extension binary",
    "System verifies the binary SHA256 and places it atomically under .specforge/extensions/",
    "System adds extension to specforge.json",
    "New entity kinds become available from the extension's domain vocabulary",
    "Developer runs specforge extensions to verify installed extensions",
    "Developer can later run specforge remove to uninstall",
  ]
}

journey install_domain_extensions "Install Domain Extensions" {
  persona  developer
  priority medium
  tags     ["developer", "cli"]
  features [extension_management, wasm_extension_runtime, dynamic_entity_registration]
  flow     [
    "Developer runs specforge add @specforge/atomic-design",
    "System resolves extension from registry",
    "System downloads .wasm binary and validates manifest",
    "System verifies the binary SHA256 and places it atomically under .specforge/extensions/",
    "System adds extension to specforge.json extensions list",
    "New entity kinds (atom, molecule, organism, template, page) become available",
    "Developer writes .spec files using the new domain vocabulary",
    "All validation, LSP, and graph export work with new entity types",
  ]
}

journey configure_ref_providers "Configure Ref Providers" {
  persona  developer
  priority medium
  tags     ["developer", "cli"]
  features [provider_based_ref_validation]
  flow     [
    "Developer adds a providers block to specforge.json",
    "Developer configures one or more provider instances with aliases",
    "Developer writes ref entities using registered schemes (e.g., gh.issue:42)",
    "Compiler delegates validation to the matching provider instance",
    "Developer runs specforge providers to verify configured instances",
    "Success: valid refs are resolved, invalid identifiers produce diagnostics",
    "Developer can configure multiple instances of the same provider for different repos",
  ]
}

journey view_project_statistics "View Project Statistics" {
  persona  developer
  priority medium
  tags     ["developer", "cli"]
  features [ci_integration]
  flow     [
    "Developer runs specforge stats",
    "System computes entity counts, coverage %, and unreferenced count",
    "Summary table is printed",
    "Developer uses this to track project health",
  ]
}

journey j_query_graph_multi_resolution "Query Graph at Multiple Resolutions" {
  persona  developer
  priority medium
  tags     ["developer", "cli"]
  features [agent_export]
  flow     [
    "Developer runs specforge query --scope=verify auth_login",
    "System returns only the verify declarations for the specified behavior",
    "Developer runs specforge query --depth=1 auth_login for adjacent entities",
    "Developer runs specforge query --scope=deep auth_feature for full subgraph",
    "Agent receives exactly the context slice it needs — no more, no less",
    "Works for any agent type: coding, PM, compliance, documentation",
  ]
}

journey j_format_spec_files "Format Spec Files" {
  persona  developer
  priority high
  tags     ["developer", "cli"]
  features [code_formatting]
  flow     [
    "Developer runs specforge format",
    "System discovers all .spec files in the project",
    "System parses each file into a CST via tree-sitter",
    "System applies formatting rules and writes formatted output",
    "Success: formatted file names and summary count printed",
    "Developer optionally runs specforge format --diff to preview changes",
  ]
}

journey check_formatting_in_ci "Check Formatting in CI" {
  persona  ci
  channels [ci_surface]
  priority high
  tags     ["ci", "automation"]
  features [code_formatting]
  flow     [
    "CI pipeline runs specforge format --check",
    "System compares formatted output against existing files",
    "No changes needed: exit 0",
    "Changes needed: exit 1 with list of unformatted files",
    "Developer must run specforge format and commit",
  ]
}

journey format_on_save "Format on Save in IDE" {
  persona  developer
  channels [ide]
  priority medium
  tags     ["developer", "ide"]
  features [lsp_formatting]
  flow     [
    "Developer enables format-on-save in their editor",
    "Developer edits a .spec file",
    "Developer saves the file",
    "LSP receives textDocument/formatting request",
    "LSP formats the document and returns TextEdit operations",
    "Editor applies edits atomically",
    "File is saved with canonical formatting",
  ]
}

// ── Developer + IDE ──────────────────────────────────────────

journey navigate_to_entity_definitions "Navigate to Entity Definitions" {
  persona  developer
  channels [ide]
  priority high
  tags     ["developer", "ide"]
  features [go_to_definition_and_references]
  flow     [
    "Developer Ctrl+clicks on an entity ID in a .spec file",
    "LSP finds the entity's declaration",
    "Editor navigates to the declaration file and line",
    "Developer Ctrl+clicks on references to navigate further",
  ]
}

journey explore_entity_references "Explore Entity References" {
  persona  developer
  channels [ide]
  priority medium
  tags     ["developer", "ide"]
  features [go_to_definition_and_references]
  flow     [
    "Developer right-clicks an entity ID",
    "Developer selects \"Find All References\"",
    "LSP returns all reference sites across workspace",
    "Developer clicks on results to navigate",
  ]
}

journey get_inline_help "Get Inline Help" {
  persona  developer
  channels [ide]
  priority medium
  tags     ["developer", "ide"]
  features [hover_and_autocomplete, extension_driven_lsp]
  flow     [
    "Developer hovers over an entity ID",
    "Popup shows entity title, contract/guarantee, reference count",
    "Developer types in reference list [INV-",
    "Autocomplete suggests matching invariants with titles",
    "Developer selects from suggestions",
  ]
}

journey rename_entities_safely "Rename Entities Safely" {
  persona  developer
  channels [ide]
  priority medium
  tags     ["developer", "ide"]
  features [rename_refactoring]
  flow     [
    "Developer triggers rename on an entity ID (F2)",
    "Developer types new ID",
    "LSP updates declaration and all references atomically",
    "All .spec files reflect the new name",
  ]
}

journey see_live_errors_while_typing "See Live Errors While Typing" {
  persona  developer
  channels [ide]
  priority high
  tags     ["developer", "ide"]
  features [live_diagnostics, extension_driven_lsp]
  flow     [
    "Developer edits a .spec file",
    "LSP incrementally recompiles within 100ms",
    "Red squiggles appear on broken references",
    "Yellow squiggles appear on unreferenced entities",
    "Semantic tokens color entity IDs by type and highlight keywords",
    "Diagnostics update in real time as user types",
  ]
}

journey suggest_test_declarations_from_ide "Suggest Test Declarations from IDE" {
  persona  developer
  channels [ide]
  priority medium
  tags     ["developer", "ide"]
  features [code_actions]
  flow     [
    "Developer opens a .spec file with untested behaviors",
    "LSP shows a lightbulb code action on behaviors lacking test coverage",
    "Developer clicks the code action",
    "LSP suggests adding verify declarations or linking test files",
    "Success: code action applies the suggested edit",
  ]
}

journey browse_file_structure "Browse File Structure" {
  persona  developer
  channels [ide]
  priority medium
  tags     ["developer", "ide"]
  features [outline_and_symbol_search, extension_driven_lsp]
  flow     [
    "Developer opens outline panel in IDE",
    "LSP shows tree of entities in the current file",
    "Developer clicks on entity to jump to its location",
    "Developer uses workspace symbol search to find entities across files",
  ]
}

journey get_syntax_highlighting_without_lsp "Get Syntax Highlighting Without LSP" {
  persona  developer
  channels [ide]
  priority medium
  tags     ["developer", "ide"]
  features [editor_query_files]
  flow     [
    "Developer installs tree-sitter-specforge grammar in their editor",
    "Editor loads highlights.scm, folds.scm, and indents.scm",
    "Developer opens a .spec file",
    "Keywords, strings, entity IDs, and types are syntax-highlighted",
    "Brace-delimited blocks are foldable",
    "Indentation adjusts automatically on new lines after braces",
    "No LSP server required",
  ]
}

// ── Product Manager + CLI ────────────────────────────────────

journey review_milestone_progress "Review Milestone Progress" {
  persona  product_manager
  priority medium
  tags     ["product_manager", "cli"]
  features [json_and_dot_render, traceability_serialization]
  flow     [
    "Product manager runs specforge export --format=graph",
    "System computes milestone completion ratios for all milestones",
    "Product manager reviews which features are implemented vs unknown",
    "Product manager identifies blocked milestones via dependency chain",
    "Product manager exports data to a dashboard or spreadsheet",
  ]
}

journey review_deliverable_scope "Review Deliverable Scope" {
  persona  product_manager
  priority medium
  tags     ["product_manager", "cli"]
  features [traceability_serialization, json_and_dot_render]
  flow     [
    "Product manager runs specforge trace (no arguments)",
    "System prints full traceability for all deliverables",
    "Product manager reviews which journeys and modules compose each deliverable",
    "Product manager identifies gaps in journey-to-module feature coverage",
    "Product manager communicates delivery status to stakeholders",
  ]
}

journey review_term_glossary "Review Term Glossary" {
  persona  product_manager
  priority medium
  tags     ["product_manager", "cli"]
  features [json_and_dot_render]
  flow     [
    "Product manager runs specforge product terms to list all defined terms",
    "System returns paginated term list with definitions and alias counts",
    "Product manager identifies terms with zero see_also links (I010)",
    "Product manager reads the default human table, or --format json for tooling",
    "Product manager uses term graph query to explore related terms",
    "Product manager verifies shared vocabulary consistency across the team",
    "Success: glossary is reviewed and terminology drift is caught early",
  ]
}

journey review_persona_channel_landscape "Review Persona and Channel Landscape" {
  persona  product_manager
  priority medium
  tags     ["product_manager", "cli"]
  features [json_and_dot_render]
  flow     [
    "Product manager runs specforge product:personas to list all personas",
    "Product manager runs specforge product:channels to list all channels",
    "Product manager identifies deprecated personas/channels without reasons (I069, I070)",
    "Product manager uses persona-journeys and channel-journeys queries to review coverage",
    "Product manager identifies unreferenced personas (I046) and channels (I047) not used by any journey",
    "Success: persona and channel landscape is reviewed for completeness and lifecycle hygiene",
  ]
}

// ── Architect + CLI ──────────────────────────────────────────

journey export_graph_as_json "Export Graph as JSON" {
  persona  architect
  priority medium
  tags     ["architect", "cli"]
  features [json_and_dot_render]
  flow     [
    "Architect runs specforge render json ./output/",
    "System traverses the full graph",
    "System serializes all entities, edges, and metadata to JSON",
    "Success: JSON file written to output directory",
    "Architect feeds JSON to external dashboards, analyzers, or custom tooling",
  ]
}

journey review_full_traceability "Review Full Traceability" {
  persona  architect
  priority medium
  tags     ["architect", "cli"]
  features [traceability_serialization]
  flow     [
    "Architect runs specforge trace (no arguments)",
    "System prints full traceability across all registered edge types",
    "Gaps in the chain are highlighted",
    "Architect reviews chain completeness",
    "Architect addresses gaps by adding missing links",
  ]
}

journey visualize_spec_graph "Visualize Spec Graph" {
  persona  architect
  priority medium
  tags     ["architect", "cli"]
  features [json_and_dot_render]
  flow     [
    "Architect runs specforge graph | dot -Tsvg > spec.svg",
    "System emits DOT format graph",
    "Graphviz renders SVG visualization",
    "Architect reviews architecture and dependencies",
  ]
}

// ── Contributor + CLI ────────────────────────────────────────

journey author_a_domain_extension "Author a Domain Extension" {
  persona  contributor
  priority low
  tags     ["contributor", "cli"]
  features [extension_management, wasm_extension_authoring]
  flow     [
    "Contributor runs specforge extension init to scaffold a Wasm domain extension",
    "Contributor defines entity_kinds, edge_types, and validation_rules in manifest",
    "Contributor implements validation exports using the PDK",
    "Contributor runs specforge extension build to compile to .wasm",
    "Contributor runs specforge extension test to verify against fixtures",
    "Contributor installs locally via specforge add ./path/to/extension.wasm",
    "Compiler loads .wasm and registers the domain vocabulary",
    "Contributor writes .spec files using the new entity kinds",
    "Success: extension entities participate in resolution, validation, and graph export",
    "Failure: Wasm traps or manifest errors are reported as diagnostics",
  ]
}

journey author_a_custom_provider "Author a Custom Provider" {
  persona  contributor
  priority low
  tags     ["contributor", "cli"]
  features [provider_based_ref_validation]
  flow     [
    "Contributor creates a provider extension implementing the ref validation interface",
    "Provider registers supported schemes and kinds",
    "Contributor configures a test instance in specforge.json providers block",
    "Developer writes ref entities using the provider's scheme",
    "Compiler delegates ref validation to the provider",
    "Success: valid refs pass, invalid identifiers produce diagnostics",
    "Failure: unknown schemes emit I005 if provider not installed",
  ]
}

journey scaffold_wasm_extension "Scaffold a Wasm Extension" {
  persona  contributor
  priority low
  tags     ["contributor", "cli"]
  features [wasm_extension_authoring]
  flow     [
    "Contributor runs specforge extension init",
    "System prompts for extension name and contribution types (entities, validators, renderers, providers, parsers, collectors)",
    "System scaffolds project with manifest v2, src/ skeleton, and build config",
    "Manifest declares entity_kinds, edge_types, validation_rules",
    "README includes PDK documentation and examples",
    "Success: project is ready for development",
  ]
}

journey test_wasm_extension_locally "Test Wasm Extension Locally" {
  persona  contributor
  priority low
  tags     ["contributor", "cli"]
  features [wasm_extension_authoring]
  flow     [
    "Contributor runs specforge extension test",
    "System builds .wasm binary",
    "System loads binary in production sandbox",
    "System runs against test fixtures",
    "Success: test results printed",
    "Failure: sandbox violations or Wasm traps reported",
  ]
}

journey j_publish_wasm_extension "Publish Wasm Extension" {
  persona  contributor
  priority low
  tags     ["contributor", "cli"]
  features [wasm_extension_authoring]
  flow     [
    "Contributor runs specforge extension publish",
    "System validates manifest v2 and .wasm binary",
    "System bundles binary and manifest",
    "System publishes to configured registry (npm, OCI, or GitHub Releases)",
    "Success: extension URL printed",
    "Failure: validation errors reported",
  ]
}

// ── Developer + CLI (Agent Integration) ──────────────────────

journey j_validate_agent_plan "Validate Agent Implementation Plan" {
  persona  developer
  priority medium
  tags     ["developer", "agent_integration"]
  features [traceability_serialization]
  flow     [
    "AI agent produces a plan.json mapping entities to planned actions",
    "Developer runs specforge trace --plan plan.json",
    "System validates every testable entity has a planned implementation",
    "System checks no unreferenced plans reference nonexistent entities",
    "System verifies dependency order is valid",
    "Success: plan is consistent with the spec graph",
    "Failure: gaps and inconsistencies reported with suggestions",
  ]
}

journey j_collect_rust_test_results "Collect Rust Test Results" {
  persona  developer
  priority medium
  tags     ["developer", "agent_integration"]
  features [ct_cargo_test_collection]
  flow     [
    "Developer runs specforge collect in a Cargo project",
    "System shows the cargo test command and asks for approval once",
    "System runs it; the specforge-test attribute writes one report per test binary",
    "@specforge/cargo-test maps the reports to entity IDs",
    "System writes specforge-report.json",
    "Developer runs specforge analyze coverage to see results",
  ]
}

journey annotate_tests_with_proc_macro "Annotate Tests with Proc Macro" {
  persona  developer
  priority medium
  tags     ["developer", "agent_integration"]
  features [rust_proc_macro_annotation]
  flow     [
    "Developer adds specforge-test dependency to Cargo.toml",
    "Developer writes #[specforge_test(behavior = \"entity_id\")] in place of #[test]",
    "Developer runs specforge collect (or cargo test, then specforge collect --no-run)",
    "Drop guard records pass/fail to target/specforge/",
    "System records which entities the tests prove",
  ]
}

journey export_scoped_context_for_agent "Export Scoped Context for Agent" {
  persona  developer
  priority high
  tags     ["developer", "agent_integration"]
  features [agent_export]
  flow     [
    "Developer runs specforge export --format=context --scope=payments",
    "System compiles only the payments subgraph",
    "System produces token-optimized output for agent context window",
    "Any AI agent (coding, PM, compliance, docs) receives focused context",
    "Token usage reduced by 90%+ compared to full graph export",
  ]
}

// ── CI + CI Surface ──────────────────────────────────────────

journey run_spec_validation_in_ci "Run Spec Validation in CI" {
  persona  ci
  channels [ci_surface]
  priority high
  tags     ["ci", "automation"]
  features [ci_integration]
  flow     [
    "CI pipeline runs specforge check --strict",
    "System validates all .spec files",
    "Exit code 0: pipeline continues",
    "Exit code 1: pipeline fails with error details",
    "Warnings are treated as errors in strict mode",
  ]
}

journey gate_on_coverage_in_ci "Gate on Coverage in CI" {
  persona  ci
  channels [ci_surface]
  priority high
  tags     ["ci", "automation"]
  features [te_coverage_analysis]
  flow     [
    "CI pipeline runs specforge collect --yes",
    "CI pipeline runs specforge analyze coverage --min 90",
    "Coverage above threshold: exit 0",
    "Coverage below threshold: exit 1 with summary",
  ]
}

journey validate_graph_in_ci "Validate Graph Integrity in CI" {
  persona  ci
  channels [ci_surface]
  priority medium
  tags     ["ci", "automation"]
  features [ci_integration]
  flow     [
    "CI pipeline runs specforge check --strict",
    "System validates all .spec files and graph integrity",
    "CI pipeline runs specforge export --format=graph --check",
    "System verifies graph protocol output is consistent",
    "No issues: exit 0",
    "Issues detected: exit 1 with diagnostics",
  ]
}

// ── Agent + MCP ──────────────────────────────────────────

journey consume_graph_via_mcp "Consume Graph via MCP" {
  persona  agent
  channels [mcp]
  priority high
  tags     ["agent", "mcp"]
  features [mcp_resource_exposure, mcp_core_tools]
  flow     [
    "AI agent connects to SpecForge MCP server",
    "Agent reads specforge://graph resource for full graph",
    "Agent reads specforge://schema resource to understand graph structure",
    "Agent reads specforge://context for token-optimized format",
    "Agent reads specforge://brief for minimal IDs + edges",
    "Agent reads specforge://diagnostics for current validation state",
    "Agent calls specforge.query tool for focused subgraph extraction",
    "Agent calls specforge.export tool for format-specific output",
    "Agent calls specforge.validate tool to validate after spec edits",
    "Agent calls specforge.search to discover entities by kind, field, or text",
    "Agent calls specforge.stats for project health overview",
    "Success: agent has structured, typed access to the full spec graph",
  ]
}

journey receive_delta_notifications_via_mcp "Receive Delta Notifications via MCP" {
  persona  agent
  channels [mcp]
  priority medium
  tags     ["agent", "mcp"]
  features [mcp_delta_notifications]
  flow     [
    "AI agent subscribes to specforge://graph via MCP subscribe",
    "Agent subscribes to diagnostics via specforge/diagnosticsChanged",
    "Developer edits a .spec file",
    "System incrementally recompiles and computes GraphDelta",
    "System sends specforge/graphChanged to subscribed agents",
    "System sends specforge/diagnosticsChanged with DiagnosticsDelta",
    "Agent receives deltas with added/removed/modified nodes and diagnostics",
    "Agent updates its internal context incrementally",
    "Success: agent stays synchronized without polling",
  ]
}

journey navigate_spec_graph_via_mcp "Navigate Spec Graph via MCP" {
  persona  agent
  channels [mcp]
  priority high
  tags     ["agent", "mcp"]
  features [mcp_navigation_tools]
  flow     [
    "AI agent connects to SpecForge MCP server",
    "Agent calls specforge.inspect to get entity details (kind, title, contract, references)",
    "Agent calls specforge.find_definition to locate an entity's source file and line",
    "Agent calls specforge.find_references to discover all usage sites",
    "Agent calls specforge.outline to browse entities in a specific file",
    "Agent calls specforge.suggest_fixes to get actionable fix suggestions for diagnostics",
    "Success: agent has LSP-equivalent navigation without an LSP client",
  ]
}

journey mutate_spec_project_via_mcp "Mutate Spec Project via MCP" {
  persona  agent
  channels [mcp]
  priority medium
  tags     ["agent", "mcp"]
  features [mcp_mutation_tools]
  flow     [
    "AI agent connects to SpecForge MCP server",
    "Agent calls specforge.init to create a new spec project",
    "Agent calls specforge.add_extension to install domain extensions",
    "Agent calls specforge.format to enforce canonical formatting",
    "Agent calls specforge.rename to safely rename entities across all files",
    "Agent calls specforge.migrate to upgrade spec files to current format version",
    "Agent calls specforge.remove_extension to uninstall unused extensions",
    "Success: agent can fully manage spec project lifecycle through MCP",
  ]
}

journey manage_spec_project_via_mcp "Manage Spec Project via MCP" {
  persona  agent
  channels [mcp]
  priority medium
  tags     ["agent", "mcp"]
  features [mcp_project_management_tools]
  flow     [
    "AI agent connects to SpecForge MCP server",
    "Agent calls specforge.extensions to list installed extensions",
    "Agent calls specforge.providers to list configured ref providers",
    "Agent calls specforge.doctor to diagnose extension conflicts and cache issues",
    "Agent calls specforge.collect to gather test results from framework extensions",
    "Agent calls specforge.render to produce JSON or DOT output files",
    "Success: agent has full project management visibility through MCP",
  ]
}

journey use_guided_prompts_via_mcp "Use Guided Prompts via MCP" {
  persona  agent
  channels [mcp]
  priority medium
  tags     ["agent", "mcp"]
  features [mcp_prompts]
  flow     [
    "AI agent connects to SpecForge MCP server",
    "Agent invokes specforge://prompts/implement with an entity_id",
    "MCP returns implementation guidance with contract, related entities, and test expectations",
    "Agent invokes specforge://prompts/review with an entity_id for coverage analysis",
    "Agent invokes specforge://prompts/trace with a plan JSON for gap analysis",
    "Agent invokes specforge://prompts/explore with optional kind filter for graph discovery",
    "Success: agent receives pre-composed, context-rich workflows reducing multi-step tool composition",
  ]
}

// ── Developer + CLI (Migration) ──────────────────────────────

journey migrate_spec_files "Migrate Spec Files" {
  persona  developer
  priority low
  tags     ["developer", "migration"]
  features [spec_file_migration]
  flow     [
    "Developer upgrades specforge compiler to a new version",
    "Developer runs specforge migrate --dry-run to preview changes",
    "System shows unified diff of all proposed transformations",
    "Developer reviews diff and runs specforge migrate",
    "System backs up files and transforms to current format version",
    "System validates post-migration integrity automatically",
    "Success: all .spec files updated, graph structurally equivalent",
  ]
}
