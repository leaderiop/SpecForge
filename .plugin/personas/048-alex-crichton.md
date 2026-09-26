# Position — Alex Crichton (wit-bindgen / wasmtime; ex-Rust & Cargo core)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The audit's worst wasm finding isn't about wasm: C7-03 (no IDL; stringly JSON over `call_export`) would afflict Lua/TS identically. The cure is a typed interface — model `__handshake`/`__describe`/`__pass_*` as a WIT world with generated guest/host bindings (wit-bindgen), replacing the hand-rolled structs in `crates/specforge-wasm/src/protocol/types.rs`; versioning moves into the component, not the handshake's semver string.
2. R-2 is structural only under wasm: a module cannot touch memory, files, or sockets it is not imported. C7-04 is a default-flip in `crates/specforge-wasm/src/sandbox.rs`, not architecture. PyO3 ships no sandbox plus an ambient-capability pip ecosystem; QuickJS has none built-in (evidence §4) — either violates R-2 by construction.
3. R-3/R-4 already work: static wasmtime, one binary, sha256-verified vendored blobs (`crates/specforge-extism/src/builtins.rs`). Remaining C7 debt is engineering, not runtime choice — C7-08/C7-10 fall to wasmtime pooling + epoch interruption/fuel, C7-02 to real `Engine::precompile` AOT, C7-11 by deleting the `crates/specforge-emitter/src/builtins/` mirrors.

## Biggest risk in my verdict
Wasmtime's weight (≈511 locked deps) plus unfixed per-call compilation (C7-08) makes KEEP_WASM slow in practice; and if AI-agent authors cannot write Rust guests, the registry bet dies on ergonomics, not safety.

## What would change my mind
Measured proof that pooled/AOT wasmtime still misses R-5/R-6 latency budgets, or that V8 isolates meet R-2 at wasm-grade capability granularity for hostile code. Failing both, add TypeScript as a WIT-first toolchain (generated TS bindings to the same world), not a second runtime.
