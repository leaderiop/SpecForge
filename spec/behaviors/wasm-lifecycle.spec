// Wasm module lifecycle behaviors — load, init, validate, deps, sorting,
// install, upgrade, uninstall, manifest validation, integrity

use "events/wasm-lifecycle"
use "invariants/extensions"
use "invariants/wasm"
use "ports/outbound"
use "types/config"
use "types/errors"
use "types/wasm"

// -- Wasm Module Lifecycle -----

behavior load_wasm_module "Load Wasm Module" {
  features   [wasm_extension_runtime]
  invariants [wasm_sandbox_integrity]
  category   command
  types      [ExtensionDeclaration, ExtensionError]
  ports      [WasmRuntime]
  consumes   [manifest_validated, extension_install_completed, extension_upgrade_completed]
  requires {
    manifest_validated_fired "manifest_validated event has fired, confirming manifest schema and required fields are valid"
    wasm_runtime_available   "WasmRuntime port is available for module loading"
  }
  ensures {
    extension_loaded_emitted     "extension_loaded event is emitted on successful module load"
    extension_loaded_via_runtime "the binary is loaded into the runtime engine (component compilation itself is cached by the engine — see compile_wasm_component_with_cache)"
    tampered_binary_refused      "a binary whose hash no longer matches the specforge.lock pin is refused with E033"
    missing_binary_diagnosed     "missing .wasm binary produces ExtensionError diagnostic"
  }
  contract   """
    When the compiler loads an extension, it MUST locate the .wasm binary
    from its installed path, verify its content hash against the
    specforge.lock pin (refusing a mismatch with E033; legacy entries
    without a hash warn and load), and load it into the Wasm runtime.
    Component compilation caching is the engine's concern (see
    compile_wasm_component_with_cache). Missing .wasm files MUST produce
    an ExtensionError diagnostic.
  """
  produces   [extension_loaded]
  verify unit "loads .wasm binary from manifest path"
  verify unit "tampered installed binary refused via E033 lockfile pin"
  verify unit "legacy lockfile entry without hash loads unchanged"
  verify unit "missing .wasm produces ExtensionError"
  verify contract "Load Wasm Module: Wasm module loading holds — manifest_validated_fired, wasm_runtime_available, extension_loaded_emitted, extension_loaded_via_runtime, tampered_binary_refused, missing_binary_diagnosed"
}

