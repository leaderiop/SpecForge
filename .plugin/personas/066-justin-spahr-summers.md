# Position — Justin Spahr-Summers (MCP co-designer; API/protocol design)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The runtime is a protocol boundary, not a language choice — MCP's founding bet. The audit indicts the contract, not the engine: C7-03's stringly-typed `call_export`, whose entire error surface is `WasmCallResult::{Ok, Trap}` (`crates/specforge-wasm/src/runtime.rs:5-8`) — no structured diagnostics, no drift detection. A typed IDL fixes the boundary; swapping engines resets it for every candidate.
2. Honest capability enforcement is structural only in wasm: claims machine-checked, not conventional. Flip `file_system_access` to deny-by-default (`crates/specforge-wasm/src/sandbox.rs:22`); wasmtime fuel/epoch interruption makes `max_execution_ms: 30_000` (`sandbox.rs:8`) enforced, not decorative. Lua/quickjs sandboxing is "we didn't bind fs" — a drifting convention, equally exposed for every plugin under R-1.
3. Migration buys little: evidence.md shows guests are ~97% generated manifest, ~3% logic (629 lines total; only `@specforge/formal` bears logic, 478 lines). Rewriting that in Lua/TS trades signed-blob reproducibility (R-4) and the shipped SDK for ergonomics on a 3% slice.

## Biggest risk in my verdict
KEEP_WASM re-ratifies vaporware (C7-02, C7-08, C7-10) unless the honesty fixes land. Deeper: authoring requires a Rust + `wasm32` toolchain — real friction for AI-agent authors, this product's primary plugin writers, as logic grows past 3%.

## What would change my mind
Evidence that third-party plugin logic dominates manifests and the toolchain measurably blocks AI co-authors; or a Luau-style embedding proving CPU budgets and determinism as rigorously as wasmtime fuel, with drop-a-file authoring.
