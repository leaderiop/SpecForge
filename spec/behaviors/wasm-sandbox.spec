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

  requires {
    sandbox_policy_configured "sandbox policy has been computed for the extension via configure_sandbox_policy"
    wasm_runtime_available "WasmRuntime port is available for enforcement"
  }

  ensures {
    memory_limit_enforced "memory limits are enforced via runtime's linear memory cap"
    execution_time_enforced "execution time limits are enforced via fuel metering"
    violations_trapped "sandbox violations trap the extension and emit a diagnostic"
  }

  contract """
    The runtime MUST enforce the sandbox policy for each extension: memory
    limits via the runtime's linear memory cap, execution time limits via
    fuel metering, filesystem restrictions via host function validation,
    and network restrictions via domain allowlists. Violations MUST
    trap the extension and emit a diagnostic.
  """

  produces [wasm_sandbox_violation]

  verify unit "memory limit enforced via linear memory cap"
  verify unit "execution time limit enforced via fuel metering"
  verify unit "filesystem restriction enforced"
  verify unit "network restriction enforced"
  verify contract "requires/ensures consistency for Wasm sandbox enforcement"

}

behavior aot_compile_wasm_module "AOT Compile Wasm Module" {
  invariants [aot_cache_integrity]
  types      [WasmModuleCache, ManifestV2]
  ports      [WasmRuntime, FileSystem]
  consumes   [aot_cache_invalidated]

  requires {
    wasm_binary_available ".wasm binary exists and is accessible for compilation"
    aot_cache_invalidated_fired "aot_cache_invalidated event has fired, or this is the first load of the binary"
  }

  ensures {
    wasm_aot_compiled_emitted "wasm_aot_compiled event is emitted after successful compilation"
    artifact_cached "compiled artifact is cached in .specforge/cache/ with content-hash filename"
    subsequent_loads_fast "subsequent loads use the cached artifact to reduce cold start time"
  }

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
  verify contract "requires/ensures consistency for AOT Wasm compilation"

}

behavior cache_aot_artifacts "Cache AOT Artifacts" {
  invariants [aot_cache_integrity]
  types      [WasmModuleCache]
  ports      [FileSystem]
  consumes   [wasm_aot_compiled]

  requires {
    wasm_aot_compiled_fired "wasm_aot_compiled event has fired, confirming AOT artifact is available for caching"
    filesystem_available "FileSystem port is available for writing cache entries to .specforge/cache/"
  }

  ensures {
    content_addressed "cache entries use filenames derived from SHA256 of the .wasm binary"
    corruption_detected "corrupted cache entries are detected by re-hashing on load"
    corruption_recovered "corrupted entries are evicted and recompiled"
  }

  contract """
    The AOT content-addressed cache MUST store entries using filenames
    derived from the SHA256 of the .wasm binary. The cache MUST detect
    corruption by re-hashing on load. Corrupted entries MUST be evicted
    and recompiled. The cache directory MUST be .specforge/cache/.
  """

  verify unit "cache entries use content-addressed filenames"
  verify unit "corrupted cache entry is evicted and recompiled"
  verify contract "requires/ensures consistency for AOT artifact caching"

}

behavior warm_wasm_engine_instance "Warm Wasm Engine Instance" {
  invariants [extension_isolation]
  types      [ExtensionLifecycleState, WarmEngineConfig]
  ports      [WasmRuntime]
  consumes   [engine_evicted]

  requires {
    lsp_or_mcp_context "runtime is in LSP or MCP server context (not CLI batch mode)"
    wasm_runtime_available "WasmRuntime port is available for keeping engine instances warm"
  }

  ensures {
    engine_warmed_emitted "engine_warmed event is emitted after instance is warmed"
    instance_reused "warm instances are reused for subsequent validate() and render() calls"
    instance_unloaded_on_removal "instances are unloaded when extension is removed or server shuts down"
  }

  contract """
    For LSP and MCP server contexts, the runtime MUST keep initialized
    Wasm engine instances warm across compilations. Warm instances MUST
    be reused for subsequent validate() and render() calls. Instances
    MUST be unloaded when the extension is removed or the server shuts down.
  """

  produces [engine_warmed]

  verify unit "warm instance reused across compilations"
  verify unit "instance unloaded on extension removal"
  verify contract "requires/ensures consistency for warm engine instance management"

}

