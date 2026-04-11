// Wasm module lifecycle behaviors — load, init, validate, deps, sorting,
// install, upgrade, uninstall, manifest validation, integrity

use "invariants/extensions"
use "invariants/wasm"
use "types/wasm"
use "types/config"
use "types/errors"
use "ports/outbound"
use "events/wasm-lifecycle"
// -- Wasm Module Lifecycle -----

behavior load_wasm_module "Load Wasm Module" {
  invariants [wasm_sandbox_integrity]
  category   command
  types      [ManifestV2, WasmModuleCache, ExtensionError]
  ports      [WasmRuntime]
  consumes   [manifest_validated, wasm_integrity_verified, extension_install_completed, extension_upgrade_completed]

  requires {
    manifest_validated_fired "manifest_validated event has fired, confirming manifest schema and required fields are valid"
    wasm_integrity_verified_fired "wasm_integrity_verified event has fired, confirming SHA256 hash matches lock file"
    wasm_runtime_available "WasmRuntime port is available for module loading"
  }

  ensures {
    extension_loaded_emitted "extension_loaded event is emitted on successful module load"
    aot_cache_utilized "AOT cache is checked and used on cache hit, skipping recompilation"
    missing_binary_diagnosed "missing .wasm binary produces ExtensionError diagnostic"
  }

  contract """
    When the compiler loads an extension, it MUST locate the .wasm binary
    from the manifest's wasmPath, check the AOT cache for a pre-compiled
    module matching the content hash, and load it into the Wasm runtime.
    Cache hits MUST skip recompilation. Missing .wasm files MUST produce
    a ExtensionError diagnostic.
  """

  produces [extension_loaded]

  verify unit "loads .wasm binary from manifest path"
  verify unit "uses AOT cache on cache hit"
  verify unit "missing .wasm produces ExtensionError"
  verify contract "requires/ensures consistency for Wasm module loading"

}

behavior initialize_wasm_extension "Initialize Wasm Extension" {
  invariants [peer_dependency_satisfaction]
  category   command
  types      [ManifestV2, ExtensionLifecycleState]
  ports      [WasmRuntime]
  consumes   [extension_loaded]

  requires {
    extension_loaded_fired "extension_loaded event has fired, confirming Wasm module is loaded into runtime"
    registries_populated "entity kinds, edge types, and validation rules are registered into KindRegistry and FieldRegistry before initialize() is called"
  }

  ensures {
    extension_initialized_emitted "extension_initialized event is emitted on successful initialization"
    lifecycle_state_updated "extension lifecycle transitions to initialized on success or failed on error"
    no_manifest_override "initialize() does not re-register or override manifest-declared registrations"
  }

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
  verify contract "requires/ensures consistency for Wasm extension initialization"

}

behavior call_extension_validators "Call Extension Validators" {
  invariants [extension_load_order_determinism]
  category   command
  types      [ManifestV2, ExtensionLifecycleState]
  ports      [WasmRuntime]
  consumes   [extension_initialized, extensions_sorted]

  requires {
    extension_initialized_fired "extension_initialized event has fired for all extensions"
    extensions_sorted_fired "extensions_sorted event has fired, confirming topological order is computed"
  }

  ensures {
    extension_validated_emitted "extension_validated event is emitted for each extension after validate() completes"
    diagnostics_collected "all diagnostics emitted via host function are collected by the compiler"
    validation_continues "validation continues to next extension after errors in any single extension"
  }

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
  verify contract "requires/ensures consistency for extension validator dispatch"

}

// -- Dependencies -----

behavior validate_extension_peer_dependencies "Validate Extension Peer Dependencies" {
  invariants [peer_dependency_satisfaction]
  category   validation
  types      [PeerDependency, ManifestV2, ExtensionError]

  requires {
    manifests_loaded "all extension manifests have been loaded and parsed"
  }

  ensures {
    peer_dependencies_validated_emitted "peer_dependencies_validated event is emitted when all peers are satisfied"
    unsatisfied_peers_diagnosed "unsatisfied peer dependencies produce hard error diagnostics"
  }

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
  verify contract "requires/ensures consistency for peer dependency validation"

}

