# Position — Joshua Barretto (ariadne/chumsky creator; diagnostics rendering)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Determinism is load-bearing, and wasm buys it structurally.** Guests can only observe what the host imports; with no clock/env imports, `analyze` output is reproducible by construction. That's what makes R-6 snapshot tests honest — the same discipline I built ariadne around: `crates/specforge-validator/src/render.rs` renders ordered, source-anchored Reports whose bytes must be stable. Scripting runtimes reintroduce interpreter-version drift that R-4 reproducibility then has to fight.
2. **The audit gaps are bugs in the model, not failures of it.** C7-04 (fs allow-by-default), C7-02 (byte-copy AOT), C7-08 (EnginePool ledger), C7-10 (unenforced `max_execution_ms`) all have in-model fixes — capability imports defaulting to deny, real pooling, wasmtime fuel. Switching runtimes to escape them discards ~9.4k LOC of working, tested infrastructure for a rewrite.
3. **The actual guest workload is tiny and mostly declarative.** Evidence: 629 lines of lib.rs across four guests, ~97% generated manifest, only `@specforge/formal` (~478 lines) carries logic — written, compiled, vendored (324–415 KB blobs in `extensions/*/wasm/`), and sha256-pinned through the registry. A scripting tier buys ergonomics for a codebase that doesn't exist yet while breaking `builtin_blob_sync`/`extension_json_sync` guarantees.

## Biggest risk in my verdict
Wasm's Rust-toolchain requirement is real friction for AI-agent authoring; if third-party authorship never materializes because compiling to `wasm32-unknown-unknown` is too heavy, the registry bet dies.

## What would change my mind
Evidence that non-Rust authors actually ship plugins — or that determinism/sandbox parity (R-2/R-6) is achievable in mlua's capability model without forking the runtime.
