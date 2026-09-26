# Position — Sven Efftinge (DSL & language tooling design)

**Verdict:** TYPESCRIPT
**Confidence:** 3
## Arguments
1. The workload itself demotes the runtime: evidence.md §2 measures the four builtin guests at ~97% generated manifest, ~3% logic. The `describe_*` manifest categories are a declarative DSL wearing JSON — the real product surface; what executes is thin per-entity validation plus four compiler passes. So choose the runtime for the authoring loop, not the 3%. Rust+wasm32 (`crates/specforge-extension-sdk`'s requirement) is the slowest loop an AI agent can be handed — cargo + target install per edit — while an embedded TS engine makes edit→analyze re-runs (R-5) instantaneous.
2. Tooling is my home turf, and SpecForge already owns the machinery: `integrations/vscode/src/lsp-client.ts`, `schemas/specforge.schema.json`, and a real LSP (`crates/specforge-lsp/src/hover.rs`, `completion.rs`, `code_actions.rs`). A TS plugin API with a generated `.d.ts` gives plugin authors diagnostics and hovers from that same infrastructure — the Langium thesis applied to plugins. Lua and Python offer no static type layer to serve; wasm authors get plain Rust tooling with zero SpecForge-specific IDE services.
3. The audit's open wounds C7-03 (no IDL, stringly `call_export`) and C7-11 (three parallel implementations) are language-design failures. One typed IDL generating both manifest validation and plugin-API types fixes both, and TS consumes that IDL natively end-to-end — no other candidate does.
## Biggest risk in my verdict
R-6 determinism and R-2 sandboxing in JS engines are conventions (frozen `Date`/`Math.random`, op allowlists), not wasmtime's memory isolation. And rewriting `@specforge/formal`'s ~478 lines of Rust passes, plus per-entity checks over 1.7k-entity graphs, risks real performance regression.
## What would change my mind
Benchmarks showing order-of-magnitude JS overhead on the 1.7k-entity graph workload (criterion 3); or a committed plan fixing C7-02/C7-08/C7-10 inside KEEP_WASM. MULTI becomes acceptable only if the C7-03 IDL fix lands first.
