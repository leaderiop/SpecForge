# Position — David Tolnay (serde lead; Rust library ergonomics)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The defect is the wire contract, not the runtime. C7-03's missing IDL should be fixed with typed protocol types — serde already is SpecForge's de facto IDL: `GraphProtocolSchema` (crates/specforge-emitter/src/schema.rs:146) is derive-backed and content-hashed via `serde_json::to_string` + SHA-256 (schema.rs:718). A Lua/TS boundary trades typed structs and the guests' `anyhow` error path (extensions/*/Cargo.toml) for dynamic tables and nil-returns — strictly worse ergonomics.
2. R-6 determinism favors wasm: same blob, same output, snapshot-stable across macOS arm64/Linux x64; LuaJIT/CPython/V8 make engine versions part of the snapshot contract. The existing `extension_json_sync`/`builtin_blob_sync` guard-test pattern extends naturally to a real IDL.
3. R-3 kills PYTHON (system interpreter or fragile bundling) and favors wasmtime's static linkage. R-1's convergence (C7-11) means deleting the `crates/specforge-emitter/src/builtins/` mirrors and routing builtins through the wasm path — one mechanism, not a second runtime beside the first. Guests are ~97% generated manifest (evidence.md), so any swap's migration win is small.

## Biggest risk in my verdict
KEEP_WASM as-is ships unenforced claims: C7-04 fs allow-by-default, C7-10 `max_execution_ms` ignored, C7-08 engine pool is a ledger, C7-02 fake AOT. Open, "sandboxable" is marketing; wasmtime's 485→511 dep weight is real. Rust-guest compile latency may also deter AI-agent authors.

## What would change my mind
Evidence of author demand typed Rust can't serve, plus an embedder (mlua or deno_core) proven to match byte-deterministic snapshots, capability-scoped sandboxing, and static single-binary distribution at lower maintenance than fixing C7-02/04/08/10.
