# Position — Dan Gohman (WASI lead; capability-based sandboxing)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Only wasm makes R-2 structural, not conventional. `wasm32-unknown-unknown` guests import nothing ambient — every capability is an explicit host call (`query`, `emit_diagnostic`, `resolve_ref`, `read_file`) mediated by `crates/specforge-wasm/src/sandbox.rs`: empty `allowed_domains`/`allowed_paths`, `network_access: false`, output-extension allowlist, most-restrictive-wins merge (`min_opt`, `intersection`), 64 MB ceiling. Lua/QuickJS sandboxes are interpreter discipline; evidence.md itself scores Python "none by default" and quickjs "none built-in". deno_core grants are process-level, not memory-isolated per plugin.
2. R-3/R-4 favor static wasm: wasmtime compiles into the single binary, and plugin blobs are sha256-pinned signed registry artifacts (evidence §1.4); CPython embed-and-ship is "notoriously fragile".
3. The audit record — C7-04 (`file_system_access: Some(true)` contradicting empty `allowed_paths` in the same default struct), C7-02, C7-08, C7-10 — is all fixable inside the wasm model. Switching discards 9.4k LOC of isolation machinery plus 3,070 green tests to re-buy the same guarantees worse, for a guest workload that is ~97% manifest, ~3% logic (evidence §2).

## Biggest risk in my verdict
The team's wasm discipline is unproven: C7-09/C7-10 show policy knobs silently ignored, so "structural" stays aspirational until C7-04 and C7-10 actually close; wasmtime dependency weight and the missing IDL (C7-03) are real, compounding debts.

## What would change my mind
Proof that hot-reload (R-5) per-call overhead cannot be fixed with genuine engine pooling/AOT (not the C7-02 byte-copy), or a rival runtime offering per-plugin memory isolation plus static single-binary distribution at comparable cost.
