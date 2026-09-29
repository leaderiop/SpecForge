// Wasm sandbox and lifecycle events

use "types/wasm"

event wasm_sandbox_violation "Wasm Sandbox Violation" {
  channel "wasm.sandbox_violation"
  payload {
    extensionName   string
    violationType   string
    attemptedAction string
    policyLimit     string
  }
  verify integration "emits wasm_sandbox_violation with violation details"
  verify integration "consumer handle_wasm_trap receives event and transitions extension to failed"
}

event wasm_trap_caught "Wasm Trap Caught" {
  channel "wasm.trap_caught"
  payload {
    extensionName string
    trapKind      string
    exportName    string
    message       string
  }
  verify integration "emits wasm_trap_caught with correct trapKind, exportName, and message"
}

// ── Sandbox Configuration Events ─────────────────────────────

event sandbox_policy_configured "Sandbox Policy Configured" {
  channel "wasm.sandbox_policy_configured"
  payload {
    extensionName      string
    maxMemoryMb        integer
    maxExecutionMs     integer
    allowedDomainCount integer
    allowedPathCount   integer
  }
  verify integration "emits sandbox_policy_configured with merged policy details"
}