behavior topological_sort_extensions "Topological Sort Extensions" {
  invariants [extension_load_order_determinism]
  category   command
  types      [PeerDependency, ManifestV2]
  consumes   [peer_dependencies_validated]

  requires {
    peer_dependencies_validated_fired "peer_dependencies_validated event has fired, confirming all peer dependencies are satisfied"
  }

  ensures {
    extensions_sorted_emitted "extensions_sorted event is emitted with the computed topological order"
    sort_deterministic "sort is deterministic with ties broken by extension name"
    cycles_diagnosed "cycles in peer dependencies produce an error diagnostic"
  }

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
  verify contract "requires/ensures consistency for topological extension sorting"

}

// -- Extension Lifecycle -----

behavior install_wasm_extension "Install Wasm Extension" {
  invariants [aot_cache_integrity, extension_operation_atomicity, offline_first_extension_resolution]
  category   command
  types      [ManifestV2, ExtensionInstallResult, ExtensionSource, ExtensionError]
  ports      [WasmRuntime, FileSystem]

  requires {
    extension_source_available "extension source (registry, local path, or git) is reachable"
    filesystem_available "FileSystem port is available for writing .wasm binary and specforge.json"
  }

  ensures {
    extension_install_completed_emitted "extension_install_completed event is emitted on successful install"
    integrity_verified "SHA256 integrity of downloaded .wasm binary is verified before placement"
    atomic_install_enforced "on failure at any step, all changes are rolled back — no partial installs"
    config_updated "specforge.json is updated with the extension entry"
  }

  contract """
    When specforge add <pkg> is invoked, the system MUST resolve the extension
    from its source (registry, local path, or git), download the .wasm
    binary, verify its SHA256 integrity, place it in the project, AOT compile
    it, and update specforge.json with the extension entry. On failure at any
    step, the system MUST rollback all changes — no partial installs.
    Per Principle 8 (seconds to value), installation MUST complete within
    the caller's time budget. When the remaining time budget is insufficient
    for AOT compilation, the system MUST defer AOT to first use — installing
    the raw .wasm binary and compiling on first load. This deferred-AOT
    strategy ensures P8 compliance even with slow networks or large binaries.
  """

  produces [extension_install_completed]

  verify unit "resolves extension from registry"
  verify unit "resolves extension from local path"
  verify unit "verifies SHA256 integrity of downloaded .wasm"
  verify unit "AOT compiles after install"
  verify unit "updates specforge.json with extension entry"
  verify unit "rolls back on download failure"
  verify performance "single extension install completes within 30 seconds on commodity hardware"
  verify unit "defers AOT compilation when time budget is insufficient"
  verify contract "requires/ensures consistency for Wasm extension installation"

}

behavior upgrade_wasm_extension "Upgrade Wasm Extension" {
  invariants [peer_dependency_satisfaction, aot_cache_integrity, extension_operation_atomicity]
  category   mutation
  types      [ManifestV2, PeerDependency, ExtensionInstallResult, ExtensionError]
  ports      [WasmRuntime, FileSystem]

  requires {
    extension_installed "target extension is currently installed with a valid manifest"
    source_available "extension source is reachable for version check"
  }

  ensures {
    extension_upgrade_completed_emitted "extension_upgrade_completed event is emitted on successful upgrade"
    old_cache_invalidated "old AOT cache entry is invalidated and binary is recompiled"
    peer_compatibility_enforced "breaking peer dependency changes are rejected without --force"
  }

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
  verify contract "requires/ensures consistency for Wasm extension upgrade"

}

// uninstall_wasm_extension is the Wasm lifecycle implementation for extension
// removal. It is called by remove_extension (behaviors/extensions.spec), which
// is the user-facing CLI entry point. This behavior handles all Wasm-specific
// cleanup; remove_extension handles CLI interaction and post-removal messaging.
behavior uninstall_wasm_extension "Uninstall Wasm Extension" {
  invariants [aot_cache_integrity, peer_dependency_satisfaction, extension_load_order_determinism, extension_operation_atomicity]
  category   command
  types      [ManifestV2, ExtensionInstallResult, ExtensionError]
  ports      [WasmRuntime, FileSystem]

  requires {
    extension_installed_ready "target extension is currently installed and its manifest is loaded"
    filesystem_available "FileSystem port is available for removing binary and updating config"
  }

  ensures {
    extension_unloaded_emitted "extension_unloaded event is emitted after warm engine instances are unloaded"
    wasm_extension_removed_emitted "wasm_extension_removed event is emitted after full cleanup"
    dependent_check_enforced "removal is rejected when dependents exist unless --force is provided"
    atomic_uninstall_enforced "on failure, all changes are rolled back"
  }

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
  verify contract "requires/ensures consistency for Wasm extension uninstall"
}

