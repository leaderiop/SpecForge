// Wasm/Extism runtime invariants

use behaviors/wasm-authoring
use behaviors/wasm-extensions
use behaviors/wasm-host-functions
use behaviors/wasm-lifecycle
use behaviors/wasm-sandbox
use behaviors/extensions
use behaviors/init

invariant wasm_sandbox_integrity "Wasm Sandbox Integrity" {
  guarantee """
    Wasm extensions MUST NOT escape the Extism sandbox. An extension MUST NOT
    access the host filesystem, network, or memory outside its linear
    memory region unless explicitly permitted by the sandbox policy.
    Any sandbox violation MUST trap the extension and emit a diagnostic.
  """
  enforced_by [enforce_wasm_sandbox, configure_sandbox_policy, load_wasm_module, provide_host_function_emit_file, provide_host_function_http_get, provide_host_function_query_graph, enforce_per_call_site_permissions, validate_wasm_extension_locally, handle_wasm_trap, dispatch_collector, update_all_extensions, support_private_registries, build_wasm_extension, invoke_extension_migration_hooks, scaffold_wasm_extension_project]
  risk high

  verify property "no extension can read or write outside its sandbox boundaries"
  verify unit "sandbox violation traps the extension and emits a diagnostic"

}

invariant extension_load_order_determinism "Extension Load Order Determinism" {
  guarantee """
    Given the same set of installed extensions, the compiler MUST produce
    the same topological load order on every invocation. The ordering
    MUST be deterministic and reproducible across platforms.
  """
  enforced_by [
    topological_sort_extensions, call_extension_validators,
    compose_query_files_from_extensions, dispatch_contribution_exports,
    write_lock_file, read_lock_file,
    toggle_extension_contributions, discover_extensions, load_extension_manifest,
    uninstall_wasm_extension, register_collector_contributions,
    load_extension_manifests,
    auto_detect_collector, invoke_extension_migration_hooks,
  ]
  risk medium

  verify property "same extension set produces identical load order across 100 runs"
  verify unit "load order is deterministic across different platforms"

}

invariant peer_dependency_satisfaction "Peer Dependency Satisfaction" {
  guarantee """
    If an extension declares peer dependencies, the compiler MUST verify that
    all declared peers are installed and satisfy the declared semver ranges.
    Unsatisfied peer dependencies MUST produce an error diagnostic (E-level), not
    a silent degradation.
  """
  enforced_by [validate_extension_peer_dependencies, initialize_wasm_extension, upgrade_wasm_extension, uninstall_wasm_extension, update_all_extensions, add_extension_to_existing_project]
  risk high

  verify unit "satisfied peer dependencies pass validation"
  verify unit "unsatisfied peer dependency produces an error diagnostic"
  verify unit "peer with wrong version range produces an error diagnostic"

}

// -- Cache & Isolation Invariants ---------------------------------------------

invariant aot_cache_integrity "AOT Cache Integrity" {
  guarantee """
    AOT artifacts MUST be verified on load by re-hashing the source .wasm
    binary and comparing against the cache key. Corrupted, truncated, or
    platform-mismatched cache entries MUST be automatically evicted and
    recompiled. The cache MUST NOT serve stale or invalid artifacts.
  """
  enforced_by [aot_compile_wasm_module, cache_aot_artifacts, invalidate_aot_cache, install_wasm_extension, upgrade_wasm_extension, verify_wasm_integrity, uninstall_wasm_extension, verify_registry_integrity, update_all_extensions, refresh_lock_file]
  risk medium

  verify property "corrupted AOT artifact is detected and recompiled"
  verify unit "platform-mismatched cache entry is evicted"

}

invariant extension_isolation "Extension Isolation" {
  guarantee """
    An extension failure MUST NOT affect other extensions or the host compiler.
    After an extension traps or fails during any lifecycle phase, the remaining
    extensions MUST continue execution normally. The failed extension MUST be
    excluded from subsequent phases in the current compilation.
  """
  enforced_by [handle_wasm_trap, enforce_wasm_sandbox, warm_wasm_engine_instance, evict_warm_engine_instance, provide_host_function_emit_file, provide_host_function_http_get, dispatch_collector, invoke_extension_migration_hooks, uninstall_wasm_extension]
  risk high

  verify property "extension trap does not affect other extensions"
  verify unit "failed extension excluded from subsequent phases"

}

invariant host_function_type_safety "Host Function Type Safety" {
  guarantee """
    Data exchanged between the host and extensions via host functions MUST
    conform to declared schemas. Malformed input from an extension MUST
    produce an ExtensionError diagnostic, not undefined behavior. The host
    MUST validate all extension-provided data before processing.
  """
  enforced_by [provide_host_function_query_graph, provide_host_function_emit_diagnostic, provide_host_function_add_graph_node, provide_host_function_add_graph_edge, provide_host_function_emit_file, provide_host_function_http_get, provide_extension_query_extensions, validate_contribution_exports, validate_extension_manifest]
  risk high

  verify unit "malformed extension input produces ExtensionError"
  verify unit "valid extension input is processed correctly"

}

