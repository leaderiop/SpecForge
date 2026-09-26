# Position — Mads Hartmann (LSP/tooling engineer; bash-language-server creator)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The workload is host-shaped. evidence.md §2: the guest payload is ~97% generated manifest, ~3% real logic — and `crates/specforge-lsp/src/backend.rs` loads extensions only to seed the registries behind completions/hovers (`known_extension_keywords`, protocol-host manifests); `crates/tree-sitter-specforge` does the syntax work. Scripting runtimes optimize authoring, i.e. the 3%. That 3% (`@specforge/formal`'s four passes, ~478 lines) is exactly where Rust and the existing `#[compiler_pass]` macro pay off; porting it buys no sandbox strength.

2. R-2 is decisive. Wasmtime gives memory isolation plus capability imports; C7-04's allow-by-default `file_system_access` is a config bug, not an architectural limit. `mlua`/quickjs sandbox by interpreter convention — one escape yields direct host memory. Years of forwarding shellcheck taught me: trust memory/process boundaries over "the interpreter is usually safe."

3. R-3/R-4/R-6 all favor byte blobs: wasmtime static-links (PyO3 fails single-binary outright), plugins are deterministic and sha256-pinnable in the signed registry. The wasm path's failures — C7-02 byte-copy AOT, C7-08 ledger-only engine pool, C7-10 unenforced `max_execution_ms` — are implementation debt inside the model, fixable without a migration.

## Biggest risk in my verdict

Cold-compile per instantiation while C7-02/C7-08 stay unfixed makes `watch`/LSP hot-reload loops sluggish; latency pain gets misread as "the runtime is wrong" and triggers the migration anyway.

## What would change my mind

A benchmark showing pooled/AOT instantiation cannot bring watch re-analysis under ~100ms, or credible third-party AI-authored plugins demanding an iterative TypeScript loop — then MULTI with a `deno_core` tier behind the same protocol.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The workload is 97% host-side metadata; wasmtime is the only candidate meeting R-2/R-3/R-4 simultaneously, and the audit findings are fixable debt, not model failure.
