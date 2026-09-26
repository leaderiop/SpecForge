# Position — Andreas Rossberg (Wasm semantics; spec-as-toolchain)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. **Determinism is structural only in Wasm.** Only Wasm has a normative, executable semantics and conformance suite; mlua/PyO3/deno_core embed moving targets (Lua 5.4 vs LuaJIT, CPython point releases, V8) whose revisions shift plugin output — fatal to R-6 snapshots and R-4 reproducibility. A `.wasm` blob is a fixed byte artifact the signed registry already sha256-verifies.
2. **Isolation is architectural, not policy.** Linear memory plus capability imports give R-2 by construction; C7-04 (fs allow-by-default) is a host-policy bug fixable in-model. PyO3: no default sandbox, ambient-capability pip; QuickJS: none.
3. **The disease is drift, not the runtime.** C7-03: no IDL — even the "single source of truth" is a 566-line Rust-only crate (`crates/specforge-protocol-types/src/lib.rs`), sharing types only Rust↔Rust. C7-11's three parallel implementations and the C7-02/C7-08/C7-10 spec-vs-code gaps are exactly what one executable protocol definition cures — SpecTec-style, generating host bridge, SDK macros, and conformance tests from one source. KEEP_WASM plus that formalization is cheapest: guests are ~97% generated manifest, ~3% logic; no ecosystem pressure justifies Python/TS.

## Biggest risk in my verdict

KEEP_WASM without the protocol formalization perpetuates C7-03/C7-11; wasmtime's weight (~511 locked deps) and per-call compile persist while real AOT/pooling stays vaporware (C7-02, C7-08).

## What would change my mind

A language-neutral protocol spec landing first, then evidence that third parties author genuinely logic-bearing (not manifest-only) plugins in Python/TS — MULTI behind that spec wins. Or proof that real AOT/engine pooling is infeasible and cold-start dominates analyze latency.
