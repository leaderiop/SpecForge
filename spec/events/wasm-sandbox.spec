// Wasm sandbox and AOT cache events

use "types/wasm"
use "behaviors/wasm-sandbox"
event wasm_aot_compiled "Wasm AOT Compiled" {
  trigger   aot_compile_wasm_module
  channel   "wasm.aot_compiled"

  payload {
    extensionName   string
    wasmHash      string
    aotSize       integer
    compileTimeMs integer
  }

  consumers [cache_aot_artifacts]

  verify integration "emits wasm_aot_compiled with correct wasmHash, aotSize, and compileTimeMs"
  verify integration "consumer cache_aot_artifacts receives event and stores compiled artifact"

}

event wasm_sandbox_violation "Wasm Sandbox Violation" {
  trigger   enforce_wasm_sandbox
  channel   "wasm.sandbox_violation"

  payload {
    extensionName     string
    violationType   string
    attemptedAction string
    policyLimit     string
  }

  consumers [handle_wasm_trap]

  verify integration "emits wasm_sandbox_violation with violation details"
  verify integration "consumer handle_wasm_trap receives event and transitions extension to failed"

}

event wasm_trap_caught "Wasm Trap Caught" {
  trigger   handle_wasm_trap
  channel   "wasm.trap_caught"

  payload {
    extensionName string
    trapKind    string
    exportName  string
    message     string
  }

  consumers [run_doctor_check]

  verify integration "emits wasm_trap_caught with correct trapKind, exportName, and message"

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

}

// ── Sandbox Configuration Events ─────────────────────────────

event engine_warmed "Wasm Engine Warmed" {
  trigger   warm_wasm_engine_instance
  channel   "wasm.engine_warmed"

  payload {
    extensionName     string
    maxInstances      integer
    maxMemoryMb       integer
  }

  consumers [evict_warm_engine_instance]

  verify integration "emits engine_warmed with correct maxInstances and maxMemoryMb"
  verify integration "consumer evict_warm_engine_instance receives event and tracks warm instance"

}

event engine_evicted "Wasm Engine Evicted" {
  trigger   evict_warm_engine_instance
  channel   "wasm.engine_evicted"

  payload {
    extensionName     string
    reason            string
    instancesRemaining integer
  }

  consumers [warm_wasm_engine_instance]

  verify integration "emits engine_evicted with correct reason and instancesRemaining"
  verify integration "consumer warm_wasm_engine_instance receives event and may reclaim slot"

}

event sandbox_policy_configured "Sandbox Policy Configured" {
  trigger   configure_sandbox_policy
  channel   "wasm.sandbox_policy_configured"

  payload {
    extensionName     string
    maxMemoryMb       integer
    maxExecutionMs    integer
    allowedDomainCount integer
    allowedPathCount  integer
  }

  verify integration "emits sandbox_policy_configured with merged policy details"

}
