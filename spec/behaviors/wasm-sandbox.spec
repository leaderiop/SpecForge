// Wasm sandbox enforcement, compile cache, session runtime reuse,
// error recovery, and sandbox configuration

use "invariants/wasm"
use "types/wasm"
use "types/config"
use "types/errors"
use "ports/outbound"
use "events/wasm-sandbox"
behavior enforce_wasm_sandbox "Enforce Wasm Sandbox" {
  invariants [wasm_sandbox_integrity, extension_isolation]
  category   command
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
  verify contract "Enforce Wasm Sandbox: Wasm sandbox enforcement holds — sandbox_policy_configured, wasm_runtime_available, memory_limit_enforced, execution_time_enforced, violations_trapped"

  tests ["crates/specforge-wasm/tests/sandbox_integration.rs"]
}

behavior compile_wasm_component_with_cache "Compile Wasm Component With Cache" {
  invariants [wasm_compile_cache_integrity]
  category   command
  types      [ManifestV2]
  ports      [WasmRuntime, FileSystem]

  requires {
    component_binary_available "component .wasm binary exists and is readable"
    cache_dir_resolved "compile cache directory resolved: SPECFORGE_WASMTIME_CACHE if set, else $HOME/.cache/specforge/wasmtime; 'off' disables"
  }

  ensures {
    engine_configured_at_construction "the compile cache is configured when the runtime engine is built, before any component compiles"
    first_compile_populates_cache "first compile of a binary writes its compiled artifact to the cache directory"
    cache_hit_skips_compilation "a later engine over the same cache directory deserializes the artifact instead of recompiling"
    cache_failure_degrades "an unwritable or corrupted cache degrades to uncached compilation with a warning, never a load failure"
  }

  contract """
    The runtime engine (wasmtime) MUST be constructed with its native
    on-disk compilation cache when SPECFORGE_WASMTIME_CACHE selects a
    directory (default: $HOME/.cache/specforge/wasmtime; the value 'off'
    disables the cache). Compiled machine code MUST be cached and
    deserialized on later loads, keyed by the engine configuration and
    component bytes. Cache corruption or an unusable cache directory MUST
    degrade to uncached compilation with a warning. Installed-binary
    integrity is a separate concern enforced by the lockfile hash pin
    (E033) at load time.
  """

  verify unit "first compile populates the compile cache directory"
  verify unit "second engine over the same cache dir loads via cache and executes"
  verify unit "unwritable cache dir degrades to uncached compile with warning"
  verify unit "tampered installed binary refused via E033 lockfile pin"
  verify contract "Compile Wasm Component With Cache: wasm compile cache holds — component_binary_available, cache_dir_resolved, engine_configured_at_construction, first_compile_populates_cache, cache_hit_skips_compilation, cache_failure_degrades"

  tests ["crates/specforge-component/tests/compile_cache.rs"]
}

behavior reuse_session_runtime "Reuse Session Runtime" {
  invariants [extension_isolation]
  category   command
  types      [ExtensionLifecycleState]
  ports      [WasmRuntime]

  requires {
    session_context "the process is a CLI run, an LSP session, or an MCP server session"
    wasm_runtime_available "the session's ComponentRuntime is available to all compilation stages"
  }

  ensures {
    single_engine_per_session "one runtime engine is constructed per run/session and shared by every stage"
    plugin_instances_reused "loaded component instances are reused across repeated calls without re-instantiation"
    instance_replaced_atomically "reloading an extension atomically replaces its loaded instance"
    instances_dropped_on_shutdown "all instances are dropped when the runtime is dropped at session end"
  }

  contract """
    Each process MUST construct a single ComponentRuntime and share it
    across compilation stages (CLI pipeline, LSP state, MCP server).
    Loaded component instances live in the runtime and MUST be reused for
    subsequent calls. Hot reload MUST atomically replace an extension's
    loaded instance. No cross-process warm pool is promised: a new
    process pays component compilation once per binary (mitigated by the
    on-disk compile cache) and instances end with the session.
  """

  verify unit "same runtime instance serves repeated calls without re-instantiation"
  verify unit "hot reload atomically replaces a loaded component"
  verify unit "runtime dropped at session end releases all instances"
  verify contract "Reuse Session Runtime: session runtime reuse holds — session_context, wasm_runtime_available, single_engine_per_session, plugin_instances_reused, instance_replaced_atomically, instances_dropped_on_shutdown"

  tests ["crates/specforge-component/tests/runtime.rs", "crates/specforge-component/tests/compile_cache.rs"]
}


// -- Error Recovery -----

behavior handle_wasm_trap "Handle Wasm Trap" {
  invariants [extension_isolation, wasm_sandbox_integrity]
  category   command
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
  verify contract "Handle Wasm Trap: Wasm trap handling holds — trap_occurred, wasm_trap_caught_emitted, lifecycle_transitioned, trapped_extension_skipped, remaining_extensions_continue"

  tests ["crates/specforge-component/tests/runtime.rs"]
}

// -- Compile Cache -----
// The compile cache is owned by the runtime engine (wasmtime): entries are
// keyed by bytes + engine config and validated by the engine itself, so
// there is no host-side invalidation behavior. See
// compile_wasm_component_with_cache and the wasm_compile_cache_integrity
// invariant.

// V2: .yaml/.yml removed from the default filesystem allowlist. Extensions
// that need to emit YAML output MUST explicitly declare .yaml or .yml in
// their manifest's allowed_output_extensions field. This prevents accidental
// config-file generation by extensions that do not intend it.
behavior configure_sandbox_policy "Configure Sandbox Policy" {
  invariants [wasm_sandbox_integrity]
  category   command
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
  verify contract "Configure Sandbox Policy: sandbox policy configuration holds — manifest_available, config_available, sandbox_policy_configured_emitted, most_restrictive_wins, list_intersection_applied, memory_ceiling_enforced, code_extensions_blocked"

  tests ["crates/specforge-wasm/tests/wasm_lifecycle.rs"]
}
