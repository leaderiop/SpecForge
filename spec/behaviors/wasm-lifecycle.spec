// Wasm module lifecycle behaviors — load, init, validate, deps, sorting,
// install, upgrade, uninstall, manifest validation, integrity

use invariants/extensions
use invariants/wasm
use types/wasm
use types/config
use types/errors
use ports/outbound
use events/wasm-lifecycle

// -- Wasm Module Lifecycle -----

behavior load_wasm_module "Load Wasm Module" {
  invariants [wasm_sandbox_integrity]
  types      [ManifestV2, WasmModuleCache, ExtensionError]
  ports      [WasmRuntime]

  contract """
    When the compiler loads an extension, it MUST locate the .wasm binary
    from the manifest's wasmPath, check the AOT cache for a pre-compiled
    module matching the content hash, and load it into the Extism runtime.
    Cache hits MUST skip recompilation. Missing .wasm files MUST produce
    a ExtensionError diagnostic.
  """

  produces [extension_loaded]

  verify unit "loads .wasm binary from manifest path"
  verify unit "uses AOT cache on cache hit"
  verify unit "missing .wasm produces ExtensionError"

}

behavior initialize_wasm_extension "Initialize Wasm Extension" {
  invariants [peer_dependency_satisfaction]
  types      [ManifestV2, ExtensionLifecycleState]
  ports      [WasmRuntime]

  contract """
    After loading a Wasm module, the compiler MUST call the extension's
    initialize() export. The initialize() call allows the extension to
    perform runtime setup (e.g., validating its own configuration).
    Entity kinds, edge types, and validation rules are registered
    declaratively from the manifest — NOT via host function calls
    during initialize(). See register_entity_kinds_from_manifest,
    register_edge_types_from_manifest, and
    register_validation_rules_from_manifest in behaviors/zero-entity-core.spec.

    TIMING GUARANTEE: Entity kinds, edge types, and validation rules
    MUST be registered into KindRegistry and FieldRegistry BEFORE the
    extension's initialize() export is invoked. This ensures that
    initialize() can query the registry for its own registered kinds
    and that cross-extension references resolve correctly during setup.

    The extension lifecycle MUST transition from loading to initialized
    on success, or to failed on error. The extension's initialize() export
    receives the registered state but MUST NOT re-register or override
    manifest-declared registrations.
  """

  produces [extension_initialized]

  verify unit "calls initialize() export on loaded module"
  verify unit "lifecycle transitions to initialized on success"
  verify unit "lifecycle transitions to failed on error"

}

behavior call_extension_validators "Call Extension Validators" {
  invariants [extension_load_order_determinism]
  types      [ManifestV2, ExtensionLifecycleState]
  ports      [WasmRuntime]

  contract """
    After all extensions are initialized, the compiler MUST call each
    extension's validate() export in topological order determined by
    peer dependencies. Extensions MUST emit diagnostics via the
    specforge.emit_diagnostic host function. The compiler MUST
    collect all diagnostics and continue to the next extension.
  """

  produces [extension_validated]

  verify unit "calls validate() in topological order"
  verify unit "diagnostics emitted via host function are collected"
  verify unit "validation continues to next extension after errors"

}

// -- Dependencies -----

behavior validate_extension_peer_dependencies "Validate Extension Peer Dependencies" {
  invariants [peer_dependency_satisfaction]
  types      [PeerDependency, ManifestV2, ExtensionError]

  contract """
    Before initializing extensions, the compiler MUST check that all
    declared peer dependencies are satisfied. For each peer dependency,
    the referenced extension MUST be installed and its version MUST match
    the declared semver range. Unsatisfied peers MUST produce a hard
    error diagnostic.
  """

  produces [peer_dependencies_validated]

  verify unit "satisfied peer dependency passes"
  verify unit "missing peer produces hard error"
  verify unit "version mismatch produces hard error"

}

behavior topological_sort_extensions "Topological Sort Extensions" {
  invariants [extension_load_order_determinism]
  types      [PeerDependency, ManifestV2]

  contract """
    The compiler MUST compute a topological order over installed extensions
    based on their peer dependencies. Extensions with no dependencies MUST
    be loaded first. Cycles in peer dependencies MUST produce an error
    diagnostic. The sort MUST be deterministic — ties broken by extension name.
  """

  produces [extensions_sorted]

  verify unit "extensions sorted in dependency order"
  verify unit "cycle in peer dependencies produces error"
  verify unit "deterministic ordering on ties"

}

// -- Extension Lifecycle -----

