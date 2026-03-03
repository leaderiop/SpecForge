// Wasm/Extism plugin runtime behaviors

use invariants/wasm
use types/wasm
use types/config
use types/errors
use ports/outbound

// ── Wasm Module Lifecycle ─────────────────────────────────────

behavior load_wasm_module "Load Wasm Module" {
  invariants [wasm_sandbox_integrity]
  types      [WasmPluginManifest, WasmModuleCache, PluginError]
  ports      [WasmRuntime]

  contract """
    When the compiler loads a plugin, it MUST locate the .wasm binary
    from the manifest's wasmPath, check the AOT cache for a pre-compiled
    module matching the content hash, and load it into the Extism runtime.
    Cache hits MUST skip recompilation. Missing .wasm files MUST produce
    a PluginError diagnostic.
  """

  verify unit "loads .wasm binary from manifest path"
  verify unit "uses AOT cache on cache hit"
  verify unit "missing .wasm produces PluginError"
}

behavior initialize_wasm_plugin "Initialize Wasm Plugin" {
  invariants [peer_dependency_satisfaction]
  types      [WasmPluginManifest, PluginLifecycleState]
  ports      [WasmRuntime]

  contract """
    After loading a Wasm module, the compiler MUST call the plugin's
    initialize() export. During initialization, the plugin registers
    its entity types via specforge.register_entity and its edge types
    via specforge.register_edge. The plugin lifecycle MUST transition
    from loading to initialized on success, or to failed on error.
  """

  verify unit "calls initialize() export on loaded module"
  verify unit "entity types registered via host function"
  verify unit "lifecycle transitions to initialized on success"
  verify unit "lifecycle transitions to failed on error"
}

behavior call_wasm_validate "Call Wasm Validate" {
  invariants [plugin_load_order_determinism]
  types      [WasmPluginManifest, PluginLifecycleState]
  ports      [WasmRuntime]

  contract """
    After all plugins are initialized, the compiler MUST call each
    plugin's validate() export in topological order determined by
    peer dependencies. Plugins MUST emit diagnostics via the
    specforge.emit_diagnostic host function. The compiler MUST
    collect all diagnostics and continue to the next plugin.
  """

  verify unit "calls validate() in topological order"
  verify unit "diagnostics emitted via host function are collected"
  verify unit "validation continues to next plugin after errors"
}

