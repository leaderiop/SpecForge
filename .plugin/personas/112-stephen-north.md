# Position — Stephen North (Graphviz project lead; graph model/view architecture)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. GN99's lesson maps directly: Graphviz succeeded by separating the cgraph data model from pluggable layout/render engines behind ONE versioned ABI. SpecForge already has that shape — `crates/specforge-graph` is the model, `crates/specforge-emitter/src/dot.rs` and `outline/dot.rs` are views that stringify it, plugins compute over the graph snapshot. The boundary is a contract, not a language; KEEP_WASM preserves one under R-1.
2. The repo's regression culture is byte-compare guards (`extension_json_sync`, `builtin_blob_sync`) — why dot output is snapshot-tested. Wasm blobs are byte-verifiable, interpreter-free artifacts (R-4/R-6). Scripting drags interpreter+stdlib versioning into reproducibility; CPython's pip ecosystem is ambient capability (evidence.md §4) — nondeterminism plus sandbox leak.
3. C7-11's three parallel implementations are the failure mode to kill, not extend: make the wasm guest the single source of truth and delete the native mirrors in `crates/specforge-emitter/src/builtins/`. MULTI re-creates that duplication at the language layer. The genuinely broken part — C7-03's stringly JSON over `call_export`, no IDL — is a contract fix (cgraph's typed attribute model, not a new engine).

## Biggest risk in my verdict
Wasmtime weight and cold start: C7-08's EnginePool ledger and unenforced `max_execution_ms` (C7-10) mean watch hot reload (R-5) pays per-call compile until AOT pooling lands.

## What would change my mind
Evidence that plugins outgrow today's ~3% real logic into graph-slicing the wasm crossing can't express cheaply (C7-09's ignored `query_scope`), AND one runtime demonstrating default-deny capabilities plus byte-reproducible signed distribution — replacement, never a second tier.