behavior initialize_wasm_extension "Initialize Wasm Extension" {
  features   [wasm_extension_runtime]
  invariants [peer_dependency_satisfaction]
  category   command
  types      [ExtensionDeclaration]
  ports      [WasmRuntime]
  consumes   [extension_loaded]
  requires {
    extension_loaded_fired "extension_loaded event has fired, confirming Wasm module is loaded into runtime"
    registries_populated   "entity kinds, edge types, and validation rules are registered into KindRegistry and FieldRegistry before initialize() is called"
  }
  ensures {
    extension_initialized_emitted "extension_initialized event is emitted on successful initialization"
    lifecycle_state_updated       "extension lifecycle transitions to initialized on success or failed on error"
    no_manifest_override          "initialize() does not re-register or override manifest-declared registrations"
  }
  contract   """
    After loading a Wasm module, the compiler MUST call the extension's
    initialize() export. The initialize() call allows the extension to
    perform runtime setup (e.g., validating its own configuration).
    Entity kinds, edge types, and validation rules are registered
    declaratively from the extension's declaration — NOT via host
    function calls during initialize(). See registry_build_kinds,
    registry_build_edges and registry_build_rules in
    behaviors/zero-entity-registries.spec.

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
  produces   [extension_initialized]
  verify unit "calls initialize() export on loaded module"
  verify unit "lifecycle transitions to initialized on success"
  verify unit "lifecycle transitions to failed on error"
  verify contract "Initialize Wasm Extension: Wasm extension initialization holds — extension_loaded_fired, registries_populated, extension_initialized_emitted, lifecycle_state_updated, no_manifest_override"
}

// -- Dependencies -----

behavior validate_extension_peer_dependencies "Validate Extension Peer Dependencies" {
  features   [wasm_extension_runtime]
  invariants [peer_dependency_satisfaction]
  category   validation
  types      [PeerDependency, ExtensionDeclaration, ExtensionError]
  requires {
    manifests_loaded "all extension manifests have been loaded and parsed"
  }
  ensures {
    peer_dependencies_validated_emitted "peer_dependencies_validated event is emitted when all peers are satisfied"
    unsatisfied_peers_diagnosed         "unsatisfied peer dependencies produce hard error diagnostics"
  }
  contract   """
    Before initializing extensions, the compiler MUST check that all
    declared peer dependencies are satisfied. For each peer dependency,
    the referenced extension MUST be installed and its version MUST match
    the declared semver range. Unsatisfied peers MUST produce a hard
    error diagnostic.
  """
  produces   [peer_dependencies_validated]
  verify unit "satisfied peer dependency passes"
  verify unit "missing peer produces hard error"
  verify unit "version mismatch produces hard error"
  verify contract "Validate Extension Peer Dependencies: peer dependency validation holds — manifests_loaded, peer_dependencies_validated_emitted, unsatisfied_peers_diagnosed"
}

behavior topological_sort_extensions "Topological Sort Extensions" {
  features   [wasm_extension_runtime]
  invariants [extension_load_order_determinism]
  category   command
  types      [PeerDependency, ExtensionDeclaration]
  consumes   [peer_dependencies_validated]
  requires {
    peer_dependencies_validated_fired "peer_dependencies_validated event has fired, confirming all peer dependencies are satisfied"
  }
  ensures {
    extensions_sorted_emitted "extensions_sorted event is emitted with the computed topological order"
    sort_deterministic        "sort is deterministic with ties broken by extension name"
    cycles_diagnosed          "cycles in peer dependencies produce an error diagnostic"
  }
  contract   """
    The compiler MUST compute a topological order over installed extensions
    based on their peer dependencies. Extensions with no dependencies MUST
    be loaded first. A cycle among required peer dependencies MUST produce
    an error diagnostic (E027). An optional peer only prefers a load order:
    extensions MAY name each other as optional peers, so the required
    edges are sorted first and the optional ones are added in name order,
    each skipped when it would close a cycle. The sort MUST be
    deterministic — ties broken by extension name.
  """
  produces   [extensions_sorted]
  verify unit "extensions sorted in dependency order"
  verify unit "cycle in peer dependencies produces error"
  verify unit "extensions naming each other as optional peers sort without a cycle"
  verify unit "a cycle among required peers is E027, an optional edge closing a cycle is dropped"
  verify unit "deterministic ordering on ties"
  verify contract "Topological Sort Extensions: topological extension sorting holds — peer_dependencies_validated_fired, extensions_sorted_emitted, sort_deterministic, cycles_diagnosed"
}

// -- Extension Lifecycle -----

behavior install_wasm_extension "Install Wasm Extension" {
  features   [wasm_extension_installation]
  invariants [
    wasm_compile_cache_integrity,
    extension_operation_atomicity,
    offline_first_extension_resolution,
  ]
  category   command
  types      [ExtensionDeclaration, ExtensionInstallResult, ExtensionSource, ExtensionError]
  ports      [WasmRuntime, FileSystem]
  requires {
    extension_source_available "extension source (registry, local path, or git) is reachable"
    filesystem_available       "FileSystem port is available for writing .wasm binary and specforge.json"
  }
  ensures {
    extension_install_completed_emitted "extension_install_completed event is emitted on successful install"
    integrity_verified                  "SHA256 integrity of downloaded .wasm binary is verified before placement"
    atomic_install_enforced             "on failure at any step, all changes are rolled back — no partial installs"
    config_updated                      "specforge.json is updated with the extension entry"
  }
  contract   """
    When specforge add <pkg> is invoked, the system MUST resolve the
    extension from its source (registry, local path, or git), download
    the .wasm binary, verify its SHA256 integrity, place it in the
    project atomically (temp dir + rename), and update specforge.json
    with the extension entry. On failure at any step, the system MUST
    rollback all changes — no partial installs. Component compilation
    is NOT an install step: the engine compiles on first load and
    caches the artifact (see compile_wasm_component_with_cache), so a
    slow network or large binary never blocks install.
  """
  produces   [extension_install_completed]
  verify unit "resolves extension from registry"
  verify unit "resolves extension from local path"
  verify unit "verifies SHA256 integrity of downloaded .wasm"
  verify unit "places binary atomically via temp dir"
  verify unit "updates specforge.json with extension entry"
  verify unit "rolls back on download failure"
  verify unit "an extension is installed under the extensions directory of its project, by its package name"
  verify performance "single extension install completes within 30 seconds on commodity hardware"
  verify contract "Install Wasm Extension: Wasm extension installation holds — extension_source_available, filesystem_available, extension_install_completed_emitted, integrity_verified, atomic_install_enforced, config_updated"
}

behavior upgrade_wasm_extension "Upgrade Wasm Extension" {
  features   [wasm_extension_installation]
  invariants [peer_dependency_satisfaction, extension_operation_atomicity]
  category   mutation
  types      [ExtensionDeclaration, PeerDependency, ExtensionInstallResult, ExtensionError]
  ports      [WasmRuntime, FileSystem]
  requires {
    extension_installed "target extension is currently installed with a valid manifest"
    source_available    "extension source is reachable for version check"
  }
  ensures {
    extension_upgrade_completed_emitted "extension_upgrade_completed event is emitted on successful upgrade"
    binary_replaced                     "the installed .wasm binary is replaced and the lock entry records the new hash"
    peer_compatibility_enforced         "breaking peer dependency changes are rejected without --force"
  }
  contract   """
    When specforge extension upgrade is invoked, the system MUST check the
    source for a newer version, validate peer dependency compatibility
    with all installed extensions, and replace the .wasm binary. The lock
    entry MUST record the new hash, which becomes the tamper pin for
    subsequent loads. The engine's compile cache keys on binary content,
    so the new binary never resolves to the previous artifact. Breaking
    peer dependency changes MUST require the --force flag. Without
    --force, the upgrade MUST be rejected with a diagnostic listing the
    incompatible peers.
  """
  produces   [extension_upgrade_completed]
  verify unit "checks source for newer version"
  verify unit "validates peer dependency compatibility"
  verify unit "replaces binary and records new lock hash"
  verify unit "rejects breaking peer change without --force"
  verify contract "Upgrade Wasm Extension: Wasm extension upgrade holds — extension_installed, source_available, extension_upgrade_completed_emitted, binary_replaced, peer_compatibility_enforced"
}

// uninstall_wasm_extension is the Wasm lifecycle implementation for extension
// removal. It is called by remove_extension (behaviors/extensions.spec), which
// is the user-facing CLI entry point. This behavior handles all Wasm-specific
// cleanup; remove_extension handles CLI interaction and post-removal messaging.
behavior uninstall_wasm_extension "Uninstall Wasm Extension" {
  features   [wasm_extension_installation]
  invariants [
    peer_dependency_satisfaction,
    extension_load_order_determinism,
    extension_operation_atomicity,
  ]
  category   command
  types      [ExtensionDeclaration, ExtensionInstallResult, ExtensionError]
  ports      [WasmRuntime, FileSystem]
  requires {
    extension_installed_ready "target extension is currently installed and its manifest is loaded"
    filesystem_available      "FileSystem port is available for removing binary and updating config"
  }
  ensures {
    extension_unloaded_emitted     "extension_unloaded event is emitted after the extension is unloaded"
    wasm_extension_removed_emitted "wasm_extension_removed event is emitted after full cleanup"
    dependent_check_enforced       "removal is rejected when dependents exist unless --force is provided"
    atomic_uninstall_enforced      "on failure, all changes are rolled back"
  }
  contract   """
    When called by remove_extension (behaviors/extensions.spec), the system
    MUST perform the full Wasm lifecycle cleanup: remove the extension entry
    from specforge.json, delete the .wasm binary from the project, and
    update specforge.lock. If other installed extensions declare a peer
    dependency on the removed extension, the system MUST reject the removal
    with a diagnostic listing the dependent extensions unless --force is
    provided. The session runtime drops the extension's loaded instance
    (see reuse_session_runtime); stale engine cache entries are inert —
    cache keys include binary content. On failure, the system MUST
    rollback all changes.
  """
  produces   [extension_unloaded, wasm_extension_removed]
  verify unit "removes extension entry from specforge.json"
  verify unit "deletes .wasm binary from project"
  verify unit "rolls back on failure"
  verify contract "Uninstall Wasm Extension: Wasm extension uninstall holds — extension_installed_ready, filesystem_available, extension_unloaded_emitted, wasm_extension_removed_emitted, dependent_check_enforced, atomic_uninstall_enforced"
}

// -- Manifest Validation -----

behavior validate_extension_manifest "Validate Extension Manifest" {
  features   [contribution_based_extensions]
  invariants [host_function_type_safety]
  category   validation
  types      [ExtensionDeclaration, ExtensionError]
  ports      [FileSystem]
  consumes   [manifest_loaded]
  requires {
    declaration_loaded_fired "the extension's declaration has been loaded (load_extension_declaration)"
  }
  ensures {
    manifest_validated_emitted "manifest_validated event is emitted on successful validation"
    invalid_manifest_diagnosed "declarations with missing required fields, or a handshake of another protocol major, produce a hard error"
    schema_validated           "the declaration is validated by the registry build (build_registries_from_declarations)"
  }
  contract   """
    This is the single entry point for validating what an extension
    declares. The compiler MUST call this behavior once per loaded
    declaration (no sidecar manifest file is read). It delegates to
    the registry build (build_registries_from_declarations) for the
    declaration's validation. The
    entities, if present, MUST be valid entity kind descriptors.
    Declarations missing required fields MUST produce a hard error. A
    handshake whose protocol major version differs from the host's MUST
    fail the extension's load with E028.
  """
  produces   [manifest_validated]
  verify unit "valid manifest passes validation"
  verify unit "missing required fields produce hard error"
  verify unit "an unknown describe key produces W138"
  verify unit "a handshake whose protocol major differs from the host's produces E028"
  verify contract "Validate Extension Manifest: extension declaration validation holds — declaration_loaded_fired, manifest_validated_emitted, invalid_manifest_diagnosed, schema_validated"
}