behavior call_wasm_generate "Call Wasm Generate" {
  types      [WasmPluginManifest, GenConfig, PluginLifecycleState]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge gen invokes a generator plugin, the compiler MUST
    call the plugin's generate() export, passing the serialized graph
    via the specforge.query_graph host function. Generated files MUST
    be collected via specforge.emit_file. Plugin traps MUST produce
    a PluginError diagnostic without crashing the compiler.
  """

  verify unit "calls generate() export on the plugin"
  verify unit "graph is available via query_graph host function"
  verify unit "generated files collected via emit_file"
  verify unit "plugin trap produces PluginError"
}

// ── Host Functions ────────────────────────────────────────────

behavior provide_host_function_query_graph "Provide Host Function: query_graph" {
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
}

behavior provide_host_function_emit_diagnostic "Provide Host Function: emit_diagnostic" {
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
}

behavior provide_host_function_register_entity "Provide Host Function: register_entity" {
  types      [HostFunctionBinding, WasmPluginManifest]
  ports      [WasmRuntime]

  contract """
    The specforge.register_entity host function MUST accept an entity
    type registration with name, required fields, optional fields, and
    reference targets. The registered entity type MUST participate in
    parsing, resolution, and validation like built-in entity types.
  """

  verify unit "registered entity type is parseable"
  verify unit "registered entity participates in resolution"
}

behavior provide_host_function_register_edge "Provide Host Function: register_edge" {
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
}

behavior provide_host_function_emit_file "Provide Host Function: emit_file" {
  invariants [wasm_sandbox_integrity]
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
}

behavior provide_host_function_http_get "Provide Host Function: http_get" {
  invariants [wasm_sandbox_integrity]
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
}

// ── Sandbox & Performance ─────────────────────────────────────

behavior enforce_wasm_sandbox "Enforce Wasm Sandbox" {
  invariants [wasm_sandbox_integrity]
  types      [SandboxPolicy, PluginError]
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
}

behavior aot_compile_wasm_module "AOT Compile Wasm Module" {
  types      [WasmModuleCache, WasmPluginManifest]
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
}

behavior cache_aot_artifacts "Cache AOT Artifacts" {
  types      [WasmModuleCache]
  ports      [FileSystem]

  contract """
    AOT cache entries MUST use content-addressed filenames (SHA256 of
    .wasm binary). The cache MUST detect corruption by re-hashing on
    load. Corrupted entries MUST be evicted and recompiled. The cache
    directory MUST be .specforge/cache/.
  """

  verify unit "cache entries use content-addressed filenames"
  verify unit "corrupted cache entry is evicted and recompiled"
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
}

// ── Dependencies ──────────────────────────────────────────────

behavior validate_wasm_peer_dependencies "Validate Wasm Peer Dependencies" {
  invariants [peer_dependency_satisfaction]
  types      [PeerDependency, WasmPluginManifest, PluginError]

  contract """
    Before initializing plugins, the compiler MUST check that all
    declared peer dependencies are satisfied. For each peer dependency,
    the referenced plugin MUST be installed and its version MUST match
    the declared semver range. Unsatisfied peers MUST produce a hard
    error diagnostic.
  """

  verify unit "satisfied peer dependency passes"
  verify unit "missing peer produces hard error"
  verify unit "version mismatch produces hard error"
}

behavior topological_sort_plugins "Topological Sort Plugins" {
  invariants [plugin_load_order_determinism]
  types      [PeerDependency, WasmPluginManifest]

  contract """
    The compiler MUST compute a topological order over installed plugins
    based on their peer dependencies. Plugins with no dependencies MUST
    be loaded first. Cycles in peer dependencies MUST produce an error
    diagnostic. The sort MUST be deterministic — ties broken by plugin name.
  """

  verify unit "plugins sorted in dependency order"
  verify unit "cycle in peer dependencies produces error"
  verify unit "deterministic ordering on ties"
}

// ── Plugin Authoring ──────────────────────────────────────────

behavior scaffold_wasm_plugin_project "Scaffold Wasm Plugin Project" {
  types      [WasmPluginManifest]
  ports      [FileSystem]

  contract """
    When specforge plugin init is invoked, the system MUST scaffold a
    new Wasm plugin project with: a manifest file, a src/ directory
    with a skeleton implementing initialize/validate/generate exports,
    a build script targeting wasm32-wasi, and a README with PDK docs.
  """

  verify unit "scaffold creates manifest file"
  verify unit "scaffold creates src/ with skeleton exports"
  verify unit "scaffold creates build script for wasm32-wasi"
}

behavior build_wasm_plugin "Build Wasm Plugin" {
  types      [WasmPluginManifest]
  ports      [FileSystem]

  contract """
    When specforge plugin build is invoked, the system MUST compile
    the plugin source to a .wasm binary using the configured toolchain.
    The output .wasm MUST be placed alongside the manifest. Build
    errors MUST be reported as diagnostics.
  """

  verify unit "build produces .wasm binary"
  verify unit "build errors reported as diagnostics"
}

behavior test_wasm_plugin_locally "Test Wasm Plugin Locally" {
  types      [WasmPluginManifest, SandboxPolicy]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge plugin test is invoked, the system MUST load the
    locally built .wasm binary, execute it against test fixtures, and
    report results. The plugin MUST run in the same sandbox as production
    to catch permission errors early.
  """

  verify unit "test loads local .wasm binary"
  verify unit "test runs against fixtures"
  verify unit "test uses production sandbox policy"
}

behavior publish_wasm_plugin "Publish Wasm Plugin" {
  types      [WasmPluginManifest]
  ports      [FileSystem]

  contract """
    When specforge plugin publish is invoked, the system MUST package
    the .wasm binary and manifest, then publish to the configured
    registry (npm, OCI, or GitHub Releases). The manifest MUST be
    validated before publishing.
  """

  verify unit "publish packages .wasm and manifest"
  verify unit "manifest validated before publish"
}
