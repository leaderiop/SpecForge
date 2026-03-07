// Wasm/Extism extension runtime features

use behaviors/wasm-authoring
use behaviors/wasm-extensions
use behaviors/wasm-host-functions
use behaviors/wasm-lifecycle
use behaviors/wasm-sandbox

feature wasm_extension_runtime "Wasm Extension Runtime" {
  behaviors [
    load_wasm_module, initialize_wasm_extension, call_extension_validators,
    validate_extension_peer_dependencies, topological_sort_extensions,
    handle_wasm_trap,
  ]

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
  behaviors [
    compute_extension_query_scope, provide_host_function_query_graph,
    provide_host_function_emit_diagnostic,
    provide_host_function_add_graph_node, provide_host_function_add_graph_edge,
    provide_host_function_emit_file, provide_host_function_http_get,
    enforce_wasm_sandbox, configure_sandbox_policy,
  ]

  problem """
    Extensions need controlled access to compiler internals (graph queries,
    diagnostic emission, entity registration) and external resources (file
    I/O, HTTP) without escaping the sandbox. Each host function needs
    specific permission scoping.
  """

  solution """
    Six host functions (specforge.query_graph, specforge.emit_diagnostic,
    specforge.add_graph_node, specforge.add_graph_edge, specforge.emit_file,
    specforge.http_get) with sandbox enforcement via linear memory limits,
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
  behaviors [
    aot_compile_wasm_module, cache_aot_artifacts, warm_wasm_engine_instance,
    evict_warm_engine_instance,
  ]

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
  behaviors [scaffold_wasm_extension_project, build_wasm_extension, validate_wasm_extension_locally, publish_wasm_extension]

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
  behaviors [provide_extension_query_extensions, compose_query_files_from_extensions]

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
  behaviors [
    load_extension_manifest,
    register_entity_enhancements,
    detect_enhancement_conflicts,
    resolve_enhancement_conflicts,
    run_doctor_check,
  ]

  problem """
    Extensions can add new entity types but cannot enhance existing entities
    with additional fields or edges. This blocks cross-cutting extensions
    that need to annotate entity kinds defined by other extensions with
    additional metadata.
  """

  solution """
    Extensions declare entity enhancements in their sidecar manifest.json.
    The compiler loads enhancement declarations at startup, builds a
    FieldRegistry combining extension-defined and enhanced fields, and threads
    it through the resolve/graph-build/validate pipeline. Enhanced
    reference fields create graph edges. Conflicts are detected at
    startup and resolved via configurable policies (error/priority/
    namespace) or explicit overrides in specforge.json. The specforge
    doctor command provides visibility into all enhancements and
    actionable conflict resolution.
  """
}

feature entity_kind_conflict_prevention "Entity Kind Conflict Prevention" {
  behaviors [
    reject_reserved_entity_kind,
    detect_entity_kind_collision,
    resolve_entity_kind_conflict_via_config,
    qualify_entity_kind_inline,
  ]

  problem """
    Wasm extensions register new entity kinds during initialization but
    there is no mechanism to prevent name collisions. Two extensions can
    register the same kind name, an extension can shadow a reserved keyword,
    and define blocks can collide with extension kinds — all silently.
  """

  solution """
    A 4-layer conflict prevention system: Layer 1 rejects reserved
    words at registration time, Layer 2 detects duplicates via a
    KindRegistry, Layer 3 resolves conflicts through config-driven
    policies (error/priority/namespace), and Layer 4 supports inline
    @extension/kind qualification in the DSL for explicit disambiguation.
  """
}

feature wasm_extension_installation "Wasm Extension Installation" {
  behaviors [
    install_wasm_extension, uninstall_wasm_extension, upgrade_wasm_extension,
    parse_extension_specifier, resolve_extension_source,
  ]

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
  behaviors [
    write_lock_file, read_lock_file, refresh_lock_file, verify_wasm_integrity,
  ]

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
  behaviors [
    discover_extensions, invalidate_aot_cache, update_all_extensions,
  ]

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
  behaviors [
    validate_extension_manifest,
    dispatch_contribution_exports,
    validate_contribution_exports,
    enforce_per_call_site_permissions,
    toggle_extension_contributions,
  ]

  problem """
    Extensions need a structured way to declare what they contribute
    (entities, validators, renderers, providers, collectors) with
    per-entity metadata such as testable flags, field type definitions,
    and structured validation rule patterns.
  """

  solution """
    Structured manifest format with typed objects (entity_kinds
    ManifestEntityKind[], validation_rules ValidationRulePattern[],
    edge_types ManifestEdgeType[]). Each extension declares a contributes
    key listing what it provides. The five contribution types are:
    entities (domain vocabulary), validators (graph validation rules),
    renderers (non-code outputs: reports, dashboards, traceability
    matrices), providers (ref validation), and collectors (test result
    ingestion). The compiler routes to namespaced Wasm exports based on
    contributions. Per-call-site permissions enforce least-privilege for
    each contribution export.
  """
}

feature test_result_collection "Test Result Collection" {
  behaviors [register_collector_contributions, auto_detect_collector, dispatch_collector, validate_collector_output, ingest_collector_report]

  problem """
    SpecForge traces test results but does not execute tests. Extensions need
    a formal contribution type for collectors that parse test runner output
    (JUnit XML, TAP, etc.) and map results back to spec entities. Without a
    collector contribution type, test result ingestion requires ad-hoc scripts
    outside the extension model, breaking Principle 5 (extensions over built-ins).
  """

  solution """
    Formal CollectorContribution type in extension manifests. Collectors
    declare input formats, auto-detection criteria, entity mapping strategies,
    and a Wasm export. The compiler registers collectors at startup, auto-detects
    the appropriate collector, dispatches it with entity IDs, validates the
    output against specforge-report/v1 schema, and ingests results into the
    graph with coverage metadata.
  """
}