// -- Manifest Validation -----

behavior validate_extension_manifest "Validate Extension Manifest" {
  invariants [host_function_type_safety]
  category   validation
  types      [ManifestV2, ExtensionError]
  ports      [FileSystem]
  consumes   [manifest_loaded]

  requires {
    manifest_loaded_fired "manifest_loaded event has fired, confirming sidecar manifest.json has been parsed"
  }

  ensures {
    manifest_validated_emitted "manifest_validated event is emitted on successful validation"
    invalid_manifest_diagnosed "manifests with missing required fields or invalid manifest_version produce hard error"
    schema_validated "manifest schema is validated via validate_manifest_v2_schema"
  }

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
  verify contract "requires/ensures consistency for extension manifest validation"

}

behavior verify_wasm_integrity "Verify Wasm Integrity" {
  invariants [aot_cache_integrity, registry_integrity]
  category   validation
  types      [ManifestV2, LockFileEntry, ExtensionError]
  ports      [FileSystem]
  consumes   [lock_file_read]

  requires {
    lock_file_read_fired "lock_file_read event has fired, confirming lock file entries with expected hashes are available"
    filesystem_available "FileSystem port is available for reading .wasm binaries"
  }

  ensures {
    wasm_integrity_verified_emitted "wasm_integrity_verified event is emitted when hash matches"
    wasm_integrity_check_failed_emitted "wasm_integrity_check_failed event is emitted on hash mismatch"
    tampering_diagnosed "hash mismatches produce hard error diagnostic indicating potential tampering"
  }

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
  verify contract "requires/ensures consistency for Wasm integrity verification"

}

// -- Extension-Defined Grammar Loading ----------------------------------------

behavior load_extension_grammar "Load Extension Grammar" {
  invariants [aot_cache_integrity, grammar_injection_isolation]
  category   command
  types      [GrammarContribution, GrammarCacheEntry, GrammarError]
  ports      [WasmRuntime, FileSystem]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming grammar contributions are declared"
    wasm_runtime_available "WasmRuntime port is available for loading grammar binaries"
  }

  ensures {
    grammar_loaded_emitted "grammar_loaded event is emitted on successful grammar load"
    grammar_cached "successfully loaded grammars are cached via cache_grammar_artifacts"
    load_failure_diagnosed "loading failures produce GrammarError diagnostic without crashing"
  }

  contract """
    The system MUST load a grammar .wasm binary from the path declared in
    a GrammarContribution. Before loading, the system MUST validate the
    binary via validate_grammar_wasm. Successfully loaded grammars MUST be
    cached via cache_grammar_artifacts. Loading failures MUST produce a
    GrammarError diagnostic without crashing the compiler.
  """

  produces [grammar_loaded]

  verify unit "valid grammar .wasm loads successfully"
  verify unit "invalid grammar path produces GrammarError"
  verify unit "loaded grammar is cached for subsequent use"
  verify integration "grammar loading completes within performance budget"
  verify contract "requires/ensures consistency for extension grammar loading"

}

behavior validate_grammar_wasm "Validate Grammar Wasm" {
  invariants [grammar_injection_isolation]
  category   validation
  types      [GrammarContribution, GrammarError]

  requires {
    grammar_binary_available "grammar .wasm binary exists at the declared path"
  }

  ensures {
    abi_version_checked "ABI version is validated against host runtime's supported version"
    size_limit_enforced "binary size does not exceed configured limit (default 10MB)"
    language_export_verified "Wasm binary exports the expected tree-sitter language function"
  }

  contract """
    The system MUST validate a tree-sitter grammar .wasm binary before
    loading. Validation MUST check: (1) the Wasm binary exports the
    expected tree-sitter language function, (2) the ABI version matches
    the host runtime's supported ABI version, (3) the binary size does
    not exceed the configured limit (default 10MB). ABI version mismatches
    MUST produce a GrammarError with expected vs actual version. Oversized
    binaries MUST be rejected with a diagnostic.
  """

  verify unit "valid grammar passes all validation checks"
  verify unit "missing language export produces GrammarError"
  verify unit "ABI version mismatch produces GrammarError with versions"
  verify unit "oversized grammar binary is rejected"
  verify contract "requires/ensures consistency for grammar Wasm validation"

}

