# Position — Niklaus Wirth (Lean-software advocate, DSL designer)

**Verdict:** LUA
**Confidence:** 4

## Arguments
1. **Size the runtime to the programs it runs.** evidence.md: the four guests total 629 lines of lib.rs — ~97% generated manifest, ~3% logic (all `@specforge/formal`). The substance is declarative: 9 `describe_*.json` categories. Wasmtime 43.0.2 dominating the dep tree (485→511) for a payload of a few pages inverts Project Oberon's discipline: small, kept small by design.
2. **`crates/specforge-wasm/` (~9.4k LOC) is the accidental complexity "A Plea for Lean Software" condemns**: C7-02 AOT cache is a byte-copy, C7-08 EnginePool a ledger, C7-10 `max_execution_ms` never enforced. A vendored-C interpreter (`mlua`, Lua 5.4) deletes the layer, satisfies R-3, trivializes R-5, and forces the C7-11 triplication (mirrors, blobs, SDK) to converge as R-1 demands.
3. **The 5-minute-DSL principle applies to authoring**: manifests stay declarative signed JSON (guarded by `extension_json_sync`, satisfying R-4); Lua is the capability-scoped escape hatch for the 3% — no fs/net by default meets R-2; defined integer semantics keep R-6 determinism. MULTI is accretion, not design; PYTHON fails R-3; TYPESCRIPT repeats the bloat.

## Biggest risk in my verdict
Porting `@specforge/formal`'s four passes from typed Rust to untyped Lua invites semantic drift: layering_verify and event_graph_analyze rely on invariants the type system guarded. Migrate stepwise — one pass, snapshot-tested, before the next.

## What would change my mind
Evidence that passes need Rust-level performance or typed graph structures a JSON snapshot cannot express — or that sandboxable QuickJS carries the same passes lighter than wasmtime, inheriting the lean argument.

## Verdict

**Verdict:** LUA
**Confidence:** 4
**One-line rationale:** Plugins are 97% declarative manifest and 3% logic; a vendored tiny interpreter serves both under one equal mechanism, deleting 9.4k LOC of half-vaporware wasm machinery while meeting R-1..R-6.
