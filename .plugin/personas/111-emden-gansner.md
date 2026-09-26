# Position — Emden Gansner (Graphviz layout algorithms)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Determinism is the product's spine (R-6). My emitters prove it: `crates/specforge-emitter/src/dot.rs` sorts edges before emitting and tests hostile-title escaping byte-for-byte; snapshot tests only mean something if plugin-contributed passes are reproducible. `@specforge/formal`'s `layering_verify` pass is literally a layered-ranking check over the graph snapshot (evidence.md §1.1) — deterministic, CPU-bound algorithm work a compiled wasm guest carries natively; interpreter float/stdlib drift adds risk for zero gain.
2. The guest payload is ~97% generated manifest, ~3% real logic (evidence.md §2). Language choice barely touches authoring ergonomics because a plugin is mostly declarative JSON already; the 3% that computes deserves the fast, memory-isolated path. Lua buys ergonomics for glue that mostly does not exist.
3. The open audit gaps are host-policy bugs, not wasm properties: C7-04 (fs allow-by-default), C7-10 (`max_execution_ms` never enforced), C7-03 (no IDL) are all fixable inside KEEP_WASM. Switching runtimes discards ~9.4k LOC of host plus signed-blob registry integration (R-4) without fixing any of them.

## Biggest risk in my verdict
Authoring friction: the Rust + wasm32 toolchain and recompile-per-edit may throttle third-party and AI-agent plugin uptake, and C7-08's missing warm engines make per-call compile cost real in watch loops (R-5).

## What would change my mind
If manifests become pure data (no code) and only formal-style passes remain, and third-party authoring measurably stalls on the toolchain, a Lua tier with CPU-limit hooks behind one protocol (MULTI) would beat purity.
