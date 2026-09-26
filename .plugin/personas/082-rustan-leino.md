# Position — K. Rustan M. Leino (formal verification; VC generation, Dafny/Boogie)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. R-6 is a determinism requirement, and determinism starts with semantics: WebAssembly is the only candidate whose execution semantics is a formal, standardized object, so replayability is a property of the language, not a discipline. CPython (hash randomization, GC) and V8 (JIT, GC timing) inject nondeterminism into the oracles a future `specforge analyze --prove` needs; Lua's prose manual leaves metatable/GC corners loose.
2. Evidence.md: `@specforge/formal` is the only logic-bearing guest (478 lines, 4 `#[compiler_pass]` passes incl. condition_check) — the front half of a Boogie-style contracts-to-discharges pipeline, where W035-style obligation reporting demands snapshot-stable output. Growing that under the compile-time-checked Rust SDK beats porting typed graph algorithms into untyped glue; the import boundary is also where C7-03's missing IDL belongs.
3. C7-02 (byte-copy AOT), C7-08 (engine-pool ledger), C7-10 (unenforced `max_execution_ms`) are host gaps wasmtime natively closes — fuel and epoch metering ship in wasmtime 43. A runtime swap re-creates each gap, fixes none, and discards the 3,070-test suite pinning current semantics.

## Biggest risk in my verdict

KEEP_WASM is defensible only with findings closed. If condition_check and friends migrate into the host and plugins reduce to declarative manifests (guests are ~97% generated manifest already), wasmtime's 485→511 locked-dep weight buys nothing and mlua wins.

## What would change my mind

An interpreter with machine-checked, deterministic semantics and enforced fuel metering meeting R-3/R-4 — or measured evidence that AI-agent authors cannot sustain the Rust+wasm32 toolchain for the logic-bearing 3%.
