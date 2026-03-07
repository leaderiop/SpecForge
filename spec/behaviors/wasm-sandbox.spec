// Wasm sandbox enforcement, AOT compilation, caching, warm engines,
// error recovery, and sandbox configuration

use invariants/wasm
use types/wasm
use types/config
use types/errors
use ports/outbound
use events/wasm-sandbox

behavior enforce_wasm_sandbox "Enforce Wasm Sandbox" {
  invariants [wasm_sandbox_integrity, extension_isolation]
  types      [SandboxPolicy, ExtensionError]
  ports      [WasmRuntime]

  contract """
    The runtime MUST enforce the sandbox policy for each extension: memory
    limits via Extism's linear memory cap, execution time limits via
    fuel metering, filesystem restrictions via host function validation,
    and network restrictions via domain allowlists. Violations MUST
    trap the extension and emit a diagnostic.
  """

  produces [wasm_sandbox_violation]

  verify unit "memory limit enforced via linear memory cap"
  verify unit "execution time limit enforced via fuel metering"
  verify unit "filesystem restriction enforced"
  verify unit "network restriction enforced"

}

behavior aot_compile_wasm_module "AOT Compile Wasm Module" {
  invariants [aot_cache_integrity]
  types      [WasmModuleCache, ManifestV2]
  ports      [WasmRuntime, FileSystem]

  contract """
    On first load of a .wasm binary, the runtime MUST AOT compile the
    module and cache the compiled artifact in .specforge/cache/ using
    a content-hash filename. Subsequent loads MUST use the cached
    artifact to reduce cold start time.
  """

  produces [wasm_aot_compiled]

  verify unit "first load triggers AOT compilation"
  verify unit "compiled artifact cached with content-hash filename"
  verify unit "subsequent load uses cached artifact"

}

behavior cache_aot_artifacts "Cache AOT Artifacts" {
  invariants [aot_cache_integrity]
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

}

behavior warm_wasm_engine_instance "Warm Wasm Engine Instance" {
  invariants [extension_isolation]
  types      [ExtensionLifecycleState]
  ports      [WasmRuntime]

  contract """
    For LSP and MCP server contexts, the runtime MUST keep initialized
    Wasm engine instances warm across compilations. Warm instances MUST
    be reused for subsequent validate() and render() calls. Instances
    MUST be unloaded when the extension is removed or the server shuts down.
  """

  produces [engine_warmed]

  verify unit "warm instance reused across compilations"
  verify unit "instance unloaded on extension removal"

}

behavior evict_warm_engine_instance "Evict Warm Engine Instance" {
  invariants [extension_isolation]
  types      [ExtensionLifecycleState, WarmEngineConfig]
  ports      [WasmRuntime]

  contract """
    The runtime MUST evict warm Wasm engine instances when memory pressure
    exceeds configured limits. Eviction follows LRU ordering. The maximum
    concurrent warm instances MUST be configurable via CompilerConfig.
    Default: 16 instances, 512MB total memory ceiling.
  """

  produces [engine_evicted]

  verify unit "LRU engine evicted when max instances exceeded"
  verify unit "memory ceiling triggers eviction of least-recent engine"

}

// -- Error Recovery -----

behavior handle_wasm_trap "Handle Wasm Trap" {
  invariants [extension_isolation, wasm_sandbox_integrity]
  types      [WasmTrapInfo, ExtensionLifecycleState, ExtensionError]
  ports      [WasmRuntime]

  contract """
    When an Extism trap occurs during any extension export call, the compiler
    MUST catch the trap, extract trap details (kind, message, export name),
    transition the extension lifecycle to failed, and emit a ExtensionError
    diagnostic. The trapped extension MUST NOT be called again in the current
    compilation. Remaining extensions MUST continue execution normally.
  """

  produces [wasm_trap_caught]

  verify unit "catches trap during validate() export"
  verify unit "catches trap during render() call"
  verify unit "extracts trap kind and message"
  verify unit "transitions extension to failed state"
  verify unit "remaining extensions continue after trap"

}

// -- Cache Management -----

behavior invalidate_aot_cache "Invalidate AOT Cache" {
  invariants [aot_cache_integrity]
  types      [WasmModuleCache]
  ports      [WasmRuntime, FileSystem]

  contract """
    The AOT cache MUST be invalidated when: (1) the Extism/Wasmtime runtime
    version changes, (2) the user runs specforge cache clear, or (3) the
    .wasm binary content has changed. Stale AOT artifacts MUST be removed
    and the affected extension MUST be marked for recompilation on next load.
  """

  produces [aot_cache_invalidated]

  verify unit "invalidates on runtime version change"
  verify unit "invalidates on specforge cache clear"
  verify unit "invalidates when .wasm binary changes"
  verify unit "removes stale AOT artifacts"

}

// V2: .yaml/.yml removed from the default filesystem allowlist. Extensions
// that need to emit YAML output MUST explicitly declare .yaml or .yml in
// their manifest's allowed_output_extensions field. This prevents accidental
// config-file generation by extensions that do not intend it.
behavior configure_sandbox_policy "Configure Sandbox Policy" {
  invariants [wasm_sandbox_integrity]
  types      [SandboxPolicy, ManifestV2]
  ports      [FileSystem]

  contract """
    The sandbox policy for each extension MUST be computed by merging three
    layers: (1) built-in defaults, (2) per-extension manifest sandbox policy,
    (3) project-level specforge.json overrides. The merged policy MUST NOT
    exceed 256MB total memory across all extensions. Overrides that would
    exceed system limits MUST produce a warning diagnostic.
    Numeric policies follow most-restrictive-wins: max_memory_mb and
    max_execution_ms use the minimum value across default, manifest,
    and config override. List policies (allowed_domains, allowed_paths)
    use the intersection of all sources. The final total memory across
    all extensions MUST NOT exceed 256MB.
    Manifest-level allowed_output_extensions MUST NOT include code file
    extensions (.rs, .py, .js, .ts, .go, .java, .c, .cpp, .rb, .swift,
    .kt). The system MUST reject manifest policies that attempt to add
    blacklisted extensions with an E030 diagnostic.
  """

  produces [sandbox_policy_configured]

  verify unit "built-in defaults applied when no override"
  verify unit "manifest policy overrides defaults"
  verify unit "specforge.json overrides manifest policy"
  verify unit "total memory exceeding 256MB produces warning"
  verify unit "manifest with code file extension (.rs, .js, .ts) in allowed_output_extensions produces E030"
  verify unit "manifest with non-code extension (.json, .csv, .md) in allowed_output_extensions passes"

}
