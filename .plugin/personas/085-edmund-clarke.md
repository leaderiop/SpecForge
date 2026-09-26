# Position — Edmund M. Clarke (formal methods, model checking)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. Wasm alone has an official formal operational semantics — exactness model checking demands. R-6 (deterministic, snapshot-testable analyze) and my roadmap (W063 property kinds, safety/liveness over event graphs beyond petgraph's reachability/cycles) make semantic rigor load-bearing: Lua 5.4 vs LuaJIT diverge; Python/V8 vary by version and flags.
2. The only real guest logic is already the analysis core: `@specforge/formal`'s 478 lines (evidence §2 — guests ~97% manifest, ~3% logic) are the four compiler passes. Future passes are fixpoints over graph snapshots needing near-native speed and enforced bounds; wasmtime fuel/epoch is the principled fix for C7-10's unenforced `max_execution_ms`; mlua offers debug hooks, PyO3 nothing.
3. R-2: memory isolation + capability imports is a browser-grade sandbox; mlua sandboxing is interpreter convention; PyO3's pip ecosystem is ambient-capability — formal security review rejects it.
4. R-1 cuts toward one mechanism: converging the native mirrors (`crates/specforge-emitter/src/builtins/`) into the wasm path collapses C7-11 instead of adding a second tier.

## Biggest risk in my verdict

Wasmtime's weight and cold path: C7-08 (EnginePool is a ledger) and C7-02 (AOT cache is a byte-copy) leave R-5 hot reload and per-call overhead on unbuilt machinery; KEEP_WASM survives only if pooling/AOT ships.

## What would change my mind

If analyze passes move host-side (petgraph is native) leaving plugins declarative manifests, a small LUA tier suffices; or a typed IDL fixing C7-03's stringly `call_export` boundary in a JS engine could win ergonomics without losing determinism.
