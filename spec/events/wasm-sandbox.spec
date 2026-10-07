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
