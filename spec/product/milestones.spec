phase 01 "Phase 1: Structural Parsing" {
  milestone        v0.1.0
  name             "Structural Parsing"
  slug             "structural-parsing"
  requirements     [FEATURE-123, FEATURE-023, FEATURE-019]
  success_criteria [
    "Tree-sitter grammar parses any keyword name { fields } block",
    "Multi-error recovery: N syntax errors produce N diagnostics, not 1",
    "highlights.scm, folds.scm and indents.scm load against the grammar and capture keywords, strings, folds and indents; VS Code highlights through a TextMate grammar and LSP semantic tokens",
    "Generic entity_block rule produces clean AST nodes for any keyword",
  ]
  goal             """
    Tree-sitter grammar and parser crate that turns .spec files into typed AST nodes with multi-error recovery.
  """
}

phase 02 "Phase 2: Resolution & Graph Construction" {
  milestone        v0.1.0
  name             "Resolution & Graph Construction"
  slug             "resolution-graph-construction"
  depends_on       [01]
  requirements     [FEATURE-109, FEATURE-047]
  success_criteria [
    "All intra-project references linked across files",
    "Import cycles detected and reported as W113 warnings, each cycle once",
    "Cross-extension refs produce I004 info if extension not installed",
    "One node per distinct entity ID (a duplicate is E002 or W060); one edge per distinct source, target and field reference",
    "Mutable graph supports incremental updates",
  ]
  goal             """
    Import resolution and mutable entity graph that links all intra-project references across files.
  """
}

phase 03 "Phase 3: Validation & Error Reporting" {
  milestone        v0.1.0
  name             "Validation & Error Reporting"
  slug             "validation-error-reporting"
  depends_on       [02]
  requirements     [FEATURE-124, FEATURE-017, FEATURE-006, FEATURE-105]
  success_criteria [
    "specforge check passes on SpecForge's own .spec files",
    "Diagnostics include source context with line/column spans",
    "Did-you-mean suggestions for misspelled entity IDs and imports (Levenshtein <= 3 and Jaro-Winkler > 0.85)",
    "Exit code 0 on clean, 1 on errors; --strict promotes warnings to errors",
    "Structured output (JSON) available for CI parsers",
  ]
  goal             """
    Structural and semantic validation with ariadne-powered diagnostic reporting and CI exit codes.
  """
}

phase 04 "Phase 4: Output & Agent Export" {
  milestone        v0.1.0
  name             "Output & Agent Export"
  slug             "output-agent-export"
  depends_on       [03]
  requirements     [FEATURE-057, FEATURE-130, FEATURE-001, FEATURE-101]
  success_criteria [
    "specforge schema --publish emits the Graph Protocol as JSON Schema (draft 2020-12); graph exports carry format_version 2.0 and a computed schema_version; specforge export --format=context produces token-optimized output",
    "specforge export --format=graph produces complete entity graph JSON; specforge export --format=brief produces id, kind and title per entity, plus edges",
    "Multi-resolution queries: export --scope <id> and query <id> --depth N [--kind K] return subgraphs; specforge trace prints full traceability chains",
    "specforge stats reports accurate entity/edge/unreferenced counts",
    "Output is deterministic: same input always produces same bytes",
  ]
  goal             """
    Graph serialization to JSON, DOT, and agent-optimized formats with multi-resolution queries and deterministic output.
  """
}

phase 05 "Phase 5: Project Initialization" {
  milestone        v0.1.0
  name             "Project Initialization"
  slug             "project-initialization"
  depends_on       [04]
  requirements     [FEATURE-106]
  success_criteria [
    "specforge init creates specforge.json and starter .spec file",
    "Full init > check > export pipeline completes in under 60 seconds",
    "Zero-extension project is valid and produces a graph",
    "Non-interactive mode works for CI: --name and --extensions flags",
    "specforge add @specforge/software adds extension to existing project",
  ]
  goal             """
    Project scaffolding via specforge init with interactive extension selection and specforge.json configuration.
  """
}

