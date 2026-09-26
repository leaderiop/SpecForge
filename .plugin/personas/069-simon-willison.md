# Position — Simon Willison (LLM-tooling critic; agent ergonomics)

**Verdict:** TYPESCRIPT
**Confidence:** 3

## Arguments

1. The plugin author of the future is an AI agent, and the current authoring surface taxes agents hardest: plugins require a Rust toolchain, the `wasm32-unknown-unknown` target, and a compile-publish loop (evidence §1.5; `crates/specforge-extension-sdk`). Agents iterate by edit→run; a cargo-plus-wasm cycle per edit burns tokens. TypeScript gives the instant loop agents already excel at.

2. The wasm guest payload is ~97% generated manifest, ~3% real logic (629 lines total; `formal`'s 478). Metadata should stay host-executed declarative JSON; a light scripting runtime carries only the logic — wasmtime plus ~9.4k LOC of machinery in `specforge-wasm` is disproportionate to a 478-line workload, and its audit trail is vaporware (C7-02 AOT byte-copy, C7-08 engine-pool ledger, C7-10 limits never enforced).

3. `deno_core`'s permissions model is the capability-scoped sandbox R-2 demands by construction — fixing C7-04 (fs allow-by-default) architecturally instead of patching Extism knobs. Static V8 still ships single-binary (R-3), and script files hot-reload trivially (R-5).

## Biggest risk in my verdict

C7-03 is the real disease: no IDL, stringly JSON over `call_export`. Swapping runtimes without a typed host API merely relocates drift; V8 adds maintenance weight and GC nondeterminism risk against R-6.

## What would change my mind

A truly fixable Extism sandbox (C7-04 closed) with enforced limits and a real IDL would make KEEP_WASM a fix-instead-of-swap. Or data showing agents author Lua as fluently as TS — then mlua's tiny footprint and CPU-instruction hooks win every other criterion.