behavior evict_warm_engine_instance "Evict Warm Engine Instance" {
  invariants [extension_isolation]
  types      [ExtensionLifecycleState, WarmEngineConfig]
  ports      [WasmRuntime]
  consumes   [engine_warmed]

  requires {
    engine_warmed_fired "engine_warmed event has fired, confirming warm instances exist to potentially evict"
    memory_pressure_detected "memory pressure exceeds configured limits or max concurrent instances exceeded"
  }

  ensures {
    engine_evicted_emitted "engine_evicted event is emitted after LRU instance is evicted"
    lru_order_respected "eviction follows LRU ordering"
    memory_ceiling_enforced "total memory across warm instances stays within configured ceiling (default 512MB)"
  }

  contract """
    The runtime MUST evict warm Wasm engine instances when memory pressure
    exceeds configured limits. Eviction follows LRU ordering. The maximum
    concurrent warm instances MUST be configurable via CompilerConfig.
    Default: 16 instances, 512MB total memory ceiling.
  """

  produces [engine_evicted]

  verify unit "LRU engine evicted when max instances exceeded"
  verify unit "memory ceiling triggers eviction of least-recent engine"
  verify contract "requires/ensures consistency for warm engine eviction"

}

// -- Error Recovery -----

behavior handle_wasm_trap "Handle Wasm Trap" {
  invariants [extension_isolation, wasm_sandbox_integrity]
  types      [WasmTrapInfo, ExtensionLifecycleState, ExtensionError]
  ports      [WasmRuntime]
  consumes   [wasm_sandbox_violation, wasm_integrity_check_failed]

  requires {
    trap_occurred "a Wasm trap has occurred during an extension export call (sandbox violation or integrity failure)"
  }

  ensures {
    wasm_trap_caught_emitted "wasm_trap_caught event is emitted with trap details"
    lifecycle_transitioned "extension lifecycle transitions to failed state"
    trapped_extension_skipped "trapped extension is not called again in the current compilation"
    remaining_extensions_continue "remaining extensions continue execution normally after trap"
  }

  contract """
    When a Wasm trap occurs during any extension export call, the compiler
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
  verify contract "requires/ensures consistency for Wasm trap handling"

}

// -- Cache Management -----

behavior invalidate_aot_cache "Invalidate AOT Cache" {
  invariants [aot_cache_integrity]
  types      [WasmModuleCache]
  ports      [WasmRuntime, FileSystem]
  consumes   [wasm_extension_removed, batch_update_completed]

  requires {
    invalidation_trigger "one of: runtime version changed, specforge cache clear invoked, .wasm binary content changed, or extension removed/updated"
  }

  ensures {
    aot_cache_invalidated_emitted "aot_cache_invalidated event is emitted after stale artifacts are removed"
    stale_artifacts_removed "stale AOT artifacts are deleted from .specforge/cache/"
    extension_marked_for_recompilation "affected extension is marked for recompilation on next load"
  }

  contract """
    The AOT cache MUST be invalidated when: (1) the Wasm runtime
    version changes, (2) the user runs specforge cache clear, or (3) the
    .wasm binary content has changed. Stale AOT artifacts MUST be removed
    and the affected extension MUST be marked for recompilation on next load.
  """

  produces [aot_cache_invalidated]

  verify unit "invalidates on runtime version change"
  verify unit "invalidates on specforge cache clear"
  verify unit "invalidates when .wasm binary changes"
  verify unit "removes stale AOT artifacts"
  verify contract "requires/ensures consistency for AOT cache invalidation"

}

// V2: .yaml/.yml removed from the default filesystem allowlist. Extensions
// that need to emit YAML output MUST explicitly declare .yaml or .yml in
// their manifest's allowed_output_extensions field. This prevents accidental
// config-file generation by extensions that do not intend it.
behavior configure_sandbox_policy "Configure Sandbox Policy" {
  invariants [wasm_sandbox_integrity]
  types      [SandboxPolicy, ManifestV2]
  ports      [FileSystem]

  requires {
    manifest_available "extension manifest with optional sandbox policy is loaded"
    config_available "specforge.json with optional project-level overrides is available"
  }

  ensures {
    sandbox_policy_configured_emitted "sandbox_policy_configured event is emitted with the merged policy"
    most_restrictive_wins "numeric policies use minimum value across default, manifest, and config override"
    list_intersection_applied "list policies use intersection of all sources"
    memory_ceiling_enforced "total memory across all extensions does not exceed 256MB"
    code_extensions_blocked "manifest-level allowed_output_extensions with code file extensions produce E030"
  }

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
  verify contract "requires/ensures consistency for sandbox policy configuration"

}
