# Decision — Plugin Runtime for SpecForge

**Generated:** 2026-09-27 · rev 4c9e9f2 · 125 personas + 12 dimensions + 25 research studies

## Verdict

**KEEP_WASM** — stay with Extism/Wasmtime; close the audit gaps inside the model.

**Tally:** 110/125 personas (88%) · 8/12 dimensions · weighted confidence 15.2 vs 10.6 (LUA) vs 9.0 (TS)

## Why

1. **Sandbox by construction, not by convention** (D05, conf 4). WASM imports are capabilities — a plugin cannot touch the filesystem, network, or environment unless the host explicitly provides that import. Memory isolation is enforced by the engine. Lua/Python/JS sandbox by convention: you delete `io`, `os` from the environment and hope you didn't miss an escape path. Redis Lua has 3 RCE-class CVEs in 4 years (CVE-2022-0543 CVSS 10.0; CVE-2025-49844 CVSS 9.9). Figma replaced its same-VM Realms sandbox with QuickJS-compiled-to-WASM after multiple escapes. Wasmtime provides memory isolation + fuel/epoch preemption with no atomicity-vs-liveness dilemma.
2. **Determinism is structural** (D09, conf 4). WASM 3.0 defines a deterministic profile (DET). Wasmtime fuel makes output = f(module bytes, input, fuel). Lua 5.4 seeds string hashes from time+ASLR; LuaJIT uses PRNG; both re-shuffle `pairs()` per run. Python has no mechanical determinism story. Shopify Functions, Soroban, and CosmWasm all chose capability-restricted WASM for exactly this reason — Soroban guarantees replay-identical results across host upgrades.
3. **AI-agent safety: hallucinated capabilities are inert** (D07, conf 4). A hallucinated WASM import is structurally inert — the guest can't call a function the host didn't link. A hallucinated `os.execute("rm -rf /")` in Lua is real. Rust-to-wasm via typed SDK macros catches hallucinated APIs at compile time. AI codegen benchmarks confirm: Rust ranks #1 on SWE-bench Multilingual (58.1%) because compile-check-repair converts weak generation into caught errors; Lua ranks last (zero SWE-bench presence, smallest training corpus).
4. **Zero migration cost** (D10, conf 3). The 4 builtins carry ~212 LOC of pass logic; their wasm payloads are ~97% generated manifest. Rewriting them for a scripting runtime spends engineering for zero capability gain.
5. **Binary/distribution** (D06, conf 4). Wasmtime is already integrated and paid for. Lua adds ~1 MB (cheap). Python cannot be shipped inside a single binary (R-3 fails). V8 adds tens of MB.

## The real dissents (not dismissed)

### D12 (LUA, conf 4) — "single .lua file deletes the whole pipeline"
A single sandboxed `.lua` file equals the authored, executed, distributed, and signed artifact — deleting the mirrors, committed JSON, blobs, guest crates, SDK macros, xtask extractor, and both sync guards. **Counter:** fails R-2 (shared address space) and R-6 (no fuel mechanism). The consolidation is real, but the sandbox cost is too high.

### D02 (LUA, conf 4) + D11 (LUA, conf 3) — hot reload and developer loop
Lua re-reads scripts instantly; WASM pays Cranelift recompile (~65–80 ms per 324–415 KB blob, or ~16 ms with real AOT deserialize). **Counter:** the authoring-model rebuild (Rust → wasm32) dominates, not the host-side reload — and real AOT fixes it.

### D08 (KEEP_WASM, conf 4) — the host API needs WIT, not a language swap
C7-03 (no IDL) is best fixed by adopting the WebAssembly Component Model with typed WIT interfaces — not by changing the plugin language. WASI 0.3 (ratified 2026-06-11) ships native async; Wasmtime 46+ enables it by default.

### D07 (consumer-surfaces, conf 3) — two of three surfaces already run native
LSP and MCP use native BuiltinRuntime mirrors — they never instantiate wasmtime. Under R-1, this native tier must be eliminated or the runtime decision is moot for 2/3 surfaces. The Lua camp correctly identifies this as a real cost of KEEP_WASM.

## Conditions for revisit

1. A mature **Lua-to-wasm/componentize bridge** emerges (write Lua, compile to wasm, get both authoring DX and sandbox)
2. **WIT/component-model IDL** ships with typed graph-access interfaces (closes C7-03, makes the host API type-safe)
3. A genuinely **sandboxed CPython distribution** appears (unlikely — GIL + C extensions)

## Execution plan (KEEP_WASM) — EXECUTED 2026-09-27

**Detailed, phased migration plan: [`migration-plan.md`](migration-plan.md)** — migrates all extension execution to WASM and deletes the native tier (C7-11). Phases: parity harness → scanner guests → validator exports → shared runtime constructor → LSP/MCP cutover → custom rules through wasm → perf/limits → deletion → verification.

> **Status: COMPLETE.** All extension execution — CLI, LSP, MCP — runs through the Extism/Wasmtime runtime. The native mirror tier (BuiltinRuntime, six native impls, NativeCustomRules, CompositeRuntime, EnginePool ledger, fake-AOT cache) is deleted, with a permanent gate test (`native_tier_gate.rs`) preventing reintroduction. Custom rules (E004/E006/E010/W010) dispatch through guest `validate__*` exports; fuel limits are engine-enforced (C7-10); the on-disk Wasmtime compile cache is wired (warm CLI runs save ~0.8 s). Audit C7-02 (honest removal), C7-04/09 (host-fn capability boundary), C7-08, C7-10, C7-11, and C10 are closed.

Audit-item mapping:

1. C7-04: sandbox fs deny-by-default

2. C7-10: wire max_execution_ms via wasmtime fuel/epoch
3. C7-02: real AOT via `PluginBuilder::compile()` + `with_cache_config`
4. C7-08: real warm-engine pool (reuse instantiated plugins)
5. C7-03: WIT IDL for the host API (component model)
6. C7-09: honor query_scope in `make_query_graph_fn`
7. Eliminate native mirrors once the wasm path is the only path (C7-11 closure)

Each item is a bounded slice with tests; none changes the plugin model.
