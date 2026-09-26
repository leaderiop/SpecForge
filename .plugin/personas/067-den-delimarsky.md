# Position — Den Delimarsky (Spec Kit creator; SDD & MCP context engineering)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Requirements eliminate alternatives before taste enters. R-2+R-3+R-4 jointly admit one candidate: wasm alone offers memory isolation, zero system deps, and byte-addressable sha256-verified registry artifacts (evidence.md §1.4). Python embedding is ambient-capability by design; Lua/TS sandboxes are interpreter policy — weaker guarantees than memory isolation for R-2.
2. The workload is mostly declarative — Spec Kit's own lesson: structure carries the value; the executable delta is small and hot. evidence.md measures guests at ~97% generated manifest, ~3% logic (`@specforge/formal`, 478 lines). Optimize artifact verifiability, not authoring comfort for the 3%.
3. Agents need a typed contract, not a friendlier language. The agent-facing gap is C7-03's stringly JSON over `call_export` — no IDL. Schema-backed describe payloads fix agent ergonomics inside KEEP_WASM; switching to TypeScript wouldn't. The PRD-007 `specforge_validate` loop consumes the graph, not the guest language.
4. Only KEEP_WASM converges C7-11's three implementations: delete `crates/specforge-emitter/src/builtins/` mirrors and native `NativeCustomRules` dispatch (evidence §1.2), routing everything through guests under R-1's one security model.

## Biggest risk in my verdict
Rust-toolchain + wasm32 friction suppresses third-party authorship; the registry bet dies from cold-start ergonomics, not runtime capability.

## What would change my mind
Evidence that agent-authored SDK publishing fails in practice (build friction → abandoned contributions), or formal-pass overhead that survives real engine pooling (C7-08) yet still can't meet analyze latency.
