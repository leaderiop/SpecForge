# Position — Syrus Akbary (Wasmer founder; wasm runtime landscape)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The sandbox is the one axis wasm wins on merit, not incumbency. R-2 demands deny-by-default capability isolation for untrusted plugins; wasmtime gives structural memory isolation plus capability-gated imports (undermined only by C7-04's fs allow-by-default — a bug, not a paradigm limit). Lua/QuickJS sandboxes are interpreter-convention, historically escapable; CPython's pip ecosystem is ambient capability, breaking R-3; deno_core drags V8 into the binary.
2. The swap point exists: `WasmRuntime` at `crates/specforge-wasm/src/runtime.rs:39-48` (`load_module`/`call_export`/`has_cached_module`). The honest fix for C7-06's false "wasm only" claim is proving that seam — embed a second engine (Wasmer Singlepass) cheaply, making the abstraction real, not decor.
3. The open audit items — C7-02 (AOT cache is a byte-copy), C7-08 (EnginePool ledger, zero warm instances), C7-10 (`max_execution_ms` never enforced) — are engine-hygiene debt solvable inside the model; per evidence.md the guest payload is ~97% generated manifest, so switching paradigms buys little ergonomics, and rewriting formal's ~478 lines of pass logic into Lua/TS is pure migration cost under R-1.

## Biggest risk in my verdict
If AI-authored third-party plugins with genuine logic become the product, the Rust→wasm toolchain tax strangles the registry bet; MULTI's scripting tier is the escape valve I refuse.

## What would change my mind
A second engine failing to fit behind `WasmRuntime` within days (the trait is a lie), or plugin needs crossing WASI into WASIX territory — threads, sockets — where interpreters degrade more gracefully than capability shims.
