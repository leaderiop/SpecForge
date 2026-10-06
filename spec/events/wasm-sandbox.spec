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
