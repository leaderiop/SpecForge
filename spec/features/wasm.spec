// Wasm/Extism extension runtime features

use "behaviors/wasm-authoring"
use "behaviors/wasm-extensions"
use "behaviors/wasm-host-functions"
use "behaviors/wasm-lifecycle"
use "behaviors/wasm-sandbox"
use "behaviors/surface-contributions"
feature wasm_extension_runtime "Wasm Extension Runtime" {

  problem """
    Extensions need a unified runtime that works across all platforms
    without requiring specific language runtimes (Node.js, Python, JVM)
    on the host machine. The runtime must handle module lifecycle,
    dependency ordering, and graceful error recovery.
  """

  solution """
    Wasm/Extism as the sole extension runtime. Extensions compile to .wasm
    binaries. The compiler loads modules, validates peer dependencies,
    initializes in topological order, calls validators, and handles traps
    gracefully — failed extensions do not affect others.
  """
}

feature wasm_host_function_api "Wasm Host Function API" {

  problem """
    Extensions need controlled access to compiler internals (graph queries,
    diagnostic emission, entity registration) and external resources (file
    I/O, HTTP) without escaping the sandbox. Each host function needs
    specific permission scoping.
  """

  solution """
    Seven host functions (specforge.query_graph, specforge.emit_diagnostic,
    specforge.add_graph_node, specforge.add_graph_edge, specforge.read_file,
    specforge.emit_file, specforge.http_get) plus three supporting behaviors
    (compute_extension_query_scope for extension-scoped graph views,
    enforce_wasm_sandbox and configure_sandbox_policy for sandbox enforcement)
    providing linear memory limits,
    fuel metering, filesystem restrictions, and domain allowlists. The
    emit_file host function is restricted to non-code outputs only (reports,
    dashboards, traceability matrices, graph visualizations) — extensions
    MUST NOT use it to generate source code, configuration files, or
    executable artifacts. SpecForge provides context, agents produce code.
    Sandbox policy is computed by merging defaults, manifest, and project
    overrides. Host functions are synchronous leaf operations and
    intentionally produce no events — traceability comes from the calling
    behavior's events. Debug-level tracing of individual host function
    calls is available via the sandbox fuel metering counters reported in
    extension lifecycle diagnostics.
  """
}

feature wasm_performance_optimization "Wasm Performance Optimization" {

  problem """
    Cold-loading .wasm binaries on every compilation is too slow for CLI
    and unacceptable for interactive LSP/MCP contexts. Extensions need
    fast startup without sacrificing sandbox isolation.
  """

  solution """
    AOT compilation cached in .specforge/cache/ using content-addressed
    filenames for CLI cold start performance. Warm engine instances kept
    alive across compilations for LSP/MCP interactive use. Cache integrity
    verified by re-hashing on load.
  """
}

feature wasm_extension_authoring "Wasm Extension Authoring" {

  problem """
    Extension authors need a streamlined workflow to create, test, and
    publish Wasm extensions. Without tooling, authors must manually
    configure build targets, sandbox policies, and registry publishing.
  """

  solution """
    specforge extension CLI subcommands: init scaffolds a project with
    PDK skeleton, build compiles to .wasm targeting wasm32-wasi,
    validate loads the binary in a production sandbox against fixtures,
    publish uploads to npm/OCI/GitHub Releases. Publishing adheres to
    the registry_api_openness invariant — the registry API specification
    is published as an open standard.
  """
}

feature extension_query_contributions "Extension Query Contributions" {

  problem """
    While the generic entity_block rule handles all entity blocks
    uniformly, extensions may want custom syntax highlighting patterns
    for their entity keywords. Without query extensions, all extension
    entities share the same default highlighting.
  """

  solution """
    Extensions declare .scm query extensions in their manifest via the
    queryExtensions field. The LSP composes final query files by
    concatenating base queries with extension extensions in load order.
    Query patterns are validated at extension load time — invalid patterns
    produce warnings without blocking the extension. Combined with the
    generic entity_block grammar rule (Tier 1) and semantic tokens
    (Tier 3), this provides rich editor support for extension entities.
  """
}