behavior install_wasm_extension "Install Wasm Extension" {
  invariants [aot_cache_integrity, extension_operation_atomicity, offline_first_extension_resolution]
  types      [ManifestV2, ExtensionInstallResult, ExtensionSource, ExtensionError]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge add <pkg> is invoked, the system MUST resolve the extension
    from its source (registry, local path, or git), download the .wasm
    binary, verify its SHA256 integrity, place it in the project, AOT compile
    it, and update specforge.json with the extension entry. On failure at any
    step, the system MUST rollback all changes — no partial installs.
  """

  produces [extension_install_completed]

  verify unit "resolves extension from registry"
  verify unit "resolves extension from local path"
  verify unit "verifies SHA256 integrity of downloaded .wasm"
  verify unit "AOT compiles after install"
  verify unit "updates specforge.json with extension entry"
  verify unit "rolls back on download failure"

}

behavior upgrade_wasm_extension "Upgrade Wasm Extension" {
  invariants [peer_dependency_satisfaction, aot_cache_integrity]
  types      [ManifestV2, PeerDependency, ExtensionInstallResult, ExtensionError]
  ports      [WasmRuntime, FileSystem]

  contract """
    When specforge extension upgrade is invoked, the system MUST check the
    source for a newer version, validate peer dependency compatibility
    with all installed extensions, replace the .wasm binary, invalidate the
    old AOT cache entry, and recompile. Breaking peer dependency changes
    MUST require the --force flag. Without --force, the upgrade MUST be
    rejected with a diagnostic listing the incompatible peers.
  """

  produces [extension_upgrade_completed]

  verify unit "checks source for newer version"
  verify unit "validates peer dependency compatibility"
  verify unit "replaces binary and invalidates old cache"
  verify unit "recompiles AOT after upgrade"
  verify unit "rejects breaking peer change without --force"

}

// uninstall_wasm_extension is the Wasm lifecycle implementation for extension
// removal. It is called by remove_extension (behaviors/extensions.spec), which
// is the user-facing CLI entry point. This behavior handles all Wasm-specific
// cleanup; remove_extension handles CLI interaction and post-removal messaging.
behavior uninstall_wasm_extension "Uninstall Wasm Extension" {
  invariants [aot_cache_integrity, peer_dependency_satisfaction, extension_load_order_determinism, extension_operation_atomicity]
  types      [ManifestV2, ExtensionInstallResult, ExtensionError]
  ports      [WasmRuntime, FileSystem]

  contract """
    When called by remove_extension (behaviors/extensions.spec), the system
    MUST perform the full Wasm lifecycle cleanup: remove the extension entry
    from specforge.json, delete the .wasm binary from the project, invalidate
    the AOT cache entry, and update specforge.lock. If other installed
    extensions declare a peer dependency on the removed extension, the system
    MUST reject the removal with a diagnostic listing the dependent extensions
    unless --force is provided. Warm engine instances MUST be unloaded. On
    failure, the system MUST rollback all changes.
  """

  produces [extension_unloaded, wasm_extension_removed]

  verify unit "removes extension entry from specforge.json"
  verify unit "deletes .wasm binary from project"
  verify unit "invalidates AOT cache entry"
  verify unit "updates specforge.lock after removal"
  verify unit "rejects removal when dependents exist without --force"
  verify unit "unloads warm engine instance"
  verify unit "rolls back on failure"
}

// -- Manifest Validation -----

behavior validate_extension_manifest "Validate Extension Manifest" {
  invariants [host_function_type_safety]
  types      [ManifestV2, ExtensionError]
  ports      [FileSystem]

  contract """
    This is the single entry point for manifest validation. The compiler
    MUST call this behavior once per extension manifest. It delegates to
    validate_manifest_v2_schema for schema validation. The entity_kinds
    array, if present, MUST contain valid ManifestEntityKind entries.
    Manifests missing required fields or with an invalid manifest_version
    MUST produce a hard error. The compiler MUST accept manifest_version
    2 (current). Unknown or missing manifest_version values MUST produce
    a hard error with diagnostic code E028. Future manifest versions
    MUST be rejected until the compiler is updated to support them.
  """

  produces [manifest_validated]

  verify unit "valid manifest passes validation"
  verify unit "missing required fields produce hard error"
  verify unit "unknown fields produce warning"
  verify unit "unknown manifest_version produces hard error"

}

behavior verify_wasm_integrity "Verify Wasm Integrity" {
  invariants [aot_cache_integrity, registry_integrity]
  types      [ManifestV2, LockFileEntry, ExtensionError]
  ports      [FileSystem]

  contract """
    The system MUST verify the SHA256 hash of each .wasm binary against
    the wasm_hash recorded in specforge.lock. Hash mismatches MUST produce
    a hard error diagnostic indicating potential tampering. The --skip-verify
    flag MUST bypass integrity checks with a warning.
  """

  produces [wasm_integrity_verified, wasm_integrity_check_failed]

  verify unit "matching hash passes verification"
  verify unit "mismatched hash produces hard error"
  verify unit "--skip-verify bypasses check with warning"

}