phase 01 "Phase 6: Incremental Compilation" {
  milestone        v0.5.0
  name             "Incremental Compilation"
  slug             "incremental-compilation"
  requirements     [FEATURE-050, FEATURE-051, FEATURE-142]
  success_criteria [
    "A file change is detected within 100ms plus the debounce window, and an incremental update of a small project yields diagnostics in under 100ms",
    "Incremental rebuild matches a cold rebuild, checked by --verify-incremental on every fixture and by seeded random update sequences",
    "Graph delta contains only added/removed/modified nodes and edges; specforge diff shows it between two snapshots and a pass can declare it as an input",
    "File change debouncing prevents redundant rebuilds",
    "Only changed files are re-parsed; imports are resolved again on every rebuild",
  ]
  goal             """
    File watching with debounced incremental rebuild, graph deltas, and minimal invalidation (only changed files re-parsed).
  """
}

phase 02 "Phase 7: LSP Server" {
  milestone        v0.5.0
  name             "LSP Server"
  slug             "lsp-server"
  depends_on       [01]
  requirements     [
    FEATURE-062,
    FEATURE-046,
    FEATURE-049,
    FEATURE-111,
    FEATURE-059,
    FEATURE-121,
    FEATURE-008,
    FEATURE-078,
  ]
  success_criteria [
    "Go-to-definition and find-references work across files; Hover shows entity details, contract text, and reference count",
    "Autocomplete suggests entity IDs, field names, and keywords; Rename updates declaration and all references atomically",
    "Live diagnostics appear within 100ms via shared incremental pipeline; Semantic tokens classify entity keywords from extensions",
    "Code actions: replace an unresolved ID or import with its close match, create an entity stub, add a verify stub",
    "Outline view and workspace symbol search work for all entity types",
  ]
  goal             """
    Full Language Server Protocol implementation with navigation, completion, refactoring, live diagnostics, and semantic tokens.
  """
}

phase 03 "Phase 8: Code Formatting" {
  milestone        v0.5.0
  name             "Code Formatting"
  slug             "code-formatting"
  requirements     [FEATURE-009, FEATURE-061]
  success_criteria [
    "format(format(x)) == format(x) on a representative input set and through stdin; All comments preserved after formatting",
    "A 50-entity file formats in under 50ms; specforge format --check exits 1 on unformatted files",
    "LSP textDocument/formatting produces same result as CLI",
    "Range formatting matches full formatting for affected blocks",
    "Files with parse errors are partially formatted without data loss",
  ]
  goal             """
    Idempotent code formatter with CST-preserving comment handling, CLI check mode, and LSP formatting integration.
  """
}

phase 04 "Phase 9: Zero-Entity Core Architecture" {
  milestone        v0.5.0
  name             "Zero-Entity Core Architecture"
  slug             "zero-entity-core-architecture"
  depends_on       [03]
  requirements     [
    FEATURE-016,
    FEATURE-029,
    FEATURE-018,
    FEATURE-026,
    FEATURE-027,
    FEATURE-140,
    FEATURE-141,
    FEATURE-021,
    FEATURE-098,
    FEATURE-024,
    FEATURE-025,
  ]
  success_criteria [
    "Core compiler has zero hardcoded entity types — all from extensions; KindRegistry boots empty and is populated only from the loaded extensions' ExtensionDeclarations; ExtensionDeclaration declares entity kinds (with testability), edge types, enhancements, validation rules, surfaces, collectors and passes",
    "@specforge/software (5), product (9) and governance (3) declare the 17 domain kinds SpecForge's own specs use; Two-phase compilation separates structural parsing from semantic validation",
    "E024 diagnostics suggest which extension provides unknown keywords; Graceful degradation with zero extensions produces I002 info",
    "Declarative validation patterns interpreted by core, not hardcoded passes; LSP highlights, completes, and navigates extension-defined entity types",
    "Enhancements from several extensions add their fields to a kind; a field two extensions add resolves to the first in specforge.json order, and never overrides the kind's own field; Third-party domain extensions work end-to-end",
  ]
  goal             """
    Core compiler refactored to have zero hardcoded entity types. All domain vocabulary comes from extensions via ExtensionDeclaration declarations.
  """
}

