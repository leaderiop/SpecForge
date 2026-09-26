# Position — Nick Fitzgerald (Wasmtime/Cranelift engineer)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. Wasm is the only candidate satisfying R-2+R-3+R-6 simultaneously: linear-memory isolation with capability-scoped imports (C7-04 is a sandbox config bug, not a model limit), zero system dependencies for a static single binary, and deterministic replay for snapshot tests. PyO3 fails R-3 outright (system Python or a fragile bundled CPython); mlua/QuickJS give interpreter-level traps, not true isolation; deno_core drags V8 into the binary.

2. The damning C7 findings are host gaps my playbook exists to fix, not model failures. `crates/specforge-wasm/src/cache.rs:17-19` confesses the `.aot` artifact is a byte-copy (C7-02); Wasmtime's `Module::serialize`/cli-cache design makes true AOT a localized change. `engine_pool.rs:22-36` pools metadata records, not instances (C7-08). That is ~500 lines of rework versus discarding 9.4k LOC of working host.

3. The genuine debt — C7-03's stringly JSON over `call_export` — is runtime-agnostic: Lua/TS inherit the same boundary, so migration buys full re-porting of SDK macros, vendored blobs, and sync guards while keeping the problem.

## Biggest risk in my verdict

Authoring friction: guests are 97% generated manifest today (evidence §2), and Rust+wasm32 toolchains may strangle the AI-agent authoring the registry bets on, even with the superior host runtime.

## What would change my mind

Proof that serialized AOT plus a real instance pool cannot hold the watch/MCP cold-start budget, or a non-Rust wasm authoring path failing against TypeScript ergonomics at acceptable binary size.
