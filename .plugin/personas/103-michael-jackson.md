# Position — Michael A. Jackson (Requirements engineer — Problem Frames / JSP)

**Verdict:** LUA

**Confidence:** 4

## Arguments
1. **Frame the problem before the machine.** The verified problem is small: ~97% declarative manifest, ~3% logic — `@specforge/formal`'s four passes, ~478 lines (evidence.md §2). A machine (wasmtime 43, ~9.4k LOC in `crates/specforge-wasm/`) an order larger than its problem invites vaporware: C7-02's byte-copy "AOT cache", C7-08's warmless engine pool, C7-10's unenforced `max_execution_ms`. Solution machinery unmoored from problem description — domain leaked into the machine.
2. **JSP: mirror the problem's data structure.** Plugins are data-shaped: static manifests plus pure functions over graph snapshots (`__pass_*` contract). A `.lua` script mirrors exactly that — tables as manifest, functions as passes — and makes hot reload (R-5) the natural edit→rerun loop of `specforge watch`.
3. **Authoring is a problem-domain fact.** `spec/features/*.spec` pairs each feature's `problem` with its `solution`; today's SDK forces authors (AI agents, evidence §1.5) to carry Rust + `wasm32-unknown-unknown` toolchains — solution cost leaking into the problem. Lua (vendored C, evidence §4) keeps R-3 single-binary and R-2 deny-by-default honest; CPython fails R-3, V8 bloats it, MULTI adds a fourth tier to C7-11's three implementations, betraying R-1.

## Biggest risk in my verdict
The 3% is load-bearing: rewriting `@specforge/formal`'s passes in Lua risks Rust performance and expressiveness; sync guards and 629 guest LOC must migrate.

## What would change my mind
Evidence that plugin logic exceeds interpreter-scale compute or needs Rust crates; or KEEP_WASM actually closing C7-02/03/04/08/09/10 within its 9.4k-LOC budget.