phase 01 "Phase 10: Wasm Extension Runtime" {
  milestone        v1.0.0
  name             "Wasm Extension Runtime"
  slug             "wasm-extension-runtime"
  requirements     [
    FEATURE-136,
    FEATURE-139,
    FEATURE-022,
    FEATURE-107,
    FEATURE-013,
    FEATURE-125,
    FEATURE-100,
    FEATURE-104,
    FEATURE-102,
    FEATURE-103,
  ]
  success_criteria [
    "Wasm extensions load, initialize, and validate without errors; Compiled components are cached on disk; a second runtime reuses the cache, and an unwritable cache degrades to no cache with a warning",
    "Sandbox enforcement blocks unauthorized filesystem and network access; Peer dependency validation catches missing or incompatible extensions",
    "Wasm traps produce structured diagnostics without crashing the compiler; Entity kind conflicts between extensions detected and reported",
    "Configured providers register ref schemes: a ref whose scheme no provider registers is I005, and a scheme two providers claim is E057; Contribution-based dispatch routes to correct exports per contribution type",
    "Surface contributions registered from the declaration's surfaces field; CLI commands auto-promoted to MCP tools with matching schemas",
  ]
  goal             """
    Wasm component runtime with compile caching, sandbox enforcement, peer dependency validation, and surface contribution dispatch. The host-port import surface is not part of it: it is Phase 15 (wasm_host_functions).
  """
}

phase 02 "Phase 11: Extension Ecosystem" {
  milestone        v1.0.0
  name             "Extension Ecosystem"
  slug             "extension-ecosystem"
  depends_on       [01]
  requirements     [
    FEATURE-028,
    FEATURE-134,
    FEATURE-138,
    FEATURE-135,
    FEATURE-133,
    FEATURE-030,
    FEATURE-110,
    FEATURE-128,
    FEATURE-089,
    FEATURE-083,
  ]
  success_criteria [
    "Full install/upgrade/remove lifecycle for Wasm extensions; specforge.lock pins exact versions with SHA256 integrity hashes",
    "Extension authoring: init > build > test > publish works e2e; Collectors produce specforge-report.json from test frameworks",
    "Registry search, resolve and publish work over the SpecForge HTTP package registry; specforge doctor reports conflicts, cache health, and extension status",
    "Private registry authentication: login keeps a registry credential under its registry root, logout removes it; Surface commands dispatched via cmd__{id} Wasm exports with sandbox enforcement",
    "Surface MCP tools/resources dispatched via mcp__{name} Wasm exports",
  ]
  goal             """
    Full extension lifecycle: install, upgrade, remove, author, build, test, publish. Registry integration and lock management.
  """
}

phase 03 "Phase 12: Graph Protocol Schema Versioning" {
  milestone        v1.0.0
  name             "Graph Protocol Schema Versioning"
  slug             "graph-protocol-schema-versioning"
  requirements     [FEATURE-120, FEATURE-048]
  success_criteria [
    "The schema is embedded in graph exports by default (opt out with --no-schema; opt in for context, brief or budgeted exports with --with-schema)",
    "Schema version is computed by diffing every attribute against the last export's cached schema (major for breaking, minor for additions, patch for documentation and provenance; 1.0.0 with no cache)",
    "Breaking changes warn W053 with the versions they move between",
    "--schema-version admits a version whose readers read the current schema and refuses others; the export is never relabelled",
    "specforge schema --publish emits a JSON Schema (draft 2020-12) for graph, context and brief exports",
  ]
  goal             """
    Self-describing graph protocol schema embedded in exports with version auto-computation, breaking change detection, and schema negotiation.
  """
}

phase 05 "Phase 13: MCP Server" {
  milestone        v0.5.0
  name             "MCP Server"
  slug             "mcp-server"
  requirements     [
    FEATURE-067,
    FEATURE-073,
    FEATURE-064,
    FEATURE-069,
    FEATURE-068,
    FEATURE-070,
    FEATURE-065,
    FEATURE-071,
    FEATURE-072,
    FEATURE-066,
  ]
  success_criteria [
    "MCP server initializes and shuts down cleanly per protocol spec; All 8 core resources (5 fixed, 3 templates) registered and return current graph state",
    "All 22 core and navigation tools respond with correct results; All 12 mutation and management tools execute operations successfully",
    "Subscribed clients get graph and diagnostics delta notifications when a request finds the project changed and the incremental update is applied; All 5 prompts (context, review, trace, explore, infer) return pre-composed workflows",
    "Protocol errors produce JSON-RPC error responses, not crashes",
    "Agents consume graph without CLI invocation",
  ]
  goal             """
    Model Context Protocol server exposing graph resources, core/navigation/mutation/project tools, delta notifications, and guided prompts.
  """
}
