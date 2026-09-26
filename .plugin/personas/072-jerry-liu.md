# Position — Jerry Liu (LlamaIndex co-founder; retrieval & context budgets)
**Verdict:** KEEP_WASM
**Confidence:** 3
## Arguments
1. **Pay only for real work.** evidence.md: the guest payload is ~97% generated manifest, ~3% logic (629 lines of lib.rs; only `@specforge/formal`'s 478 bear logic). A second interpreter (V8, CPython) buys nothing there while multiplying host weight; wasm is sunk cost with the strongest isolation (R-2) and static linking (R-3).
2. **Determinism is retrieval fidelity (R-6).** Fixed-blob wasm replays identically — same spec, same diagnostics — keeping snapshot tests and MCP re-queries (`query --depth N`, `crates/specforge-mcp/tools/query.rs`, RES-18) trustworthy context. Scripting tiers drag ambient state (pip, env, GIL) into the graph agents consume.
3. **The disease is divergence, not wasm.** C7-11 (three parallel implementations) and C7-03 (no IDL, drift already happened) demand convergence to one mechanism: delete the `crates/specforge-emitter/src/builtins/*.rs` mirrors, enforce fuel-based CPU limits (C7-10), flip sandbox fs to deny-by-default (C7-04). MULTI doubles the drift surface the audit already caught.
## Biggest risk in my verdict
Authoring ergonomics: Rust + `wasm32-unknown-unknown` is the worst iteration loop for AI agents — the primary authors — and the registry's third-party bet fails if no agent fights a cross-compile toolchain for a 30-line validator. C7-02's byte-copy AOT cache keeps compile costs real.
## What would change my mind
Telemetry showing many third-party small logic-only plugins: a Lua tier (`mlua`, vendored C, deny-by-default, instant hot reload for R-5) behind the same protocol wins on latency; or `deno_core` if its permission model ships statically without breaking R-3 binary bounds.
