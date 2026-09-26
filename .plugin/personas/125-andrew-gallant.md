# Position — Andrew Gallant (regex/automata engineer; direct-dependency maintainer)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. Determinism is a correctness guarantee, not a perf nicety — the same reason the regex crate compiles user patterns into linear-time automata instead of backtracking: bounded worst case for untrusted input. Wasm32-unknown-unknown guests are GC-free, single-threaded, linear-memory; wasmtime's fuel/epoch metering makes C7-10's unenforced `max_execution_ms` real, deterministic. LuaJIT GC pauses and V8 JIT warmup variance are poison for R-6 snapshot-testable output.
2. The logic surface is tiny. evidence.md: guests are ~97% generated manifest, ~3% logic (formal's 478 lines is the only real code). That declarative 97% is static JSON under any runtime, and `matches` rules already execute host-side through the regex crate in `crates/specforge-registry/src/compilation/validation_engine.rs` — compiled once at parse time, linear-time per entity. Swapping runtimes optimizes authoring for workloads that don't exist; fixing C7-03's IDL buys more per LOC spent.
3. R-2/R-3: wasmtime gives memory isolation and static linking — zero system packages. C7-04 is a config default bug, not an architecture flaw; PyO3's system-Python requirement fails R-3 outright.

## Biggest risk in my verdict
Per-call overhead: C7-08's EnginePool is a ledger, so guests may recompile per call. If pooled instantiation still can't carry per-entity validation over 1.7k-entity graphs interactively, DX pressure forces MULTI drift anyway.

## What would change my mind
Measurements (ripgrep discipline: benchmark before believing) showing pooled wasm can't hit interactive analyze latency — or that AI-agent authors reliably can't produce Rust wasm artifacts. Then one protocol, one capability-scoped scripting tier.
