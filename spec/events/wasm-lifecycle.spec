// Wasm extension lifecycle events — load, init, validate, unload, deps,
// sorting, manifest, integrity, install, remove, upgrade

use types/wasm
use behaviors/wasm-lifecycle
use behaviors/wasm-sandbox
use behaviors/surface-contributions

event extension_loaded "Extension Loaded" {
  trigger   load_wasm_module
  channel   "wasm.extension_loaded"

  payload {
    extensionName string
    wasmSize    integer
    fromCache   boolean
    loadTimeMs  integer
  }

  consumers [initialize_wasm_extension]

  verify integration "emits extension_loaded with correct extensionName, wasmSize, fromCache, and loadTimeMs"
  verify integration "consumer initialize_wasm_extension receives event and begins initialization"

}

event extension_initialized "Extension Initialized" {
  trigger   initialize_wasm_extension
  channel   "wasm.extension_initialized"

  payload {
    extensionName    string
    entityCount    integer
    edgeTypeCount  integer
  }

  consumers [call_extension_validators]

  verify integration "emits extension_initialized with correct entityCount and edgeTypeCount"
  verify integration "consumer call_extension_validators receives event and begins validation"

}

event extension_validated "Extension Validated" {
  trigger   call_extension_validators
  channel   "wasm.extension_validated"

  payload {
    extensionName     string
    diagnosticCount integer
    durationMs      integer
  }

  verify integration "emits extension_validated with correct diagnosticCount and durationMs"

}

event extension_unloaded "Extension Unloaded" {
  trigger   uninstall_wasm_extension
  channel   "wasm.extension_unloaded"

  payload {
    extensionName string
    reason      string
  }

  consumers []

  verify integration "emits extension_unloaded with correct extensionName and reason"

}

// ── Runtime Observability Events ───────────────────────────────

event peer_dependencies_validated "Peer Dependencies Validated" {
  trigger   validate_extension_peer_dependencies
  channel   "wasm.peer_dependencies_validated"

  payload {
    extensionName     string
    peerCount         integer
    allSatisfied      boolean
  }

  consumers [topological_sort_extensions]

  verify integration "emits peer_dependencies_validated with correct peerCount and satisfaction status"

}

event extensions_sorted "Extensions Sorted" {
  trigger   topological_sort_extensions
  channel   "wasm.extensions_sorted"

  payload {
    extensionCount    integer
    loadOrder         string[]
    hasCycles         boolean
  }

  consumers [call_extension_validators]

  verify integration "emits extensions_sorted with deterministic loadOrder"

}

event manifest_validated "Manifest Validated" {
  trigger   validate_extension_manifest
  channel   "wasm.manifest_validated"

  payload {
    extensionName     string
    manifestVersion   integer
    entityKindCount   integer
    validationPassed  boolean
  }

  consumers [load_wasm_module, register_surface_contributions]

  verify integration "emits manifest_validated with correct entityKindCount and validation status"

}

event wasm_integrity_verified "Wasm Integrity Verified" {
  trigger   verify_wasm_integrity
  channel   "wasm.integrity_verified"

  payload {
    extensionName   string
    wasmHash        string
    verified        boolean
  }

  consumers [load_wasm_module]

  verify integration "emits wasm_integrity_verified with correct hash after successful check"

}

// ── Extension Lifecycle Events ─────────────────────────────────

event extension_install_completed "Extension Install Completed" {
  trigger   install_wasm_extension
  channel   "wasm.extension_install_completed"

  payload {
    extensionName   string
    version       string
    source        string
    aotCompiled   boolean
    installTimeMs integer
  }

  consumers [load_wasm_module]

  verify integration "emits extension_install_completed with correct extensionName, version, source, and installTimeMs"
  verify integration "consumer load_wasm_module receives event after install"

}

event wasm_extension_removed "Wasm Extension Removed" {
  trigger   uninstall_wasm_extension
  channel   "wasm.extension_removed"

  payload {
    extensionName   string
    cacheCleared  boolean
  }

  consumers [invalidate_aot_cache]

  verify integration "emits extension_removed with correct extensionName and cacheCleared"
  verify integration "consumer invalidate_aot_cache receives event and clears cache"

}

event extension_upgrade_completed "Extension Upgrade Completed" {
  trigger   upgrade_wasm_extension
  channel   "wasm.extension_upgrade_completed"

  payload {
    extensionName     string
    previousVersion   string
    newVersion        string
    peerCheckPassed   boolean
    aotRecompiled     boolean
  }

  consumers [load_wasm_module]

  verify integration "emits extension_upgrade_completed with correct previousVersion and newVersion"
  verify integration "consumer load_wasm_module receives event after upgrade"

}
