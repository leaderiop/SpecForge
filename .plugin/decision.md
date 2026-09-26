# Decision — Plugin Runtime for SpecForge

**Generated:** 2026-09-27 · rev 789d214 · 125 personas + 12 dimensions

## Verdict

**KEEP_WASM** — stay with Extism/Wasmtime; close the audit gaps inside the model.

**Tally:** 110/125 personas (88%) · 8/12 dimensions · all 4 security-critical dimensions (sandbox, determinism, performance, host API) converge on WASM.

## Why

1. **Sandbox by construction, not by convention** — wasm's imports-are-capabilities model meets R-2 structurally; embedded interpreters meet it only by subtracting stdlib from a shared address space (D05).
2. **Determinism (R-6)** — fuel + import allowlist makes wasm output a pure function of module bytes + input + fuel. Python hash randomization, Lua/JS GC, and JIT make interpreter verdicts non-reproducible (D09).
3. **Zero migration cost** — the 4 builtins carry ~212 LOC of pass logic; their wasm payloads are ~97% generated manifest. Rewriting them for a scripting runtime spends engineering for zero capability gain (D10).
4. **AI-agent safety** — hallucinated wasm capabilities are structurally inert; hallucinated `os.execute` in Lua/Python is a real execution. The typed SDK build catches hallucinated APIs at compile time (D07).

## The real dissents (not dismissed)

- **C7-11 consolidation** (D12, LUA conf 4): a single `.lua` artifact equals the authored, executed, distributed, and signed form — deleting the mirrors, committed JSON, blobs, guest crates, SDK macros, xtask extractor, and both sync tests. The counter: fails R-2 (shared address space) and R-6 (no fuel).
- **Hot-reload DX** (D11, LUA conf 3): script re-read vs wasm re-instantiate + authoring rebuild. The counter: R-6 determinism and sandboxing still favor wasm.
- **Host API** (D08, KEEP_WASM conf 4): the fix is WIT/component-model, not a language swap.

## Conditions for revisit

- A mature Lua-to-wasm/componentize bridge emerges (would address D12's consolidation inside the wasm sandbox).
- A WIT IDL ships with typed graph-access interfaces (closes C7-03 structurally).
- A genuinely sandboxed CPython distribution model appears (unlikely — GIL + C extensions).

## Execution plan (KEEP_WASM)

1. Fix C7-04: sandbox fs deny-by-default (currently allow)
2. Fix C7-10: wire max_execution_ms via wasmtime fuel/epoch
3. Fix C7-02: real AOT via `PluginBuilder::compile()` + `with_cache_config`
4. Fix C7-08: real warm-engine pool (reuse instantiated plugins)
5. Fix C7-03: WIT IDL for the host API (component model)
6. Fix C7-09: honor query_scope in make_query_graph_fn
7. Delete the native mirrors once the wasm path is the only path (C7-11 closure)

Each item is a bounded slice with tests; none changes the plugin model.
