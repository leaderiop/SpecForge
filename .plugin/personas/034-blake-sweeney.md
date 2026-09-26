# Position — bluss / Blake Sweeney (petgraph author; graph data structures)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **The boundary, not the runtime, is the cost.** Guests are ~97% generated manifest, ~3% logic (629 lines; `formal` 478). Per-call cost is dominated by serializing the ~1.7k-entity snapshot plus module instantiation — C7-08 shows EnginePool is a ledger with no warm instances, so every call pays cold start. That is the StableGraph lesson: keep the structure alive, reuse free slots. Fix pooling inside wasm; a runtime swap re-creates cold-start in mlua/V8 with weaker isolation.
2. **Isolation is structural, not policy.** Wasm memory isolation is capability-by-construction (R-2). Lua/Python sandboxing is subtraction (strip `os`/`io`, hope nothing re-imports); C7-04 shows even wasm has an allow-by-default hole — a config flip in `crates/specforge-wasm/src/sandbox`, not grounds to migrate.
3. **Determinism (R-6) lives host-side.** `crates/specforge-graph/src/graph.rs` iterates `HashMap<Sym, Node>`; unordered iteration breaks snapshot tests regardless of runtime. The real lever is indexmap-style ordered tables — no runtime swap buys this.

R-1 convergence (C7-11) means deleting `crates/specforge-emitter/src/builtins/*.rs` mirrors and routing `NativeCustomRules` through guest `validate__*` — guests stay Rust, so SDK macros and sync-guard tests carry over.

## Biggest risk in my verdict
Real AOT (C7-02 is a byte-copy) plus warm pooling is unproven engineering; if per-call wasm overhead still dominates analyze latency afterward, KEEP_WASM inherits perf debt.

## What would change my mind
Profiling showing warm-instance wasm overhead, not payload serialization, dominates pass latency — or a Lua/JS embedding with structural (not policy) capability isolation and a materially smaller binary.
