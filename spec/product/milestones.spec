// Milestones — delivery phases
//
// Rewritten from scratch with clear priority ordering based on:
// 1. Entity dependency DAG — foundational entities before dependent ones
// 2. Compilation pipeline order — parse > resolve > graph > validate > export
// 3. Vision horizons — H1 (individual value) > H2 (ecosystem) > H3 (standard)
// 4. Each phase ships something usable — no setup-only phases
//
// Phase dependency DAG (enforced via depends_on fields):
// H1: P1 > P2 > P3 > P4 > P5, P4 > P6 > P7, P4 > P8
// H2: P8 > P9 > P10 > P11, P4 > P12 > P13, P11 > P14

use "extensions/formal/features"
use "extensions/product/features"
use "extensions/software/features"
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
use "product/features"
use "product/modules"

// ════════════════════════════════════════════════════════════════
// H1: Individual Value — install to first validated output in <60s
// ════════════════════════════════════════════════════════════════

milestone structural_parsing "Phase 1: Structural Parsing" {
  description "Tree-sitter grammar and parser crate that turns .spec files into typed AST nodes with multi-error recovery."
  status      completed
  start_date  "2025-04-15"
  target_date "2025-06-01"
  owner       "specforge-team"
  contributors ["specforge-team"]
  features    [spec_file_parsing, error_recovery_during_parsing, editor_query_files]
  modules     [tree_sitter_specforge, specforge_parser]
  tags        ["h1", "core"]
  exit_criteria [
    "Tree-sitter grammar parses any keyword name { fields } block",
    "Multi-error recovery: N syntax errors produce N diagnostics, not 1",
    "Editor query files (highlights.scm, folds.scm, indents.scm) work in Neovim/VS Code",
    "Generic entity_block rule produces clean AST nodes for any keyword",
  ]
}

milestone resolution_and_graph "Phase 2: Resolution & Graph Construction" {
  description "Import resolution and mutable entity graph that links all intra-project references across files."
  status      completed
  start_date  "2025-06-01"
  target_date "2025-07-15"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [structural_parsing]
  features    [reference_resolution, graph_construction]
  modules     [specforge_resolver, specforge_graph]
  tags        ["h1", "core"]
  exit_criteria [
    "All intra-project references linked across files",
    "Import cycles detected and reported as E003",
    "Cross-extension refs produce I004 info if extension not installed",
    "Graph has one node per entity, one edge per reference",
    "Mutable graph supports incremental updates",
  ]
}

milestone validation_and_errors "Phase 3: Validation & Error Reporting" {
  description "Structural and semantic validation with ariadne-powered diagnostic reporting and CI exit codes."
  status      completed
  start_date  "2025-07-15"
  target_date "2025-08-30"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [resolution_and_graph]
  features    [structural_validation, diagnostic_reporting, ci_integration, product_validation]
  modules     [specforge_validator, specforge_cli]
  tags        ["h1", "core"]
  exit_criteria [
    "specforge check passes on SpecForge's own .spec files",
    "Diagnostics include source context with line/column spans",
    "Did-you-mean suggestions for misspelled entity IDs (Levenshtein <= 2)",
    "Exit code 0 on clean, 1 on errors; --strict promotes warnings to errors",
    "Structured output (JSON) available for CI parsers",
  ]
}

