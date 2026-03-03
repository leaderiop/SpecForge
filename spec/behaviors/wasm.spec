// Wasm/Extism package runtime behaviors

use invariants/wasm
use types/wasm
use types/config
use types/errors
use ports/outbound

// ── Wasm Module Lifecycle ─────────────────────────────────────

behavior load_wasm_module "Load Wasm Module" {
  invariants [wasm_sandbox_integrity]
  types      [PackageManifest, WasmModuleCache, PackageError]
  ports      [WasmRuntime]

  contract """
    When the compiler loads a plugin, it MUST locate the .wasm binary
    from the manifest's wasmPath, check the AOT cache for a pre-compiled
    module matching the content hash, and load it into the Extism runtime.
    Cache hits MUST skip recompilation. Missing .wasm files MUST produce
    a PackageError diagnostic.
  """

  verify unit "loads .wasm binary from manifest path"
  verify unit "uses AOT cache on cache hit"
  verify unit "missing .wasm produces PackageError"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior initialize_wasm_package "Initialize Wasm Package" {
  invariants [peer_dependency_satisfaction]
  types      [PackageManifest, PluginLifecycleState]
  ports      [WasmRuntime]

  contract """
    After loading a Wasm module, the compiler MUST call the package's
    initialize() export. During initialization, the package registers
    its entity types via specforge.register_entity and its edge types
    via specforge.register_edge. The package lifecycle MUST transition
    from loading to initialized on success, or to failed on error.
  """

  verify unit "calls initialize() export on loaded module"
  verify unit "entity types registered via host function"
  verify unit "lifecycle transitions to initialized on success"
  verify unit "lifecycle transitions to failed on error"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior call_package_validators "Call Package Validators" {
  invariants [plugin_load_order_determinism]
  types      [PackageManifest, PluginLifecycleState]
  ports      [WasmRuntime]

  contract """
    After all packages are initialized, the compiler MUST call each
    package's validate() export in topological order determined by
    peer dependencies. Packages MUST emit diagnostics via the
    specforge.emit_diagnostic host function. The compiler MUST
    collect all diagnostics and continue to the next package.
  """

  verify unit "calls validate() in topological order"
  verify unit "diagnostics emitted via host function are collected"
  verify unit "validation continues to next package after errors"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior call_package_generators "Call Package Generators" {
  types      [PackageManifest, GenConfig, PluginLifecycleState]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge gen invokes a generator package, the compiler MUST
    call the package's generate() export, passing the serialized graph
    via the specforge.query_graph host function. Generated files MUST
    be collected via specforge.emit_file. Package traps MUST produce
    a PackageError diagnostic without crashing the compiler.
  """

  verify unit "calls generate() export on the package"
  verify unit "graph is available via query_graph host function"
  verify unit "generated files collected via emit_file"
  verify unit "package trap produces PackageError"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Host Functions ────────────────────────────────────────────

behavior provide_host_function_query_graph "Provide Host Function: query_graph" {
  invariants [wasm_sandbox_integrity, host_function_type_safety]
  types      [HostFunctionBinding]
  ports      [WasmRuntime]

  contract """
    The specforge.query_graph host function MUST expose the compiled
    graph as a JSON string to the calling plugin. The graph MUST include
    all entities, edges, and metadata accessible to the plugin based
    on its declared scope.
  """

  verify unit "query_graph returns valid JSON graph"
  verify unit "graph includes entities and edges"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior provide_host_function_emit_diagnostic "Provide Host Function: emit_diagnostic" {
  invariants [host_function_type_safety]
  types      [HostFunctionBinding]
  ports      [WasmRuntime]

  contract """
    The specforge.emit_diagnostic host function MUST accept a diagnostic
    object with severity, code, message, and optional source span. The
    diagnostic MUST be added to the compiler's diagnostic collection and
    rendered like built-in diagnostics.
  """

  verify unit "emit_diagnostic adds to compiler diagnostic collection"
  verify unit "plugin diagnostics rendered like built-in diagnostics"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior provide_host_function_register_entity "Provide Host Function: register_entity" {
  invariants [host_function_type_safety]
  types      [HostFunctionBinding, PackageManifest]
  ports      [WasmRuntime]

  contract """
    The specforge.register_entity host function MUST accept an entity
    type registration with name, required fields, optional fields, and
    reference targets. The registered entity type MUST participate in
    parsing, resolution, and validation like built-in entity types.
  """

  verify unit "registered entity type is parseable"
  verify unit "registered entity participates in resolution"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior provide_host_function_register_edge "Provide Host Function: register_edge" {
  invariants [host_function_type_safety]
  types      [HostFunctionBinding]
  ports      [WasmRuntime]

  contract """
    The specforge.register_edge host function MUST accept an edge type
    registration with name, source entity kind, and target entity kind.
    The registered edge type MUST participate in reference resolution
    and graph construction.
  """

  verify unit "registered edge type is used in resolution"
  verify unit "registered edge appears in graph"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior provide_host_function_emit_file "Provide Host Function: emit_file" {
  invariants [wasm_sandbox_integrity, host_function_type_safety]
  types      [HostFunctionBinding, SandboxPolicy]
  ports      [WasmRuntime, FileSystem]

  contract """
    The specforge.emit_file host function MUST accept a file path and
    content from the plugin. The path MUST be validated against the
    sandbox policy — writes outside the allowed output directory MUST
    be rejected with a diagnostic. Valid files MUST be written to disk.
  """

  verify unit "valid file path within output directory is written"
  verify unit "file path outside output directory is rejected"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior provide_host_function_http_get "Provide Host Function: http_get" {
  invariants [wasm_sandbox_integrity, host_function_type_safety]
  types      [HostFunctionBinding, SandboxPolicy]
  ports      [WasmRuntime]

  contract """
    The specforge.http_get host function MUST fetch a URL and return
    the response body to the calling plugin. The URL MUST be validated
    against the sandbox policy's allowed domains. Requests to disallowed
    domains MUST be rejected. Timeouts MUST be enforced.
  """

  verify unit "allowed domain returns response body"
  verify unit "disallowed domain is rejected"
  verify unit "timeout is enforced"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Sandbox & Performance ─────────────────────────────────────

behavior enforce_wasm_sandbox "Enforce Wasm Sandbox" {
  invariants [wasm_sandbox_integrity]
  types      [SandboxPolicy, PackageError]
  ports      [WasmRuntime]

  contract """
    The runtime MUST enforce the sandbox policy for each plugin: memory
    limits via Extism's linear memory cap, execution time limits via
    fuel metering, filesystem restrictions via host function validation,
    and network restrictions via domain allowlists. Violations MUST
    trap the plugin and emit a diagnostic.
  """

  verify unit "memory limit enforced via linear memory cap"
  verify unit "execution time limit enforced via fuel metering"
  verify unit "filesystem restriction enforced"
  verify unit "network restriction enforced"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior aot_compile_wasm_module "AOT Compile Wasm Module" {
  types      [WasmModuleCache, PackageManifest]
  ports      [WasmRuntime, FileSystem]

  contract """
    On first load of a .wasm binary, the runtime MUST AOT compile the
    module and cache the compiled artifact in .specforge/cache/ using
    a content-hash filename. Subsequent loads MUST use the cached
    artifact to reduce cold start time.
  """

  verify unit "first load triggers AOT compilation"
  verify unit "compiled artifact cached with content-hash filename"
  verify unit "subsequent load uses cached artifact"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior cache_aot_artifacts "Cache AOT Artifacts" {
  types      [WasmModuleCache]
  ports      [FileSystem]

  contract """
    The AOT content-addressed cache MUST store entries using filenames
    derived from the SHA256 of the .wasm binary. The cache MUST detect
    corruption by re-hashing on load. Corrupted entries MUST be evicted
    and recompiled. The cache directory MUST be .specforge/cache/.
  """

  verify unit "cache entries use content-addressed filenames"
  verify unit "corrupted cache entry is evicted and recompiled"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior warm_wasm_engine_instance "Warm Wasm Engine Instance" {
  types      [PluginLifecycleState]
  ports      [WasmRuntime]

  contract """
    For LSP and MCP server contexts, the runtime MUST keep initialized
    Wasm engine instances warm across compilations. Warm instances MUST
    be reused for subsequent validate() and generate() calls. Instances
    MUST be unloaded when the plugin is removed or the server shuts down.
  """

  verify unit "warm instance reused across compilations"
  verify unit "instance unloaded on plugin removal"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Dependencies ──────────────────────────────────────────────

behavior validate_package_peer_dependencies "Validate Package Peer Dependencies" {
  invariants [peer_dependency_satisfaction]
  types      [PeerDependency, PackageManifest, PackageError]

  contract """
    Before initializing packages, the compiler MUST check that all
    declared peer dependencies are satisfied. For each peer dependency,
    the referenced package MUST be installed and its version MUST match
    the declared semver range. Unsatisfied peers MUST produce a hard
    error diagnostic.
  """

  verify unit "satisfied peer dependency passes"
  verify unit "missing peer produces hard error"
  verify unit "version mismatch produces hard error"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior topological_sort_packages "Topological Sort Packages" {
  invariants [plugin_load_order_determinism]
  types      [PeerDependency, PackageManifest]

  contract """
    The compiler MUST compute a topological order over installed packages
    based on their peer dependencies. Packages with no dependencies MUST
    be loaded first. Cycles in peer dependencies MUST produce an error
    diagnostic. The sort MUST be deterministic — ties broken by package name.
  """

  verify unit "packages sorted in dependency order"
  verify unit "cycle in peer dependencies produces error"
  verify unit "deterministic ordering on ties"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Package Authoring ─────────────────────────────────────────

behavior scaffold_wasm_package_project "Scaffold Wasm Package Project" {
  types      [PackageManifest]
  ports      [FileSystem]

  contract """
    When specforge package init is invoked, the system MUST scaffold a
    new Wasm package project with: a manifest file, a src/ directory
    with a skeleton implementing initialize/validate/generate exports,
    a build script targeting wasm32-wasi, and a README with PDK docs.
  """

  verify unit "scaffold creates manifest file"
  verify unit "scaffold creates src/ with skeleton exports"
  verify unit "scaffold creates build script for wasm32-wasi"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior build_wasm_package "Build Wasm Package" {
  types      [PackageManifest]
  ports      [FileSystem]

  contract """
    When specforge package build is invoked, the system MUST compile
    the package source to a .wasm binary using the configured toolchain.
    The output .wasm MUST be placed alongside the manifest. Build
    errors MUST be reported as diagnostics.
  """

  verify unit "build produces .wasm binary"
  verify unit "build errors reported as diagnostics"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior test_wasm_package_locally "Test Wasm Package Locally" {
  types      [PackageManifest, SandboxPolicy]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge package test is invoked, the system MUST load the
    locally built .wasm binary, execute it against test fixtures, and
    report results. The package MUST run in the same sandbox as production
    to catch permission errors early.
  """

  verify unit "test loads local .wasm binary"
  verify unit "test runs against fixtures"
  verify unit "test uses production sandbox policy"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior publish_wasm_package "Publish Wasm Package" {
  types      [PackageManifest]
  ports      [FileSystem]

  contract """
    When specforge package publish is invoked, the system MUST package
    the .wasm binary and manifest, then publish to the configured
    registry (npm, OCI, or GitHub Releases). The manifest MUST be
    validated before publishing.
  """

  verify unit "publish packages .wasm and manifest"
  verify unit "manifest validated before publish"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Query Extensions ─────────────────────────────────────────

behavior provide_plugin_query_extensions "Provide Plugin Query Extensions" {
  types      [PackageManifest, QueryExtension, QueryFileKind, PackageError]
  ports      [WasmRuntime]

  contract """
    When a plugin manifest declares queryExtensions, the compiler
    MUST extract the .scm query patterns and make them available to
    the LSP and editor tooling. Query patterns MUST be validated for
    syntax correctness at plugin load time by parsing them with
    tree_sitter::Query::new(). Invalid patterns MUST produce a
    warning diagnostic without blocking plugin loading. Valid patterns
    MUST be stored alongside the plugin's registration data for
    retrieval during query composition.
  """

  verify unit "valid query extension stored in plugin registration"
  verify unit "invalid query pattern produces warning diagnostic"
  verify unit "invalid pattern does not block plugin loading"
  verify unit "query extensions extracted from manifest"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior compose_query_files_from_plugins "Compose Query Files From Plugins" {
  invariants [plugin_load_order_determinism]
  types      [QueryExtension, QueryFileKind]
  ports      [WasmRuntime]

  contract """
    The LSP MUST compose final query files by concatenating base
    queries with plugin query extensions in plugin load order. The
    composition MUST follow the string concatenation pattern: base
    queries first, extensions appended. Plugin patterns with #match?
    predicates for entity keywords MUST work correctly in the composed
    query. The composed query MUST be re-validated after concatenation
    to catch cross-pattern conflicts. Composition MUST be deterministic
    — the same set of plugins always produces the same final query.
  """

  verify unit "base queries come first in composed output"
  verify unit "plugin extensions appended in load order"
  verify unit "#match? predicates work in composed query"
  verify unit "composition is deterministic across runs"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Entity Kind Conflict Prevention ──────────────────────────

behavior reject_reserved_entity_kind "Reject Reserved Entity Kind" {
  invariants [entity_kind_uniqueness]
  types      [HostFunctionBinding, PackageManifest]
  ports      [WasmRuntime]

  contract """
    The specforge.register_entity host function MUST reject entity
    kind names that match any of the 16 built-in keywords (spec,
    invariant, behavior, feature, event, type, port, ref, capability,
    deliverable, roadmap, library, glossary, decision, constraint,
    failure_mode) or DSL syntax words (define, use, verify, scenario,
    given, when, then, true, false). Rejection MUST return an error
    to the calling plugin before the kind is registered.
  """

  verify unit "rejects built-in keyword 'behavior'"
  verify unit "rejects DSL syntax word 'define'"
  verify unit "accepts valid custom kind name"
  verify unit "rejects invalid identifier characters"

  tests ["../crates/specforge-wasm/src/host_functions.rs"]
}

behavior detect_entity_kind_collision "Detect Entity Kind Collision" {
  invariants [entity_kind_uniqueness]
  types      [PackageManifest, PackageError]

  contract """
    The KindRegistry MUST detect when two plugins attempt to register
    the same entity kind name. Collisions with built-in kinds MUST
    always produce an unresolved conflict. Collisions with define
    block kinds MUST always produce an unresolved conflict. Collisions
    between plugin kinds MUST be recorded for policy-based resolution.
    Each conflict MUST emit E022 or E023 depending on the collision type.
  """

  verify unit "two plugins registering same kind produces conflict"
  verify unit "collision with built-in produces E023"
  verify unit "collision with define block produces E022"
  verify unit "no false positive for different kind names"

  tests ["../crates/specforge-common/src/kind_registry.rs"]
}

behavior resolve_entity_kind_conflict_via_config "Resolve Entity Kind Conflict Via Config" {
  invariants [entity_kind_uniqueness]
  types      [PackageManifest]

  contract """
    When entity kind conflicts between plugins are detected, the
    compiler MUST apply the configured entity_kind_policy from
    specforge.json. With policy "error" (default), unresolved
    conflicts MUST produce E022 diagnostics. With policy "priority",
    the first plugin in the plugins array MUST win and a W027
    warning MUST be emitted. With policy "namespace", conflicting
    kinds MUST be prefixed with the plugin short name. Explicit
    entity_kinds overrides MUST take precedence over any policy.
  """

  verify unit "error policy produces E022 for unresolved conflicts"
  verify unit "priority policy selects first plugin and emits W027"
  verify unit "namespace policy prefixes conflicting kinds"
  verify unit "explicit override takes precedence over policy"

  tests ["../crates/specforge-common/src/kind_registry.rs"]
}

behavior qualify_entity_kind_inline "Qualify Entity Kind Inline" {
  types      [PackageManifest]

  contract """
    The parser MUST recognize the @plugin/kind syntax as a qualified
    entity keyword. When encountered, the compiler MUST extract the
    plugin name and kind name, resolve via the KindRegistry's
    resolve_qualified method, and produce an EntityKind::Custom with
    the qualified name. Unresolved qualified names MUST produce an
    error diagnostic.
  """

  verify unit "parser recognizes @plugin/kind syntax"
  verify unit "qualified kind resolves to correct plugin"
  verify unit "unresolved qualified name produces error"

  tests ["../crates/specforge-parser/src/cst_to_ast.rs"]
}

// ── Entity Enhancement ───────────────────────────────────────

behavior load_package_manifest "Load Package Manifest" {
  types      [PackageManifest, PackageError]
  ports      [FileSystem]

  contract """
    When the compiler discovers an installed package, it MUST locate
    and parse the package's sidecar manifest.json file. For built-in
    packages, the manifest MUST be constructed from hardcoded Rust
    factory methods. For Wasm packages, the manifest MUST be read
    from the sidecar JSON alongside the .wasm binary. Malformed
    manifests MUST produce a PackageError diagnostic.
  """

  verify unit "built-in package produces valid manifest"
  verify unit "sidecar JSON parsed into PackageManifest"
  verify unit "malformed sidecar produces PackageError"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior register_entity_enhancements "Register Entity Enhancements" {
  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
  types      [PackageManifest, FieldEnhancement, DynamicEdgeType]

  contract """
    When a plugin manifest declares entity enhancements, the compiler
    MUST parse the enhancement declarations, validate that the target
    entity kinds exist, register the field-to-edge mappings in the
    FieldRegistry, and register any dynamic edge types. Registration
    MUST happen before the resolve phase begins. The order of
    registration MUST follow the plugins array order in specforge.json.
  """

  verify unit "enhancement fields registered in FieldRegistry"
  verify unit "unknown target entity kind produces error"
  verify unit "enhanced reference fields create graph edges"
  verify unit "enhanced data fields participate in type validation"
  verify unit "registration order follows plugins array"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior detect_enhancement_conflicts "Detect Enhancement Conflicts" {
  invariants [enhancement_field_uniqueness]
  types      [EnhancementConflict, FieldEnhancement, EnhancedFieldType, EnumFieldType, ReferenceFieldType]

  contract """
    During enhancement registration, the compiler MUST detect when two
    plugins register the same field name for the same entity kind. Each
    conflict MUST be recorded with both plugin identities and the
    conflicting field types. Conflicts with built-in fields MUST always
    produce a hard error (E018). Conflicts between plugins MUST be
    resolved according to the configured enhancement_policy.
  """

  verify unit "same (entity, field) from two plugins produces conflict"
  verify unit "conflict with built-in field produces E018"
  verify unit "conflict record includes both plugin identities"
  verify unit "no false positives for same field on different entities"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior resolve_enhancement_conflicts "Resolve Enhancement Conflicts" {
  types      [EnhancementConflict, ConflictResolution, EnhancementPolicy]

  contract """
    When enhancement conflicts are detected, the compiler MUST apply
    the configured enhancement policy. With policy "error" (default),
    unresolved conflicts MUST produce E017 diagnostics. With policy
    "priority", the first plugin in the plugins array MUST win and
    a W023 warning MUST be emitted. With policy "namespace", conflicting
    fields MUST be prefixed with the plugin short name. Explicit
    enhancement_overrides in specforge.json MUST take precedence over
    any policy.
  """

  verify unit "error policy produces E017 for unresolved conflicts"
  verify unit "priority policy selects first plugin and emits W023"
  verify unit "namespace policy prefixes conflicting fields"
  verify unit "explicit override takes precedence over policy"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Package Lifecycle ─────────────────────────────────────────

behavior install_wasm_package "Install Wasm Package" {
  types      [PackageManifest, PackageInstallResult, PackageSource, PackageError]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge add <pkg> is invoked, the system MUST resolve the package
    from its source (registry, local path, or git), download the .wasm
    binary, verify its SHA256 integrity, place it in the project, AOT compile
    it, and update specforge.json with the package entry. On failure at any
    step, the system MUST rollback all changes — no partial installs.
  """

  verify unit "resolves package from registry"
  verify unit "resolves package from local path"
  verify unit "verifies SHA256 integrity of downloaded .wasm"
  verify unit "AOT compiles after install"
  verify unit "updates specforge.json with package entry"
  verify unit "rolls back on download failure"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior upgrade_wasm_package "Upgrade Wasm Package" {
  types      [PackageManifest, PeerDependency, PackageInstallResult, PackageError]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge package upgrade is invoked, the system MUST check the
    source for a newer version, validate peer dependency compatibility
    with all installed packages, replace the .wasm binary, invalidate the
    old AOT cache entry, and recompile. Breaking peer dependency changes
    MUST require the --force flag. Without --force, the upgrade MUST be
    rejected with a diagnostic listing the incompatible peers.
  """

  verify unit "checks source for newer version"
  verify unit "validates peer dependency compatibility"
  verify unit "replaces binary and invalidates old cache"
  verify unit "recompiles AOT after upgrade"
  verify unit "rejects breaking peer change without --force"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Manifest Validation ──────────────────────────────────────

behavior validate_package_manifest "Validate Package Manifest" {
  types      [PackageManifest, PackageError]
  ports      [FileSystem]

  contract """
    The compiler MUST validate the manifest schema version before any
    parsing of manifest fields. A v1 manifest on a v2 runtime MUST
    produce a migration error with instructions to update. Unknown fields
    MUST produce a warning diagnostic. Missing required fields MUST
    produce a hard error diagnostic.
  """

  verify unit "v1 manifest on v2 runtime produces migration error"
  verify unit "unknown fields produce warning"
  verify unit "missing required fields produce hard error"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Error Recovery ───────────────────────────────────────────

behavior handle_wasm_trap "Handle Wasm Trap" {
  invariants [plugin_isolation, wasm_sandbox_integrity]
  types      [WasmTrapInfo, PluginLifecycleState, PackageError]
  ports      [WasmRuntime]

  contract """
    When an Extism trap occurs during any plugin export call, the compiler
    MUST catch the trap, extract trap details (kind, message, export name),
    transition the plugin lifecycle to failed, and emit a PackageError
    diagnostic. The trapped plugin MUST NOT be called again in the current
    compilation. Remaining plugins MUST continue execution normally.
  """

  verify unit "catches trap during validate() export"
  verify unit "catches trap during generate() export"
  verify unit "extracts trap kind and message"
  verify unit "transitions plugin to failed state"
  verify unit "remaining plugins continue after trap"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Cache Management ─────────────────────────────────────────

behavior invalidate_aot_cache "Invalidate AOT Cache" {
  types      [WasmModuleCache]
  ports      [WasmRuntime, FileSystem]

  contract """
    The AOT cache MUST be invalidated when: (1) the Extism/Wasmtime runtime
    version changes, (2) the user runs specforge cache clear, or (3) the
    .wasm binary content has changed. Stale AOT artifacts MUST be removed
    and the affected plugin MUST be marked for recompilation on next load.
  """

  verify unit "invalidates on runtime version change"
  verify unit "invalidates on specforge cache clear"
  verify unit "invalidates when .wasm binary changes"
  verify unit "removes stale AOT artifacts"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Discovery & Configuration ────────────────────────────────

behavior discover_packages "Discover Packages" {
  types      [PackageSource, PackageManifest, PackageError]
  ports      [WasmRuntime]

  contract """
    The system MUST determine the package source from the specifier format:
    @scope/name resolves to registry, ./path resolves to a local file,
    and git:url#ref resolves to a git repository. The system MUST query
    the source for the latest version matching the declared semver range.
  """

  verify unit "@scope/name resolves to registry"
  verify unit "./path resolves to local file"
  verify unit "git:url#ref resolves to git repository"
  verify unit "queries for latest version matching semver"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior configure_sandbox_policy "Configure Sandbox Policy" {
  types      [SandboxPolicy, PackageManifest]
  ports      [FileSystem]

  contract """
    The sandbox policy for each plugin MUST be computed by merging three
    layers: (1) built-in defaults, (2) per-plugin manifest sandbox policy,
    (3) project-level specforge.json overrides. The merged policy MUST NOT
    exceed 256MB total memory across all plugins. Overrides that would
    exceed system limits MUST produce a warning diagnostic.
  """

  verify unit "built-in defaults applied when no override"
  verify unit "manifest policy overrides defaults"
  verify unit "specforge.json overrides manifest policy"
  verify unit "total memory exceeding 256MB produces warning"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior run_doctor_check "Run Doctor Check" {
  types      [PackageManifest, EnhancementConflict, FieldEnhancement]
  ports      [FileSystem]

  contract """
    When specforge doctor is invoked, the system MUST load all plugin
    manifests, build the FieldRegistry, detect all conflicts, and
    produce a human-readable report listing installed plugins, their
    enhancements, any conflicts with actionable resolution suggestions,
    and additional checks (shadowed built-ins, unknown target entities,
    edge label conflicts). The --json flag MUST produce machine-readable
    JSON output for CI integration.
  """

  verify unit "doctor lists all installed packages with enhancement counts"
  verify unit "doctor lists all enhancements grouped by entity kind"
  verify unit "doctor reports conflicts with resolution suggestions"
  verify unit "doctor detects shadowed built-in fields"
  verify unit "doctor --json produces valid JSON output"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Package Source Resolution ────────────────────────────────

behavior parse_package_specifier "Parse Package Specifier" {
  types      [PackageManifest, PackageSource]

  contract """
    The system MUST parse package specifier strings into structured
    source descriptors. Supported formats: "@scope/name@version" for
    registry packages, "./path" for local packages, and "git:url#ref"
    for git-sourced packages. Invalid specifiers MUST produce a
    PackageError diagnostic with the expected format.
  """

  verify unit "@scope/name@version parsed as registry source"
  verify unit "./path parsed as local source"
  verify unit "git:url#ref parsed as git source"
  verify unit "invalid specifier produces PackageError"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior resolve_package_source "Resolve Package Source" {
  types      [PackageManifest, PackageSource, PackageError]
  ports      [WasmRuntime, FileSystem]

  contract """
    Given a parsed package specifier, the system MUST resolve it to a
    concrete manifest and .wasm binary. Registry sources MUST query the
    registry API. Local sources MUST read from the filesystem. Git sources
    MUST clone or fetch the repository at the specified ref. Resolution
    failures MUST produce a PackageError diagnostic.
  """

  verify unit "registry source resolves via registry API"
  verify unit "local source resolves from filesystem"
  verify unit "git source resolves from repository"
  verify unit "resolution failure produces PackageError"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Lock File Management ─────────────────────────────────────

behavior write_lock_file "Write Lock File" {
  types      [PackageManifest, LockFileEntry]
  ports      [FileSystem]

  contract """
    After resolving all packages, the system MUST write a specforge.lock
    file containing the exact resolved version and SHA256 wasm_hash for
    each installed package. The lock file format MUST be deterministic —
    same inputs always produce byte-identical output. The lock file MUST
    be written atomically to prevent corruption.
  """

  verify unit "lock file contains exact versions and wasm hashes"
  verify unit "lock file output is deterministic"
  verify unit "lock file written atomically"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior read_lock_file "Read Lock File" {
  types      [PackageManifest, LockFileEntry, PackageError]
  ports      [FileSystem]

  contract """
    When a specforge.lock file exists, the system MUST use locked versions
    instead of resolving from sources. Missing lock entries for declared
    packages MUST trigger resolution and lock file update. Malformed lock
    files MUST produce a warning and fall back to fresh resolution.
  """

  verify unit "locked versions used when lock file exists"
  verify unit "missing lock entry triggers resolution"
  verify unit "malformed lock file produces warning and falls back"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior verify_wasm_integrity "Verify Wasm Integrity" {
  types      [PackageManifest, LockFileEntry, PackageError]
  ports      [FileSystem]

  contract """
    The system MUST verify the SHA256 hash of each .wasm binary against
    the wasm_hash recorded in specforge.lock. Hash mismatches MUST produce
    a hard error diagnostic indicating potential tampering. The --skip-verify
    flag MUST bypass integrity checks with a warning.
  """

  verify unit "matching hash passes verification"
  verify unit "mismatched hash produces hard error"
  verify unit "--skip-verify bypasses check with warning"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Contribution Model ───────────────────────────────────────

behavior dispatch_contribution_exports "Dispatch Contribution Exports" {
  types      [PackageManifest, PackageContributions]
  ports      [WasmRuntime]

  contract """
    When a package declares contributions in its manifest, the compiler
    MUST route calls to the package's namespaced Wasm exports based on
    the contribution type. Entity contributions MUST call initialize()
    and validate(). Generator contributions MUST call generate().
    Provider contributions MUST call validate_ref(). Missing exports
    for declared contributions MUST produce E020.
  """

  verify unit "entity contributions dispatched to initialize() and validate()"
  verify unit "generator contributions dispatched to generate()"
  verify unit "provider contributions dispatched to validate_ref()"
  verify unit "missing export for declared contribution produces E020"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior enforce_per_call_site_permissions "Enforce Per-Call-Site Permissions" {
  invariants [wasm_sandbox_integrity]
  types      [PackageManifest, SandboxPolicy, PackageContributions]
  ports      [WasmRuntime]

  contract """
    Host function permissions MUST be enforced per export call site, not
    per package. A package's validator export MUST only access query_graph
    and emit_diagnostic. A package's generator export MUST additionally
    access emit_file. A package's provider export MUST additionally access
    http_get. Calls to unauthorized host functions MUST be rejected.
  """

  verify unit "validator export limited to query_graph and emit_diagnostic"
  verify unit "generator export additionally allows emit_file"
  verify unit "provider export additionally allows http_get"
  verify unit "unauthorized host function call is rejected"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior validate_contribution_exports "Validate Contribution Exports" {
  types      [PackageManifest, PackageContributions, PackageError]
  ports      [WasmRuntime]

  contract """
    After loading a package, the compiler MUST verify that the .wasm binary
    exports all functions required by its declared contributions. Missing
    exports MUST produce an E020 diagnostic listing the expected export
    names. Extra exports beyond declared contributions MUST be ignored.
  """

  verify unit "all declared contribution exports present passes"
  verify unit "missing contribution export produces E020"
  verify unit "extra exports beyond contributions are ignored"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior toggle_package_contributions "Toggle Package Contributions" {
  types      [PackageManifest, PackageContributions]

  contract """
    The specforge.json configuration MUST support enabling or disabling
    individual contributions from a package. Disabled contributions MUST
    be skipped during dispatch. The package MUST still be loaded and
    initialized — only the disabled contribution exports are not called.
  """

  verify unit "disabled contribution is skipped during dispatch"
  verify unit "package still loaded when some contributions disabled"
  verify unit "re-enabled contribution resumes normal dispatch"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior migrate_v1_manifest "Migrate V1 Manifest" {
  types      [PackageManifest, PackageError]

  contract """
    When a package manifest uses the v1 format with a PluginKind field,
    the compiler MUST automatically infer the contributions from the
    kind value: kind=plugin maps to entities+validators contributions,
    kind=provider maps to providers contribution, kind=generator maps
    to generators contribution. A W028 warning MUST be emitted advising
    migration to the v2 contributes format.
  """

  verify unit "v1 kind=plugin infers entity contributions"
  verify unit "v1 kind=provider infers provider contribution"
  verify unit "v1 kind=generator infers generator contribution"
  verify unit "W028 warning emitted for v1 manifest"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
