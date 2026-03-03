// Wasm/Extism runtime events

use types/wasm
use behaviors/wasm
use behaviors/extensions

event package_loaded "Package Loaded" {
  trigger   load_wasm_module
  channel   "wasm.package_loaded"

  payload {
    packageName string
    wasmSize    integer
    fromCache   boolean
    loadTimeMs  integer
  }

  consumers [initialize_wasm_package]

  verify integration "emits package_loaded with correct packageName, wasmSize, fromCache, and loadTimeMs"
  verify integration "consumer initialize_wasm_package receives event and begins initialization"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event package_initialized "Package Initialized" {
  trigger   initialize_wasm_package
  channel   "wasm.package_initialized"

  payload {
    packageName    string
    entityCount    integer
    edgeTypeCount  integer
  }

  consumers [call_package_validators]

  verify integration "emits package_initialized with correct entityCount and edgeTypeCount"
  verify integration "consumer call_package_validators receives event and begins validation"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event package_validated "Package Validated" {
  trigger   call_package_validators
  channel   "wasm.package_validated"

  payload {
    packageName     string
    diagnosticCount integer
    durationMs      integer
  }

  consumers [call_package_generators]

  verify integration "emits package_validated with correct diagnosticCount and durationMs"
  verify integration "consumer call_package_generators receives event when generation is requested"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event package_unloaded "Package Unloaded" {
  trigger   warm_wasm_engine_instance
  channel   "wasm.package_unloaded"

  payload {
    packageName string
    reason      string
  }

  consumers [enforce_wasm_sandbox]

  verify integration "emits package_unloaded with correct packageName and reason"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event wasm_aot_compiled "Wasm AOT Compiled" {
  trigger   aot_compile_wasm_module
  channel   "wasm.aot_compiled"

  payload {
    packageName   string
    wasmHash      string
    aotSize       integer
    compileTimeMs integer
  }

  consumers [cache_aot_artifacts]

  verify integration "emits wasm_aot_compiled with correct wasmHash, aotSize, and compileTimeMs"
  verify integration "consumer cache_aot_artifacts receives event and stores compiled artifact"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Package Lifecycle Events ─────────────────────────────────

event package_install_completed "Package Install Completed" {
  trigger   install_wasm_package
  channel   "wasm.package_install_completed"

  payload {
    packageName   string
    version       string
    source        string
    aotCompiled   boolean
    installTimeMs integer
  }

  consumers [load_wasm_module]

  verify integration "emits package_install_completed with correct packageName, version, source, and installTimeMs"
  verify integration "consumer load_wasm_module receives event after install"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event package_removed "Package Removed" {
  trigger   remove_plugin
  channel   "wasm.package_removed"

  payload {
    packageName   string
    cacheCleared  boolean
  }

  consumers [invalidate_aot_cache]

  verify integration "emits package_removed with correct packageName and cacheCleared"
  verify integration "consumer invalidate_aot_cache receives event and clears cache"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event wasm_sandbox_violation "Wasm Sandbox Violation" {
  trigger   enforce_wasm_sandbox
  channel   "wasm.sandbox_violation"

  payload {
    packageName     string
    violationType   string
    attemptedAction string
    policyLimit     string
  }

  consumers [handle_wasm_trap]

  verify integration "emits wasm_sandbox_violation with violation details"
  verify integration "consumer handle_wasm_trap receives event and transitions package to failed"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event wasm_trap_caught "Wasm Trap Caught" {
  trigger   handle_wasm_trap
  channel   "wasm.trap_caught"

  payload {
    packageName string
    trapKind    string
    exportName  string
    message     string
  }

  consumers [run_doctor_check]

  verify integration "emits wasm_trap_caught with correct trapKind, exportName, and message"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event aot_cache_invalidated "AOT Cache Invalidated" {
  trigger   invalidate_aot_cache
  channel   "wasm.aot_cache_invalidated"

  payload {
    entriesRemoved  integer
    reason          string
    totalCacheSizeMb integer
  }

  consumers [aot_compile_wasm_module]

  verify integration "emits aot_cache_invalidated with correct entriesRemoved and reason"
  verify integration "consumer aot_compile_wasm_module receives event and recompiles"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// ── Entity Enhancement Events ────────────────────────────────

event enhancement_registered "Enhancement Registered" {
  trigger   register_entity_enhancements
  channel   "wasm.enhancement_registered"

  payload {
    packageName     string
    targetEntity    string
    fieldName       string
    fieldType       string
    isReference     boolean
  }

  consumers [run_doctor_check]

  verify integration "emits enhancement_registered with correct field details"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event enhancement_conflict_detected "Enhancement Conflict Detected" {
  trigger   detect_enhancement_conflicts
  channel   "wasm.enhancement_conflict_detected"

  payload {
    entityKind      string
    fieldName       string
    firstPackage    string
    secondPackage   string
    resolution      string
  }

  consumers [resolve_enhancement_conflicts]

  verify integration "emits enhancement_conflict_detected with both package identities"
  verify integration "consumer resolve_enhancement_conflicts receives event"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
