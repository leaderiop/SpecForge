# Position — Adam Dymitruk (Event Modeling & ubiquitous language)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The plugin contract is a ubiquitous-language problem, not a runtime problem. C7-03 (stringly JSON over `call_export`, no IDL) and C7-11 (three parallel dialects: native mirrors in `crates/specforge-emitter/src/builtins/`, embedded blobs, SDK crates) already caused drift. Evidence §2: guests are ~97% generated manifest, ~3% logic — the nine `describe_*.json` categories ARE the language journeys and events specs compile against. Fix the IDL; guest language is an implementation detail.
2. Every Event Modeling slice carries its own acceptance criteria; SpecForge attaches `verify` blocks to behaviors. R-6 snapshot-testability demands bit-identical analyze output per input — sealed wasm plus vendored, signed blobs (`extensions/*/wasm/`, sha256 + registry signing, evidence §1.4) deliver it. pip/npm ecosystems are ambient capabilities: unmodeled actors crossing swim-lanes, breaking R-2's capability scoping by default.
3. The real grammar workload — `event_graph_analyze` in @specforge/formal over ~1.7k-entity graphs — is deterministic command→event→read-model traversal. Its actual gaps are C7-08 (EnginePool is a ledger) and C7-10 (max_execution_ms unenforced), both fixable inside wasm. MULTI recreates C7-11 at protocol level.

## Biggest risk in my verdict
Rust-toolchain-only authoring (evidence §1.5) may choke third-party adoption — the registry's whole bet — leaving the builtins as the only plugins forever.

## What would change my mind
Third-party demand provably blocked by the wasm32/Rust toolchain, plus a capability-scoped, instruction-limited Lua meeting R-2/R-6, would favor MULTI behind one IDL: wasm as portable artifact, scripting as authoring surface compiling to the same contract.
