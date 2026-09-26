# Position — Dirk Bäumer (Microsoft LSP spec lead)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **R-6 determinism is a diagnostics contract.** `crates/specforge-lsp/src/backend.rs` publishes per-URI diagnostics; its line-274 comment promises "LSP and CLI agree byte for byte." Plugin passes (`@specforge/formal`'s four `__pass_*` analyses, ~478 lines of Rust) must stay pure functions of the graph snapshot. Wasm gives deterministic, snapshot-testable execution; CPython imports ambient packages with network access — breaking R-6 and R-2 simultaneously.
2. **Distribution already fits the LSIF pattern.** Evidence.md: blobs are vendored, sha256-verified, key-pinned at install — precomputed, verifiable artifacts. Python embedding fails R-3 outright (system interpreter); deno_core/V8 bloats the single binary we ship to macOS arm64 + Linux x64.
3. **My cluster's pain lives inside this model.** C7-08 (EnginePool is a ledger — per-call instantiate on every watch re-analysis), C7-10 (unenforced `max_execution_ms` — one hung pass stalls publishDiagnostics), C14-03 (blocking LSP work). These are wasmtime-engineering fixes (pooling, epoch interruption), not runtime-swap reasons — mlua states carry mutated globals across reloads and lack memory isolation.
4. **R-1 forces convergence.** Merging C7-11's three implementations and routing `NativeCustomRules` (E004/E006/E010/W010) through guest exports is required work; switching languages also ports formal's real logic with no semantic safety net.
## Biggest risk in my verdict
Wasmtime weight plus slow watch-mode instantiation erodes confidence; pressure to restore a "fast native path for builtins" would re-break R-1.

## What would change my mind
Proof that QuickJS/deno_core meets R-3 size, R-6 determinism, and enforced timeouts while materially improving AI-agent authoring ergonomics — or that wasmtime instantiation cannot be amortized below LSP-acceptable didChange latency.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Deterministic, verifiable, sandboxable diagnostics at LSP latency is what the wasm model already delivers; the audit findings are fixable engineering inside it, while Lua/Python/TS trade away determinism, single-binary distribution, or migration safety.
