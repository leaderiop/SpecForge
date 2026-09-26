# Position — David Soria Parra (MCP co-creator; agent-tool protocol design)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The audit's open gaps are protocol defects, not wasm defects — and only wasm makes R-2 structural. C7-03 (no IDL), C7-04 (fs allow-by-default), C7-08 (EnginePool ledger), C7-10 (`max_execution_ms` never enforced) live host-side; a runtime swap re-incurs all of them. `crates/specforge-mcp/src/protocol/router.rs` is the fix pattern MCP proved: distinct typed primitives with negotiated capabilities. The plugin wire needs it too — schema'd calls, capability negotiation, enforced budgets — inside Extism's capability-import model, where a guest holds zero ambient authority by construction.
2. R-3/R-4 favor artifacts over interpreters: wasm blobs are static, reproducible, sha256-pinned registry objects inside one binary (evidence §1.4). Python fails R-3 outright (system interpreter, evidence §4); QuickJS/V8 embed ambient-reach surface needing bespoke, auditable permission layers.
3. The authoring argument is misdiagnosed. Guests are ~97% generated manifest, ~3% logic (evidence §2) — the friction is the Rust-toolchain SDK, not the runtime. Wasm is the one target where AI-agent authors can write TypeScript via component tooling behind the unchanged SDK protocol: a toolchain fix under KEEP_WASM, no second runtime.
## Biggest risk in my verdict
Cold-start overhead with C7-02/C7-08 unresolved makes analyze latency fragile, and if logic-bearing AI-authored plugins become the product, the compile-to-wasm loop could strangle the registry bet.

## What would change my mind
Measured evidence that AI-agent authoring loops cannot tolerate wasm compile round trips, or that mlua/deno_core permission models satisfy R-2/R-6 with less host code than fixing the wasm protocol.
