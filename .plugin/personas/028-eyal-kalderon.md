# Position — Eyal Kalderon (tower-lsp creator; LSP session lifecycle)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Session survival under crashing plugins. An LSP server is a long-lived session; any handler failure must degrade to a notification, never kill the process. SpecForge already treats plugin failure as a value end-to-end: `WasmCallResult::Trap` → E028 diagnostic, module state → Failed (`crates/specforge-wasm/src/lifecycle.rs`); pass traps become report warnings, not run failures (`crates/specforge-cli/src/analyze.rs`). Wasm traps are process-safe by construction; an embedded CPython (PyO3) or Lua/QuickJS crash — C stack overflow, FFI abort — takes the whole session down. Decisive for a server re-validating ~1.7k-entity graphs on debounced recomputes.
2. Concurrency shape. tower-lsp interleaves requests against shared state; plugin calls are already synchronous (`plugins.lock()` in `crates/specforge-extism/src/runtime.rs`), and C14-03/C14-04 show blocking-in-async is the live hazard. A GIL-bearing interpreter serializes interleaved requests; wasm instances are independent and poolable, so fixing C7-08 stays inside the model.
3. R-2/R-6. Memory isolation plus capability imports is the only candidate sandbox where deny-by-default (C7-04) is a config fix, not a redesign; deterministic snapshot output (R-6) follows; evidence.md's guest workload is ~97% manifest, ~3% logic.

## Biggest risk in my verdict
Cold instantiation on every debounced recompute if pooling/AOT (C7-02, C7-08) stays unfixed — interactive diagnostics latency becomes the wasm path's own worst enemy.

## What would change my mind
Proof that pooled instantiation cannot hit LSP-latency budgets on the watch path, or AI-agent authors categorically refusing Rust guest crates — then MULTI, with wasm as the sandboxed core tier.
