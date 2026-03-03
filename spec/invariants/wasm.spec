// Wasm/Extism runtime invariants

invariant wasm_sandbox_integrity "Wasm Sandbox Integrity" {
  guarantee """
    Wasm plugins MUST NOT escape the Extism sandbox. A plugin MUST NOT
    access the host filesystem, network, or memory outside its linear
    memory region unless explicitly permitted by the sandbox policy.
    Any sandbox violation MUST trap the plugin and emit a diagnostic.
  """
  enforced_by [enforce_wasm_sandbox, provide_host_function_emit_file, provide_host_function_http_get]
  risk high

  verify property "no plugin can read or write outside its sandbox boundaries"
  verify unit "sandbox violation traps the plugin and emits a diagnostic"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant plugin_load_order_determinism "Plugin Load Order Determinism" {
  guarantee """
    Given the same set of installed plugins, the compiler MUST produce
    the same topological load order on every invocation. The ordering
    MUST be deterministic and reproducible across platforms.
  """
  enforced_by [topological_sort_packages]
  risk medium

  verify property "same plugin set produces identical load order across 100 runs"
  verify unit "load order is deterministic across different platforms"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant peer_dependency_satisfaction "Peer Dependency Satisfaction" {
  guarantee """
    If a plugin declares peer dependencies, the compiler MUST verify that
    all declared peers are installed and satisfy the declared semver ranges.
    Unsatisfied peer dependencies MUST produce an error diagnostic, not
    a silent degradation.
  """
  enforced_by [validate_package_peer_dependencies]
  risk high

  verify unit "satisfied peer dependencies pass validation"
  verify unit "unsatisfied peer dependency produces an error diagnostic"
  verify unit "peer with wrong version range produces an error diagnostic"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Cache & Isolation Invariants ─────────────────────────────

invariant aot_cache_integrity "AOT Cache Integrity" {
  guarantee """
    AOT artifacts MUST be verified on load by re-hashing the source .wasm
    binary and comparing against the cache key. Corrupted, truncated, or
    platform-mismatched cache entries MUST be automatically evicted and
    recompiled. The cache MUST NOT serve stale or invalid artifacts.
  """
  enforced_by [cache_aot_artifacts, invalidate_aot_cache]
  risk medium

  verify property "corrupted AOT artifact is detected and recompiled"
  verify unit "platform-mismatched cache entry is evicted"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant plugin_isolation "Plugin Isolation" {
  guarantee """
    A plugin failure MUST NOT affect other plugins or the host compiler.
    After a plugin traps or fails during any lifecycle phase, the remaining
    plugins MUST continue execution normally. The failed plugin MUST be
    excluded from subsequent phases in the current compilation.
  """
  enforced_by [handle_wasm_trap, enforce_wasm_sandbox]
  risk high

  verify property "plugin trap does not affect other plugins"
  verify unit "failed plugin excluded from subsequent phases"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant host_function_type_safety "Host Function Type Safety" {
  guarantee """
    Data exchanged between the host and plugins via host functions MUST
    conform to declared schemas. Malformed input from a plugin MUST
    produce a PackageError diagnostic, not undefined behavior. The host
    MUST validate all plugin-provided data before processing.
  """
  enforced_by [provide_host_function_query_graph, provide_host_function_emit_diagnostic, provide_host_function_register_entity, provide_host_function_register_edge, provide_host_function_emit_file, provide_host_function_http_get]
  risk high

  verify unit "malformed plugin input produces PackageError"
  verify unit "valid plugin input is processed correctly"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Entity Kind Invariants ───────────────────────────────────

invariant entity_kind_uniqueness "Entity Kind Uniqueness" {
  guarantee """
    No two plugins MAY register the same entity kind name without
    explicit resolution via the configured entity_kind_policy or
    entity_kinds overrides in specforge.json. Built-in keywords and
    DSL syntax words MUST always be rejected at registration time.
    Collisions with define block kinds MUST produce a hard error.
  """
  enforced_by [reject_reserved_entity_kind, detect_entity_kind_collision, resolve_entity_kind_conflict_via_config]
  risk high

  verify property "no two plugins can silently register the same entity kind"
  verify unit "built-in keyword rejection is unconditional"

  tests ["../crates/specforge-common/src/kind_registry.rs"]
}

// ── Entity Enhancement Invariants ────────────────────────────

invariant enhancement_field_uniqueness "Enhancement Field Uniqueness" {
  guarantee """
    No two plugins MAY register the same field name for the same entity
    kind. When a conflict is detected, the compiler MUST resolve it
    according to the configured enhancement_policy or produce a hard
    error. The resolution MUST be deterministic and explicit.
  """
  enforced_by [detect_enhancement_conflicts, resolve_enhancement_conflicts]
  risk medium

  verify property "no two plugins can silently claim the same field"
  verify unit "conflict resolution is deterministic across runs"

  tests ["../crates/specforge-validator/src/passes.rs"]
}

invariant enhancement_builtin_precedence "Enhancement Built-in Precedence" {
  guarantee """
    Built-in field names MUST always take precedence over plugin
    enhancements. A plugin MUST NOT shadow or override a built-in
    field on any entity kind. Attempts to do so MUST produce E018
    regardless of enhancement_policy configuration.
  """
  enforced_by [register_entity_enhancements, detect_enhancement_conflicts]
  risk high

  verify unit "enhancement shadowing built-in produces E018"
  verify unit "E018 not configurable via enhancement_policy"

  tests ["../crates/specforge-validator/src/passes.rs"]
}
