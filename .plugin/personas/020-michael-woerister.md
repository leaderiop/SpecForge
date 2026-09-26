# Position — Michael Woerister (rustc incremental compilation designer)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. **A plugin call is a query; wasm gives a fingerprintable, complete input set.** Red-green reuse is only sound when every input is explicit. A wasm call's inputs are (blob hash, protocol version, graph snapshot/delta) — the registry already pins blobs by SHA-256 (evidence §1.4), and blobs are platform-independent, so one fingerprint covers macOS arm64 and Linux x64. An embedded interpreter adds implicit inputs — module caches, globals, interpreter build/version — that no fingerprint captures, making "unchanged, reuse" unsound by construction.

2. **`crates/specforge-watch/src/dispatch.rs` puts validators in the hot path** (`plan_incremental_dispatch` sends `ValidatorInput::Delta` or `FullGraph` per extension). R-6 determinism requires re-running an unchanged node to reproduce its output. Wasm calls start from zeroed linear memory — inherently reproducible. Interpreters accumulate state across reloads; C7-09 (`query_scope` ignored) already shows how one forgotten input breaks invalidation soundness today.

3. **R-5 hot reload maps cleanly to blob swap:** new fingerprint, downstream re-run, atomic. C7-04/C7-10 are config holes (allow-by-default fs, unenforced `max_execution_ms`), fixable inside the model — not evidence the model is wrong. Python fails R-3 outright; MULTI means two fingerprint schemes and two dispatch protocols in `dispatch.rs`.

## Biggest risk in my verdict

Instantiation cost per dispatch in the watch/LSP loop, since C7-02 (AOT cache is a byte-copy) and C7-08 (EnginePool is a ledger) mean no real pooling today. Wasmtime supports genuine AOT (`.cwasm`) and instance pools — the fix is bounded work, but until it lands the sound runtime feels slow.

## What would change my mind

Measured evidence that, after real pooling/AOT, per-dispatch wasm latency still misses watch budgets — while a fresh-interpreter-per-reload Lua model (no shared state, instruction-count limits) matched wasm determinism and sandboxing at materially lower cost with trivial migration of the four builtins.
