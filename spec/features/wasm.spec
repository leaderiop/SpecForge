// Wasm/Extism package runtime features

use behaviors/wasm

feature wasm_package_runtime "Wasm Package Runtime" {
  behaviors [
    load_wasm_module, initialize_wasm_package, call_package_validators, call_package_generators,
    provide_host_function_query_graph, provide_host_function_emit_diagnostic,
    provide_host_function_register_entity, provide_host_function_register_edge,
    provide_host_function_emit_file, provide_host_function_http_get,
    enforce_wasm_sandbox, aot_compile_wasm_module, cache_aot_artifacts, warm_wasm_engine_instance,
    validate_package_peer_dependencies, topological_sort_packages,
    handle_wasm_trap, validate_package_manifest, configure_sandbox_policy,
    dispatch_contribution_exports, enforce_per_call_site_permissions, validate_contribution_exports,
  ]

  problem """
    Packages need a unified, sandboxed runtime that works across all
    platforms without requiring specific language runtimes (Node.js,
    Python, JVM) on the host machine. The runtime must support AOT
    compilation for fast cold starts and warm instances for interactive
    use in LSP/MCP contexts.
  """

  solution """
    Wasm/Extism as the sole package runtime. Packages compile to .wasm
    binaries and communicate with the compiler via host functions
    (specforge.query_graph, specforge.emit_diagnostic, specforge.register_entity,
    specforge.register_edge, specforge.emit_file, specforge.http_get).
    Sandbox enforcement via linear memory limits, fuel metering, and
    domain allowlists. AOT compilation cached in .specforge/cache/
    for CLI cold start; warm engine instances for LSP/MCP.
  """
}

feature wasm_package_authoring "Wasm Package Authoring" {
  behaviors [scaffold_wasm_package_project, build_wasm_package, test_wasm_package_locally, publish_wasm_package, discover_packages]

  problem """
    Package authors need a streamlined workflow to create, test, and
    publish Wasm packages. Without tooling, authors must manually
    configure build targets, sandbox policies, and registry publishing.
  """

  solution """
    specforge package CLI subcommands: init scaffolds a project with
    PDK skeleton, build compiles to .wasm targeting wasm32-wasi,
    test loads the binary in a production sandbox against fixtures,
    publish packages and uploads to npm/OCI/GitHub Releases.
  """
}

feature package_syntax_extensions "Package Syntax Extensions" {
  behaviors [provide_plugin_query_extensions, compose_query_files_from_plugins]

  problem """
    Package entities get no syntax highlighting because tree-sitter
    query files are hardcoded for the 16 built-in entity types. When
    a package registers a new entity type (e.g., constraint, threat),
    the grammar produces ERROR nodes and all editor features break:
    no highlighting, no code folding, no document symbols.
  """

  solution """
    Packages declare .scm query extensions in their manifest via the
    queryExtensions field. The LSP composes final query files by
    concatenating base queries with package extensions in load order.
    Query patterns are validated at package load time — invalid patterns
    produce warnings without blocking the package. Combined with the
    generic_entity_block grammar fallback (Tier 1) and semantic tokens
    (Tier 3), this provides full editor support for package entities.
  """
}

feature entity_enhancement "Entity Enhancement" {
  behaviors [
    load_package_manifest,
    register_entity_enhancements,
    detect_enhancement_conflicts,
    resolve_enhancement_conflicts,
    run_doctor_check,
  ]

  problem """
    Packages can add new entity types but cannot enhance existing entities
    with additional fields or edges. This blocks architectural packages
    (e.g., hexagonal architecture) that need to annotate core entities
    like behavior and port with domain-specific metadata.
  """

  solution """
    Packages declare entity enhancements in their sidecar manifest.json.
    The compiler loads enhancement declarations at startup, builds a
    FieldRegistry combining built-in and enhanced fields, and threads
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
    Wasm packages register new entity kinds during initialization but
    there is no mechanism to prevent name collisions. Two packages can
    register the same kind name, a package can shadow a built-in keyword,
    and define blocks can collide with package kinds — all silently.
  """

  solution """
    A 4-layer conflict prevention system: Layer 1 rejects reserved
    words at registration time, Layer 2 detects duplicates via a
    KindRegistry, Layer 3 resolves conflicts through config-driven
    policies (error/priority/namespace), and Layer 4 supports inline
    @plugin/kind qualification in the DSL for explicit disambiguation.
  """
}

feature wasm_package_lifecycle "Wasm Package Lifecycle" {
  behaviors [
    install_wasm_package, upgrade_wasm_package, invalidate_aot_cache, discover_packages,
    parse_package_specifier, resolve_package_source,
    write_lock_file, read_lock_file, verify_wasm_integrity,
  ]

  problem """
    There is no specification for package install, upgrade, and cache
    lifecycle management. Developers cannot install packages from sources,
    upgrade to newer versions, or manage the AOT compilation cache through
    documented, tested behaviors.
  """

  solution """
    Explicit lifecycle behaviors covering the full install/upgrade/cache
    workflow. Install resolves from multiple sources (registry, local, git),
    verifies integrity, and AOT compiles. Upgrade checks compatibility
    and handles breaking peer dependencies. Cache invalidation triggers on
    runtime version changes, manual clear, and binary changes. Lock file
    pins exact versions with SHA256 integrity hashes for reproducible builds.
  """
}

feature contribution_based_extensions "Contribution-Based Extensions" {
  behaviors [
    dispatch_contribution_exports,
    validate_contribution_exports,
    enforce_per_call_site_permissions,
    toggle_package_contributions,
    migrate_v1_manifest,
  ]

  problem """
    The PluginKind taxonomy forces each package into a single role
    (plugin, provider, or generator). A package like @specforge/jira
    that contributes entities, ref validation, AND code generation must
    be split into three separate packages — tripling distribution,
    versioning, and configuration overhead.
  """

  solution """
    Packages declare a contributes manifest key listing what they
    contribute: entities, validators, generators, providers. The
    compiler routes to namespaced Wasm exports based on contributions.
    Per-call-site permissions enforce least-privilege for each export.
    V1 manifests with PluginKind are auto-migrated with a W028 warning.
  """
}

feature package_source_resolution "Package Source Resolution" {
  behaviors [parse_package_specifier, resolve_package_source, discover_packages]

  problem """
    Packages need reproducible installation from multiple sources. Local
    development requires loading from filesystem paths without publishing.
    Teams need to share packages via git before a registry is available.
  """

  solution """
    Three source types with string shorthand and object form:
    registry (@scope/name@version), local (./path), and git
    (git:url#ref). The specifier format is parsed into a structured
    source descriptor and resolved to a concrete manifest + .wasm binary.
  """
}

feature package_version_management "Package Version Management" {
  behaviors [write_lock_file, read_lock_file, verify_wasm_integrity]

  problem """
    Without version pinning, builds are not reproducible. Two developers
    running specforge add at different times may get different package
    versions, leading to inconsistent behavior and hard-to-debug issues.
  """

  solution """
    specforge.lock pins exact resolved versions with SHA256 wasm_hash
    for each installed package. Lock file is deterministic — same inputs
    always produce byte-identical output. Integrity verification detects
    tampering. Explicit specforge update workflow for version bumps.
  """
}
