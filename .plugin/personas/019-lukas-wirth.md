# Position — Lukas Wirth (rust-analyzer lead; incremental compilation & LSP)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Fresh instantiation per reload gives determinism by construction (R-6). `crates/specforge-watch/src/pipeline.rs` already propagates edit deltas over immutable snapshots; wasm reload is the same shape — swap blob, instantiate clean. A persistent interpreter VM (mlua/quickjs) reintroduces the stale-global-state problem salsa's durability work exists to solve: runs inherit ambient mutable state, snapshot tests go order-dependent.
2. The perf case against wasm is two open fixes: C7-02 (AOT cache is a byte-copy, `_aot_cache_path` ignored) and C7-08 (EnginePool is a ledger, no warm instances). The salsa lesson: cache the expensive compiled artifact, reuse the cheap instantiation — inside the existing ~9.4k-LOC host, not a runtime rewrite. wasmtime fuel/epoch makes C7-10's unused `max_execution_ms` enforceable, completing R-6.
3. R-4 reproducibility favors sealed artifacts: wasm blobs are sha256-verified registry artifacts (evidence §1.4); a Python plugin's "reproducible" run depends on an interpreter-version matrix — breaking R-3 single-binary and R-6 snapshots simultaneously.

## Biggest risk in my verdict
Authoring ergonomics. Rust + `wasm32-unknown-unknown` is a high wall for the third parties the registry bets on; an empty registry makes KEEP_WASM technically superior yet practically dead. The evidence's own escape hatch: guests are ~97% generated manifest, ~3% logic — manifests belong in data files, shrinking what any runtime carries.

## What would change my mind
(a) Watch/LSP latency benchmarks showing per-call wasm cost can't meet budget even with a real module cache plus warm pool; (b) demonstrated third-party authoring stalled specifically on the toolchain; (c) plugin logic needing ecosystem access (HTTP, rich JSON tooling) that capability-gated wasm handles poorly.
