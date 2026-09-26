# Position — Carl Lerche (Tokio creator; async runtime architecture)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The failures are budgeting failures, not runtime failures. C7-08 (EnginePool is a ledger, no warm instances) and C7-10 (`max_execution_ms` never enforced) are task-budgeting problems. Wasmtime ships pooling allocators, module serialization for true AOT, and epoch/fuel interruption; it is the only candidate with real preemption primitives, which R-6 determinism under a runaway plugin requires. mlua debug hooks are fragile per-line traps; CPython cannot preempt across the GIL; V8 needs platform work.
2. The host surfaces already tolerate sync CPU work correctly: `crates/specforge-lsp/src/backend.rs:66` routes blocking walkdir/parse through `tokio::task::spawn_blocking`; `crates/specforge-watch/src/debounce.rs` batches via `mpsc` before a pipeline run. Plugin calls slot into that shape — warm instance, `spawn_blocking`, fuel cap; switching runtimes fixes nothing the async design doesn't dictate.
3. The workload barely exercises a runtime: evidence.md shows guests are ~97% manifest / ~3% logic (formal's 478 lines is the only logic-bearing guest). The real authoring pain is no IDL (C7-03) and three parallel implementations (C7-11) — protocol fixes independent of runtime. Lua/MULTI would resurface the duplication R-1 exists to kill. Blobs are flat files: R-5 hot reload is a byte-reload; R-4's signed registry already distributes that artifact.

## Biggest risk in my verdict
The audit gaps (C7-02, C7-08, C7-10) never get implemented: per-call compilation with no enforcement — wasmtime's weight paid, its guarantees unused, warm-path latency dominating watch re-runs.

## What would change my mind
Measurements showing per-call wasm overhead dominating analyze/watch latency even with pooling and warm engines. Then mlua with capability-scoped imports and instruction-count budgeting becomes defensible — provided the IDL fix (C7-03) lands first, keeping guests portable across runtimes.
