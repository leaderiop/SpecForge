# Position — Xavier Leroy (verified compilation, TCB discipline)

**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. TCB: wasm is the only candidate whose isolation rests on a machine-checked formal semantics (Watt et al.'s Isabelle mechanization of Wasm); mlua ships a C interpreter with no isolation theorem, V8 is an enormous unanalyzable TCB, and CPython's sandbox is disavowed upstream. The check-zero-entity-core.sh spirit — shrink what the core trusts — favors the engine with theorem-backed guarantees.
2. The audit indicts the host bridge, not the engine: C7-03 (no IDL), C7-04 (fs allow-by-default), C7-10 (max_execution_ms never enforced) are all host-side specification failures in crates/specforge-wasm, fixable in place. Switching discards ~9.4k LOC of working host code and trades hash-pinned deterministic blobs (R-4/R-6) for interpreter-version-dependent script execution.
3. The sync guards (extension_json_sync, builtin_blob_sync) are per-pass consistency theorems in miniature. R-1 convergence means deleting the native mirrors in crates/specforge-emitter/src/builtins/ (C7-11) — guests are ~97% generated manifest, so the endgame is a declarative manifest path plus wasm only for real logic. MULTI doubles the TCB by construction; no.
## Biggest risk in my verdict
The TCB is not just wasmtime: it is the bridge. If C7-04/C7-10 stay open, "sandbox" is paper — engine-level isolation theorem, host-level violation. Plus wasmtime's dependency weight (485→511 locked deps) is a cost KEEP_WASM keeps.
## What would change my mind
Measured proof that analyze passes over the 1.7k-entity graph cannot meet latency within wasm even with enforced pooling/AOT (C7-08), or a capability-gated interpreter with a machine-checked reference semantics at comparable weight.
