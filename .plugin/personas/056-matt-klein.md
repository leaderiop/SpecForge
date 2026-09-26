# Position — Matt Klein (Envoy creator; extension-surface governance)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The protocol is the product. `docs/extension-protocol.md` defines a reviewed, versioned surface — 11 describe categories gated by `contribution_flags` (`crates/specforge-wasm/src/protocol/mod.rs` re-exports `SUPPORTED_CATEGORIES`) — Envoy's exact pattern: fixed core, enumerated extension points, runtime negotiation. Switching runtimes discards that governed surface while every audit failure (C7-03 no IDL, C7-04 allow-by-default fs, C7-08 ledger engine pool, C7-10 unenforced `max_execution_ms`) is implementation discipline that recurs under Lua or V8 too.
2. R-2/R-3/R-6 favor wasm uniquely: memory isolation with capability-scoped imports, static single-binary distribution (Python/PyO3 fails R-3 outright; V8 is heavy), and deterministic, snapshot-testable execution.
3. R-1 is violated by wasm-path bypasses, not by wasm: builtin custom rules run natively (`NativeCustomRules`, `crates/specforge-emitter/src/compile.rs`) and three parallel implementations persist (C7-11) while guests are ~97% manifest, ~3% logic. KEEP_WASM plus convergence — delete the mirrors, route `validate__*` through guests — is the honest fix; MULTI institutionalizes the duplication.

## Biggest risk in my verdict
Endorsing KEEP_WASM without enforcement normalizes a false sandbox claim: C7-04 means deny-by-default is currently allow-by-default, so the runtime's core safety property is unimplemented. Envoy taught me an extension bet is a governance commitment, not a dependency choice.

## What would change my mind
Measured evidence that Rust→wasm32 compile latency breaks AI-agent authoring loops, or a scripting tier proving deterministic, R-2-grade sandboxing — at most a sandboxed secondary tier (Envoy's Lua filter pattern), never a replacement of the reviewed surface.
