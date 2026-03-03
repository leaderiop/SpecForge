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
}

invariant plugin_load_order_determinism "Plugin Load Order Determinism" {
  guarantee """
    Given the same set of installed plugins, the compiler MUST produce
    the same topological load order on every invocation. The ordering
    MUST be deterministic and reproducible across platforms.
  """
  enforced_by [topological_sort_plugins]
  risk medium

  verify property "same plugin set produces identical load order across 100 runs"
  verify unit "load order is deterministic across different platforms"
}

invariant peer_dependency_satisfaction "Peer Dependency Satisfaction" {
  guarantee """
    If a plugin declares peer dependencies, the compiler MUST verify that
    all declared peers are installed and satisfy the declared semver ranges.
    Unsatisfied peer dependencies MUST produce an error diagnostic, not
    a silent degradation.
  """
  enforced_by [validate_wasm_peer_dependencies]
  risk high

  verify unit "satisfied peer dependencies pass validation"
  verify unit "unsatisfied peer dependency produces an error diagnostic"
  verify unit "peer with wrong version range produces an error diagnostic"
}