feature entity_enhancement "Entity Enhancement" {
  // Bridge: depends on validate_extension_manifest (contribution_based_extensions feature)
  // for manifest schema validation before enhancement registration proceeds.

  problem """
    Extensions can add new entity types but cannot enhance existing entities
    with additional fields or edges. This blocks cross-cutting extensions
    that need to annotate entity kinds defined by other extensions with
    additional metadata.
  """

  solution """
    Extensions declare entity enhancements in their sidecar manifest.json.
    Manifest validation (validate_extension_manifest) runs before
    enhancement loading — owned by contribution_based_extensions.
    The compiler loads enhancement declarations at startup, builds a
    FieldRegistry combining extension-defined and enhanced fields, and threads
    it through the resolve/graph-build/validate pipeline. Enhanced
    reference fields create graph edges. Conflicts are detected at
    startup and resolved via the error policy (hard fail) or explicit
    overrides in specforge.json. Additional policies (priority, namespace)
    are deferred to a future phase. The specforge doctor command provides
    visibility into all enhancements and actionable conflict resolution.
  """
}

feature entity_kind_conflict_prevention "Entity Kind Conflict Prevention" {

  problem """
    Wasm extensions register new entity kinds during initialization but
    there is no mechanism to prevent name collisions. Two extensions can
    register the same kind name, an extension can shadow a reserved keyword,
    and define blocks can collide with extension kinds — all silently.
  """

  solution """
    A 2-layer conflict prevention system: Layer 1 rejects reserved
    words at registration time, Layer 2 rejects any extension whose
    manifest declares a kind already registered by a previously loaded
    extension — duplicate kinds are a hard error at startup.
  """
}

feature wasm_extension_installation "Wasm Extension Installation" {

  problem """
    Extensions need a reliable install/uninstall/upgrade workflow that
    resolves from multiple sources (registry, local, git) and maintains
    project configuration integrity.
  """

  solution """
    Install resolves from multiple sources, verifies integrity, and AOT
    compiles. Uninstall removes the extension, invalidates caches, and
    checks peer dependencies. Upgrade checks compatibility and handles
    breaking peer dependencies.
  """
}

feature wasm_lock_management "Wasm Lock Management" {

  problem """
    Reproducible builds require pinning exact extension versions with
    integrity verification. Without a lock file, different environments
    may resolve different extension versions.
  """

  solution """
    Lock file (specforge.lock) pins exact versions with SHA256 integrity
    hashes for reproducible builds. Lock file is read at startup to verify
    installed extensions match expected hashes. Refresh updates the lock
    file when extensions are added or upgraded.
  """
}

feature wasm_extension_maintenance "Wasm Extension Maintenance" {

  problem """
    Extension ecosystems need discovery, cache management, and bulk
    update capabilities. Without these, users must manage extensions
    individually and manually clear stale caches.
  """

  solution """
    Discovery queries registries for available extensions. AOT cache
    invalidation triggers on runtime version changes, manual clear,
    and binary changes. Bulk update checks all extensions for newer
    versions and upgrades them in dependency order.
  """
}

feature contribution_based_extensions "Contribution-Based Extensions" {

  problem """
    Extensions need a structured way to declare what they contribute
    (entities, validators, renderers, providers, parsers, collectors,
    grammars, body_parsers, verify_kinds) with per-entity metadata such
    as testable flags, field type definitions, and structured validation
    rule patterns.
  """

  solution """
    Structured manifest format with typed objects (entity_kinds
    ManifestEntityKind[], validation_rules ValidationRulePattern[],
    edge_types ManifestEdgeType[]). Each extension declares a contributes
    key listing what it provides. The nine contribution types are:
    entities (domain vocabulary), validators (graph validation rules),
    renderers (non-code diagnostic artifacts as enforced by the emit_file
    allowlist), providers (ref validation), parsers (domain-specific file
    parsing — see ADR extension_file_parsers), and collectors (test result
    ingestion — see test_result_collection feature for collector
    behaviors). The compiler routes to namespaced Wasm exports based on
    contributions. Per-call-site permissions enforce least-privilege for
    each contribution export. This feature owns compile-time contribution
    dispatch (entities, validators, renderers, providers, parsers, grammars,
    body_parsers). Test result collection (collectors) is owned by the
    test_result_collection feature. Surface contributions (CLI commands,
    MCP tools, MCP resources) are owned by the surface_contributions feature.
    Collector behaviors (register_collector_contributions,
    auto_detect_collector, dispatch_collector, validate_collector_output,
    ingest_collector_report) are owned by the test_result_collection feature.
    The eight dispatch contribution types and their feature owners:
    1-5. entities, validators, renderers, providers, parsers — this feature.
    6. collectors — test_result_collection feature.
    7-8. grammars, body_parsers — wasm_grammar_contributions feature.
    Additionally, verify_kinds is a declarative manifest field (no Wasm dispatch).
    Cross-feature dependency: dispatch_contribution_exports consumes
    collector_report_ingested from the test_result_collection feature,
    re-rendering outputs after new evidence is ingested.
  """
}

