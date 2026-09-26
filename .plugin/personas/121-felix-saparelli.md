# Position — Félix Saparelli (notify maintainer; watch/invalidation infrastructure)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. R-5 is my lane. `crates/specforge-watch/src/watcher.rs` debounces at 50 ms and feeds `IncrementalPipeline` (`pipeline.rs`), whose `verify_incremental` diffs incremental vs cold rebuilds byte-for-byte — every batch re-invokes plugins under a hard determinism budget. Wasm reload is cheap *if* C7-02 (real module cache, not the byte-copy) and C7-08 (pooled warm instances) are fixed: two contained fixes inside an existing ~9.4k-LOC runtime, not a new platform matrix.
2. R-3 is the watchexec/notify distribution lesson: one static binary, identical semantics per OS. wasmtime 43 is heavy (485→511 locked deps) but static and uniform; PyO3 fails outright (system Python per machine); deno_core adds V8 weight; QuickJS adds vendored-C quirks of the notify-backend kind.
3. R-6: wasm is deterministic by construction — no ambient time/fs unless imported as capabilities. Interpreters make determinism a per-plugin discipline; `debounce.rs` batching assumes identical input → identical diagnostics.

## Biggest risk in my verdict
"Keep" is honest only if C7-02/04/08/10 actually close; otherwise I endorse vaporware, and a heavy runtime with allow-by-default fs loses to a small clean interpreter.

## What would change my mind
Proof that pooled instantiation with a real module cache still misses the 50 ms debounce budget — or a vendored, capability-scoped Lua/QuickJS that demonstrably deletes most of `crates/specforge-wasm` while preserving R-2/R-4/R-6.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 3
**One-line rationale:** Wasm owns the watch-loop determinism and static-binary story; fix the four open C7 gaps rather than adopt a second cross-platform quirk matrix.
