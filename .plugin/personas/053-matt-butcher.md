# Position — Matt Butcher (Fermyon Spin creator; warm-start ergonomics)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The wasm "cold start" tax is an unfinished implementation, not the model. `crates/specforge-wasm/src/engine_pool.rs` books `WarmInstance { extension_name, memory_mb }` entries in a `VecDeque` — `warm()`/`evict_lru()` shuffle ledger rows while no wasmtime engine is retained (audit C7-08). Yet `WarmEngineConfig` (16 instances, 512 MB cap, LRU eviction, `Arc`-shared for MCP handlers) is exactly the right shape; I built Spin on precisely this move: pool real engines, serialize real AOT (C7-02's cache is a byte-copy), enforce `max_execution_ms` via fuel/epoch traps (C7-10). Weeks inside wasm; switching restarts warm-scheduling from zero.
2. Distribution already matches my Helm discipline: vendored 324–415 KB blobs, sha256-pinned and signed through the registry, lock-file-mediated (`crates/specforge-wasm/src/lock_file.rs`; evidence §1.4) — reproducible R-4 essentially for free. Source-distributed scripts can't offer artifact-grade verifiability.
3. R-1's scandal is C7-11's three parallel implementations, not wasm itself. Converge: delete the `crates/specforge-emitter/src/builtins/` mirrors, route `NativeCustomRules` through guest `validate__*` exports, and give stringly `call_export` the IDL it lacks (C7-03) — one mechanism, builtins as plain plugins.

## Biggest risk in my verdict
Ergonomics: ~97% of guest payload is generated manifest and the real authors are AI agents; if SDK macros (`docs/extension-sdk.md`) stay Rust-toolchain-heavy while per-call cost stays unfixed, a TypeScript tier out-competes on authoring.

## What would change my mind
A benchmark showing pooled+AOT wasm cannot carry watch-mode (R-5) per-call latency, or a runtime matching wasm's memory isolation, static single-binary hosting, and signed-blob reproducibility at materially better agent-authoring ergonomics.