milestone output_and_export "Phase 4: Output & Agent Export" {
  description "Graph serialization to JSON, DOT, and agent-optimized formats with multi-resolution queries and deterministic output."
  status      completed
  start_date  "2025-08-30"
  target_date "2025-10-15"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [validation_and_errors]
  features    [json_and_dot_render, traceability_serialization, agent_export, product_graph_rendering]
  modules     [specforge_emitter]
  tags        ["h1", "core"]
  exit_criteria [
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

milestone project_init "Phase 5: Project Initialization" {
  description "Project scaffolding via specforge init with interactive extension selection and specforge.json configuration."
  status      completed
  start_date  "2025-10-15"
  target_date "2025-11-01"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [output_and_export]
  features    [project_initialization]
  modules     [specforge_cli]
  tags        ["h1", "platform"]
  exit_criteria [
    "specforge init creates specforge.json and starter .spec file",
    "Full init > check > export pipeline completes in under 60 seconds",
    "Zero-extension project is valid and produces a graph",
    "Non-interactive mode works for CI: --name and --extensions flags",
    "specforge add @specforge/software adds extension to existing project",
  ]
}

milestone ms_incremental_compilation "Phase 6: Incremental Compilation" {
  description "File watching with debounced incremental rebuild, graph deltas, and import DAG tracking for minimal invalidation."
  status      completed
  start_date  "2025-10-15"
  target_date "2025-11-15"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [output_and_export]
  features    [incremental_compilation, incremental_graph_deltas]
  modules     [specforge_watch]
  tags        ["h1", "core"]
  exit_criteria [
    "specforge watch delivers diagnostics within 100ms of file change",
    "Incremental rebuild matches cold rebuild (validated by property tests)",
    "Graph delta contains only added/removed/modified nodes and edges",
    "File change debouncing prevents redundant rebuilds",
    "Import DAG tracked incrementally for minimal invalidation",
  ]
}

milestone lsp_server "Phase 7: LSP Server" {
  description "Full Language Server Protocol implementation with navigation, completion, refactoring, live diagnostics, and semantic tokens."
  status      completed
  start_date  "2025-11-15"
  target_date "2025-12-15"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [ms_incremental_compilation]
  features [
    lsp_lifecycle,
    go_to_definition_and_references,
    hover_and_autocomplete,
    rename_refactoring,
    live_diagnostics,
    semantic_tokens,
    code_actions,
    outline_and_symbol_search,
  ]
  modules     [specforge_lsp]
  tags        ["h1", "platform"]
  exit_criteria [
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

milestone ms_code_formatting "Phase 8: Code Formatting" {
  description "Idempotent code formatter with CST-preserving comment handling, CLI check mode, and LSP formatting integration."
  status      completed
  start_date  "2025-10-15"
  target_date "2025-11-30"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [output_and_export]
  features    [code_formatting, lsp_formatting]
  modules     [specforge_formatter]
  tags        ["h1", "tooling"]
  exit_criteria [
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

milestone zero_entity_core "Phase 9: Zero-Entity Core Architecture" {
  description "Core compiler refactored to have zero hardcoded entity types. All domain vocabulary comes from extensions via ManifestV2 declarations."
  status      completed
  start_date  "2025-12-01"
  target_date "2026-01-15"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [ms_code_formatting]
  features [
    declarative_validation_rules,
    extension_manifest,
    dynamic_entity_registration,
    extension_driven_lsp,
    extension_driven_visualization,
    zero_entity_bootstrap,
    zero_entity_validation,
    entity_enhancement,
    product_entity_registration,
  ]
  modules     [specforge_wasm]
  tags        ["h2", "architecture"]
  exit_criteria [
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

milestone wasm_runtime "Phase 10: Wasm Extension Runtime" {
  description "Wasm/Extism runtime with AOT caching, sandbox enforcement, host function API, peer dependency validation, and surface contribution dispatch."
  status      completed
  start_date  "2026-01-15"
  target_date "2026-02-01"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [zero_entity_core]
  features [
    wasm_extension_runtime,
    wasm_host_function_api,
    wasm_performance_optimization,
    entity_kind_conflict_prevention,
    provider_based_ref_validation,
    contribution_based_extensions,
    extension_query_contributions,
    surface_contributions,
    product_graph_queries,
    product_surface_access,
    product_health_metric,
    product_impact_and_whatif,
    product_graph_diff,
  ]
  modules     [specforge_wasm]
  tags        ["h2", "runtime"]
  exit_criteria [
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

milestone extension_ecosystem "Phase 11: Extension Ecosystem" {
  description "Full extension lifecycle: install, upgrade, remove, author, build, test, publish. Registry integration, lock management, and grammar contributions."
  status      completed
  start_date  "2026-02-01"
  target_date "2026-02-28"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [wasm_runtime]
  features [
    extension_management,
    wasm_extension_installation,
    wasm_lock_management,
    wasm_extension_maintenance,
    wasm_extension_authoring,
    extension_registry,
    registry_authentication,
    test_result_collection,
    wasm_grammar_contributions,
    extension_body_parsing,
    pe_planning_insights,
    pe_external_blockers,
    fa_progressive_warnings,
  ]
  tags        ["h2", "ecosystem"]
  exit_criteria [
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

milestone software_extension_v1 "Phase 11a: @specforge/software Extension v1" {
  description "First-party domain extension implementing behavior, invariant, event, type, and port entity kinds for software engineering specifications."
  status      in_progress
  start_date  "2026-03-01"
  target_date "2026-04-15"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [zero_entity_core, wasm_runtime]
  features    [se_core_entity_kinds, se_validation_suite, se_gherkin_bridge]
  modules     [specforge_package_software]
  tags        ["h2", "extension"]
  exit_criteria [
    "manifest.json declares 5 entity kinds with all fields, testability, and LSP metadata",
    "manifest.json declares 11 edge types with source/target constraints",
    "Validation rules W001-W010, E004, E006, E010, E016 fire correctly",
    "Entity enhancements add ports and behaviors fields to product entities",
    "specforge check with @specforge/software loaded produces zero false positives on own .spec files",
    "All verify statements have corresponding test implementations",
  ]
}

milestone schema_versioning "Phase 12: Graph Protocol Schema Versioning" {
  description "Self-describing graph protocol schema embedded in exports with version auto-computation, breaking change detection, and schema negotiation."
  status      completed
  start_date  "2025-10-15"
  target_date "2025-11-30"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [output_and_export]
  features    [self_describing_graph_protocol, graph_protocol_versioning]
  modules     [specforge_emitter, specforge_cli]
  tags        ["h2", "schema"]
  exit_criteria [
    "Self-describing schema embedded in every graph export",
    "Schema version auto-computed from registry contents",
    "Breaking changes detected when entity kinds or edge types are removed",
    "Schema negotiation allows consumers to request specific versions",
    "JSON Schema specification published alongside graph exports",
  ]
}

milestone mcp_server "Phase 13: MCP Server" {
  description "Model Context Protocol server exposing graph resources, core/navigation/mutation/project tools, delta notifications, and guided prompts."
  status      completed
  start_date  "2025-12-15"
  target_date "2026-01-31"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [schema_versioning]
  modules     [specforge_mcp]
  features [
    mcp_lifecycle,
    mcp_resource_exposure,
    mcp_core_tools,
    mcp_navigation_tools,
    mcp_mutation_tools,
    mcp_project_management_tools,
    mcp_delta_notifications,
    mcp_prompts,
    mcp_protocol_compliance,
    mcp_discovery,
  ]
  tags        ["h2", "platform"]
  exit_criteria [
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

milestone migration "Phase 14: Migration" {
  description "Spec file migration with dry-run preview, backup, post-migration validation, rollback, and extension migration hook invocation."
  status      planned
  start_date  "2026-05-01"
  target_date "2026-06-30"
  owner       "specforge-team"
  contributors ["specforge-team"]
  depends_on  [extension_ecosystem]
  features    [spec_file_migration]
  modules     [specforge_cli, specforge_wasm]
  tags        ["h3", "tooling"]
  exit_criteria [
    "specforge migrate --dry-run shows unified diff of all proposed changes",
    "Backup created before in-place transformation",
    "Post-migration validation confirms graph structural equivalence",
    "Rollback restores original files on migration failure",
    "Extension migration hooks invoked in topological order",
  ]
}
