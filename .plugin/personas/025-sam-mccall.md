# Position — Sam McCall (clangd creator; LSP diagnostics)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Determinism is already the load-bearing contract.** `crates/specforge-lsp/src/lib.rs:85-89` pins `validator_dispatch_order` so CLI and LSP "agree byte for byte"; `backend.rs` keys per-URI diagnostics from the same build output. Wasm is a pure function of blob bytes + capability imports — R-6 nearly free. LuaJIT/Lua-5.4 forks, CPython hash randomization, and V8 JIT make interpreter determinism a permanent tax.
2. **The latency gaps are plumbing, not architecture.** C7-02 (AOT = byte-copy), C7-08 (EnginePool is a ledger), C7-10 (`max_execution_ms` unenforced) are pooling debt every production wasmtime host retires. An LSP publishing diagnostics under the `pending_updates` debounce (`backend.rs:43`) must also CPU-bound untrusted passes — wasmtime fuel/epoch interruption is the only candidate with instruction-grain enforcement.
3. **MULTI institutionalizes the known disease.** C7-11 found three parallel implementations; `extension_json_sync` exists to police the drift. A second runtime doubles that audit surface and breaks R-1's one-mechanism convergence.
4. **R-3 and sunk proof.** Wasmtime is static (no system packages); the 3,070-test suite includes wasm round-trips. PyO3 embed-and-ship is fragile; V8 is a heavy dep.

## Biggest risk in my verdict
The plumbing never lands: wasmtime's ~500-dependency weight taxes binary size while plugins pay cold compiles per call, and authoring stays Rust-toolchain-bound — AI-agent iteration and R-5 watch degrade to blob copies without guest rebuilds.

## What would change my mind

If warm-instance pooling cannot hold per-call overhead under the LSP debounce budget, or the registry shows non-Rust authors dominate — then one Lua tier with a deterministic profile and Luau-style instruction caps, builtins migrated, R-1 intact.