behavior compose_grammar_injections "Compose Grammar Injections" {
  invariants [grammar_composition_determinism, grammar_injection_isolation]
  category   command
  types      [GrammarContribution, GrammarConflictPolicy, KindRegistryEntry]

  requires {
    grammars_loaded "all grammar contributions have been loaded and validated"
    kind_registry_populated "KindRegistry contains all declared entity kinds for mapping"
  }

  ensures {
    grammars_composed_emitted "grammars_composed event is emitted with coherent grammar configuration"
    composition_deterministic "same set of extensions and policy always produces the same result"
    conflict_policy_applied "grammar conflicts resolved according to GrammarConflictPolicy"
  }

  contract """
    The system MUST compose grammar contributions from all loaded extensions
    into a coherent grammar configuration. Each entity kind MUST be mapped
    to at most one grammar .wasm (unless namespace policy is active). When
    multiple extensions declare grammars for the same entity kind, the
    GrammarConflictPolicy MUST be applied: error (default) fails fast,
    priority selects the higher-priority extension, namespace scopes both.
    The composition result MUST be deterministic given the same set of
    extensions and policy.
  """

  produces [grammars_composed]

  verify unit "single grammar per entity kind maps correctly"
  verify unit "conflict with error policy produces diagnostic"
  verify unit "conflict with priority policy selects higher priority"
  verify unit "conflict with namespace policy loads both grammars"
  verify property "same extensions + same policy = same composition"
  verify contract "requires/ensures consistency for grammar injection composition"

}

behavior dispatch_body_parser "Dispatch Body Parser" {
  invariants [body_parser_output_conformance, grammar_injection_isolation]
  category   command
  types      [BodyParserContribution, BodyParserError, FieldMap]
  ports      [WasmRuntime]

  requires {
    body_parser_registered "body parser contribution is registered for the target entity kind"
    wasm_runtime_available "WasmRuntime port is available for invoking parser export"
  }

  ensures {
    body_parsed_emitted "body_parsed event is emitted on successful parse"
    output_schema_validated "returned JSON is validated against declared output schema if present"
    timeout_enforced "timeout enforcement applies (configurable, default 5000ms)"
    fallback_on_failure "on parser crash, timeout, or schema violation, body is treated as raw string field"
  }

  contract """
    This behavior owns all Wasm execution mechanics for body parsing.
    When called by delegate_body_parsing_to_extension (behaviors/parsing.spec),
    the system MUST invoke the extension's body parse Wasm export with
    the raw body text. The export MUST return structured JSON fields.
    The system MUST validate the returned JSON against the declared
    output schema (if present). Timeout enforcement MUST apply
    (configurable, default 5000ms). On parser crash, timeout, or schema
    violation, the system MUST produce a BodyParserError and fall back
    to treating the body as a raw string field. This behavior owns all
    error handling and fallback logic for body parsing — the calling
    orchestrator (delegate_body_parsing_to_extension) handles only
    iteration and FieldMap replacement.
  """

  produces [body_parsed]

  verify unit "body parser called for entity kind with registered parser"
  verify unit "parser output validated against declared schema"
  verify unit "parser timeout produces BodyParserError"
  verify unit "parser crash produces BodyParserError with fallback"
  verify unit "entity kind without body parser uses default field parser"
  verify integration "body parser output feeds into Phase 2 validation"
  verify contract "requires/ensures consistency for body parser dispatch"

}

behavior cache_grammar_artifacts "Cache Grammar Artifacts" {
  invariants [aot_cache_integrity]
  category   command
  types      [GrammarCacheEntry, GrammarContribution]
  ports      [FileSystem]

  requires {
    grammar_validated "grammar .wasm binary has passed validation checks"
    filesystem_available "FileSystem port is available for writing cache entries"
  }

  ensures {
    cache_key_composite "cache key combines content hash and ABI version"
    cache_hit_skips_loading "cache hits skip grammar loading and validation on subsequent loads"
    cache_invalidation_enforced "cache is invalidated on content hash change, ABI version change, or specforge clean"
  }

  contract """
    The system MUST cache loaded grammar artifacts using a content-hash +
    ABI version composite cache key. Cache hits MUST skip grammar loading
    and validation. Cache MUST be invalidated when: (1) the grammar .wasm
    content hash changes, (2) the host ABI version changes, (3) the user
    runs specforge clean. The cache location MUST follow the existing AOT
    cache directory structure.
  """

  verify unit "cache key combines content hash and ABI version"
  verify unit "cache hit skips grammar loading"
  verify unit "content hash change invalidates cache"
  verify unit "ABI version change invalidates cache"
  verify contract "requires/ensures consistency for grammar artifact caching"

}