// -- Entity Kind Invariants ---------------------------------------------------

invariant entity_kind_uniqueness "Entity Kind Uniqueness" {
  guarantee """
    No two extensions MAY register the same entity kind name without
    explicit resolution via the configured entity_kind_policy or
    entity_kinds overrides in specforge.json. Built-in keywords and
    DSL syntax words MUST always be rejected at registration time.
    Collisions with define block kinds MUST produce a hard error.
  """
  enforced_by [reject_reserved_entity_kind, detect_entity_kind_collision, resolve_entity_kind_conflict_via_config, qualify_entity_kind_inline]
  risk high

  verify property "no two extensions can silently register the same entity kind"
  verify unit "built-in keyword rejection is unconditional"

}

// -- Entity Enhancement Invariants --------------------------------------------

invariant enhancement_field_uniqueness "Enhancement Field Uniqueness" {
  guarantee """
    No two extensions MAY register the same field name for the same entity
    kind. When a conflict is detected, the compiler MUST resolve it
    according to the configured enhancement_policy or produce a hard
    error. The resolution MUST be deterministic and explicit.
  """
  enforced_by [detect_enhancement_conflicts, resolve_enhancement_conflicts, register_entity_enhancements]
  risk medium

  verify property "no two extensions can silently claim the same field"
  verify unit "conflict resolution is deterministic across runs"

}

invariant enhancement_builtin_precedence "Enhancement Built-in Precedence" {
  guarantee """
    Core grammar-level constructs MUST always take precedence over extension
    enhancements. An extension MUST NOT register an enhancement field whose
    name collides with a grammar-level construct: the entity title (the
    string after keyword and ID), verify statements, or gherkin
    declarations. These constructs are parsed by dedicated grammar
    rules and exist independently of the FieldRegistry. Attempts to
    shadow them MUST produce E018 regardless of enhancement_policy
    configuration.
  """
  enforced_by [register_entity_enhancements, detect_enhancement_conflicts]
  risk high

  verify unit "enhancement shadowing grammar-level construct produces E018"
  verify unit "E018 not configurable via enhancement_policy"

}

// -- Collector Invariants ---------------------------------------------------

invariant collector_output_conformance "Collector Output Conformance" {
  guarantee """
    Collector output MUST conform to the specforge-report/v1 schema. Every
    CollectorReport MUST include a valid schema field, entries array, and
    stats object. Entity IDs referenced in collector entries MUST be validated
    against the graph — unknown entity IDs MUST produce a W029 warning, not
    a hard error, to allow partial coverage ingestion.
  """
  enforced_by [validate_collector_output, ingest_collector_report, dispatch_collector]
  risk medium

  verify unit "valid collector output passes schema validation"
  verify unit "unknown entity ID in collector entry produces W029"
  verify unit "missing required fields produce hard error"

}

// -- Registry Invariants ----------------------------------------------------

invariant registry_integrity "Registry Integrity" {
  guarantee """
    Downloaded extension binaries from a registry MUST be verified against
    their declared SHA256 hash before installation. Hash mismatches MUST
    produce a hard error diagnostic and abort installation. The trust level
    of the source MUST be recorded in specforge.lock.
  """
  enforced_by [verify_registry_integrity, resolve_registry_source, publish_to_registry, publish_wasm_extension, verify_wasm_integrity, refresh_lock_file, authenticate_registry_request, validate_registry_credentials, support_private_registries, configure_registries, parse_extension_specifier, resolve_extension_source, logout_registry, generate_keyword_extension_index, retry_registry_request]
  risk high

  verify unit "SHA256 match passes verification"
  verify unit "SHA256 mismatch produces hard error and aborts"
  verify unit "trust level recorded in lock file"

}

invariant extension_operation_atomicity "Extension Operation Atomicity" {
  guarantee """
    Extension install, uninstall, and update operations MUST be atomic.
    On failure, all changes MUST be rolled back — no partial installs,
    no orphaned files, no inconsistent lock state.
  """
  enforced_by [install_wasm_extension, uninstall_wasm_extension, update_all_extensions, resolve_registry_source]
  risk high

  verify unit "failed install rolls back to previous state"
  verify unit "interrupted upgrade preserves original extension"
  verify integration "concurrent install and uninstall are serialized"
}

invariant credential_secrecy "Registry Credential Secrecy" {
  guarantee """
    Raw authentication tokens MUST never be logged, stored in
    specforge.json, or included in diagnostic output. Only token
    presence/absence and validity status may be reported.
  """
  enforced_by [authenticate_registry_request, validate_registry_credentials, support_private_registries, logout_registry]
  risk high

  verify unit "registry token is not included in log output"
  verify unit "diagnostic messages report credential presence not value"
  verify property "no log line contains raw token string"
}