feature test_result_collection "Test Result Collection" {

  problem """
    SpecForge traces test results but does not execute tests. Extensions need
    a formal contribution type for collectors that parse test runner output
    (structured output from external tools — format-specific parsing is
    extension-owned) and map results back to spec entities. Without a
    collector contribution type, test result ingestion requires ad-hoc scripts
    outside the extension model, breaking Principle 7 (extensions over built-ins).
  """

  solution """
    Formal CollectorContribution type in extension manifests. Collectors
    declare input formats, auto-detection criteria, entity mapping strategies,
    and a Wasm export. The compiler registers collectors at startup, auto-detects
    the appropriate collector, dispatches it with entity IDs, validates the
    output against specforge-report/v1 schema, and ingests results into the
    graph with coverage metadata.
    The specforge collect CLI entry point is an extension-owned surface
    contribution (P7: extensions over built-ins). Extensions providing
    collectors declare a collect command in their manifest's surfaces.commands
    array, dispatched via the surface_contributions feature.
  """
}

feature surface_contributions "Surface Contributions" {

  problem """
    Extensions can extend the compilation pipeline (entities, validators,
    renderers, providers, collectors, grammars, body_parsers) but cannot
    extend the tooling surfaces — CLI and MCP server. This creates a
    capability asymmetry: domain extensions need CLI commands (e.g., analyze, audit) but has no registration mechanism; extensions cannot register
    MCP tools so agents only see core tools.
  """

  solution """
    Extensions declare surface contributions in their manifest's surfaces
    field: commands[] for CLI, mcp_tools[] for MCP tools, mcp_resources[]
    for MCP resources. Core discovers contributions from manifests (static,
    no code execution), validates Wasm exports exist, and dispatches lazily
    on invocation. CLI commands are auto-promoted to MCP tools with the
    specforge.{ext}.{cmd} naming convention. Per-contribution sandbox
    overrides can only restrict below the type ceiling (MCP resources
    cannot fs_write). Phase 1 covers CLI commands, MCP tools, and MCP
    resources. LSP providers are deferred to Phase 2.
  """
}

feature wasm_grammar_contributions "Wasm Grammar Contributions" {
  // Bridge: invariants grammar_composition_determinism and body_parser_output_conformance
  // reference register_grammar_contributions and register_body_parser_contributions in
  // enforced_by, but those behaviors belong to dynamic_entity_registration feature in
  // features/zero-entity-core.spec. This is a cross-feature dependency, similar to
  // entity_enhancement's dependency on validate_extension_manifest.
  refs      [register_grammar_contributions, register_body_parser_contributions]

  problem """
    Extensions needing structured syntax beyond key-value pairs must push
    data into opaque strings the compiler cannot validate. The fixed
    tree-sitter grammar cannot accommodate extension-specific body syntax,
    violating zero-domain-knowledge and validation-is-the-value principles.
  """

  solution """
    Two new contribution types: grammars (tree-sitter .wasm for editor
    highlighting) and body_parsers (Wasm exports that parse raw body text
    into structured JSON fields). Core grammar captures keyword name
    { raw_body } and delegates parsing to extensions via Phase 1.5.
    Grammar artifacts are cached using content-hash + ABI version keys.
    Conflict resolution is configurable via GrammarConflictPolicy.
  """
}
