# Position — Daniel Jackson (formal methods; Alloy; "impossible by construction")

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The payload is ~97% declarative: evidence.md counts 629 guest lines total, ~3% real logic; the only logic-bearing guest (@specforge/formal) carries 478 pass lines. The concept boundary lives in the nine `describe_*.json` manifest categories, already language-neutral. A new runtime buys ergonomics for the 3% while destabilizing the 97%.
2. Impossible by construction: wasm memory isolation plus capability imports is the only candidate making R-2's no-ambient-access structural. A `.py` plugin writing `import os` succeeds by default; wasm eliminates the class, scripting merely forbids it by convention. Same for R-6: wasm32 execution is deterministic by construction; CPython/V8/LuaJIT offer determinism by discipline.
3. The open findings — C7-02 byte-copy AOT, C7-08 ledger-only EnginePool in `crates/specforge-wasm`, C7-10 unenforced `max_execution_ms`, C7-03 stringly-typed `call_export` — are host-enforcement failures, not wasm-model failures. Swapping engines fixes none; enforced fuel limits, real pooling, and a typed IDL fix all. Small scope: check the few invariants that generate whole error classes.

## Biggest risk in my verdict

Authoring demands a Rust toolchain plus wasm32 target. If that friction starves the registry bet, KEEP_WASM wins the sandbox and loses the ecosystem; AI-agent authors may not tolerate the compile loop.

## What would change my mind

Proof that R-5 hot reload or agent authoring is unusable on wasm, or a hard need for ecosystem libraries reachable only from CPython/TS. Then: one scripting tier behind the same typed protocol — never two first-class mechanisms.
