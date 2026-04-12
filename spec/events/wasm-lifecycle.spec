// Wasm extension lifecycle events — load, init, validate, unload, deps,
// sorting, manifest, integrity, install, remove, upgrade

use "types/wasm"
event extension_loaded "Extension Loaded" {
  channel   "wasm.extension_loaded"

  payload {
    extensionName string
    wasmSize    integer
    fromCache   boolean
    loadTimeMs  integer
  }


  verify integration "emits extension_loaded with correct extensionName, wasmSize, fromCache, and loadTimeMs"
  verify integration "consumer initialize_wasm_extension receives event and begins initialization"

}

event extension_initialized "Extension Initialized" {
  channel   "wasm.extension_initialized"

  payload {
    extensionName    string
    entityCount    integer
    edgeTypeCount  integer
  }


  verify integration "emits extension_initialized with correct entityCount and edgeTypeCount"
  verify integration "consumer call_extension_validators receives event and begins validation"

}

event extension_validated "Extension Validated" {
  channel   "wasm.extension_validated"

  payload {
    extensionName     string
    diagnosticCount integer
    durationMs      integer
  }

  verify integration "emits extension_validated with correct diagnosticCount and durationMs"

}

event extension_unloaded "Extension Unloaded" {
  channel   "wasm.extension_unloaded"

  payload {
    extensionName string
    reason      string
  }


  verify integration "emits extension_unloaded with correct extensionName and reason"

}

// ── Runtime Observability Events ───────────────────────────────

event peer_dependencies_validated "Peer Dependencies Validated" {
  channel   "wasm.peer_dependencies_validated"

  payload {
    extensionName     string
    peerCount         integer
    allSatisfied      boolean
  }


  verify integration "emits peer_dependencies_validated with correct peerCount and satisfaction status"

}

event extensions_sorted "Extensions Sorted" {
  channel   "wasm.extensions_sorted"

  payload {
    extensionCount    integer
    loadOrder         string[]
    hasCycles         boolean
  }


  verify integration "emits extensions_sorted with deterministic loadOrder"

}

event manifest_validated "Manifest Validated" {
  channel   "wasm.manifest_validated"

  payload {
    extensionName     string
    manifestVersion   integer
    entityKindCount   integer
    validationPassed  boolean
  }


  verify integration "emits manifest_validated with correct entityKindCount and validation status"

}

event wasm_integrity_verified "Wasm Integrity Verified" {
  channel   "wasm.integrity_verified"

  payload {
    extensionName   string
    wasmHash        string
    verified        boolean
  }


  verify integration "emits wasm_integrity_verified with correct hash after successful check"

}

// ── Extension Lifecycle Events ─────────────────────────────────

event extension_install_completed "Extension Install Completed" {
  channel   "wasm.extension_install_completed"

  payload {
    extensionName   string
    version       string
    source        string
    aotCompiled   boolean
    installTimeMs integer
  }


  verify integration "emits extension_install_completed with correct extensionName, version, source, and installTimeMs"
  verify integration "consumer load_wasm_module receives event after install"

}

event wasm_extension_removed "Wasm Extension Removed" {
  channel   "wasm.extension_removed"

  payload {
    extensionName   string
    cacheCleared  boolean
  }


  verify integration "emits extension_removed with correct extensionName and cacheCleared"
  verify integration "consumer invalidate_aot_cache receives event and clears cache"

}

event extension_upgrade_completed "Extension Upgrade Completed" {
  channel   "wasm.extension_upgrade_completed"

  payload {
    extensionName     string
    previousVersion   string
    newVersion        string
    peerCheckPassed   boolean
    aotRecompiled     boolean
  }


  verify integration "emits extension_upgrade_completed with correct previousVersion and newVersion"
  verify integration "consumer load_wasm_module receives event after upgrade"

}
