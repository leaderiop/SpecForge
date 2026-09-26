# Position — Agustín Borgna (petgraph maintainer)
**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The workload is graph-shaped, so the memory boundary matters more than authoring ergonomics. Passes receive a snapshot of `crates/specforge-graph/src/graph.rs`'s `nodes: HashMap<Sym, Node>` / `edges: Vec<Edge>` (~1.7k entities) over stringly JSON `call_export` (C7-03). Wasm's copy-in/copy-out makes that snapshot a *value*: guests cannot mutate host state or observe HashMap iteration order mid-rewrite. Interpreters sharing the address space (mlua userdata, PyO3 borrows) tempt proxy views over live maps; mid-traversal mutation or rehash-on-insert poisons R-6 determinism, which analyze snapshots need.

2. Runaway-pass budget. Graph passes are where accidental O(n²)/infinite loops live. Wasmtime has epoch/fuel interruption natively; C7-10 (`max_execution_ms` never enforced) is a host wiring bug, not a model defect. mlua's Lua 5.4 has no instruction hook (only Luau), QuickJS ships none — and R-5 hot reload re-runs passes on every edit, so a hanging pass freezes `watch`.

3. R-2 under R-1: with no trusted tier, `@specforge/formal`'s 478-line guest is as untrusted as any third-party plugin. Linear-memory isolation is a runtime-level guarantee; interpreter sandboxes are convention. Blobs also slot into the signed registry's sha256-verified artifact flow (evidence §1.4), serving R-4.

## Biggest risk in my verdict
Wasmtime's weight (485→511 locked deps) plus open C7-02/C7-08 (AOT byte-copy, no warm engines) may tax every watch re-run with recompilation.

## What would change my mind
A shared-memory ABI with value-semantics snapshots (typed views, documented layout, host-enforced budget), or proof mlua debug hooks give deterministic cross-platform instruction budgets plus an immutability story for graph payloads.
