# Position — Patrick Thomson (stack-graphs creator; declarative name resolution, incremental analysis)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Determinism is my home turf. Stack graphs win because declarative rules compile into a graph and path-finding returns the same answer every run — no interpreter in the loop. Wasm keeps that property: guests are fixed bytecode, so analyze output over the ~1.7k-entity graph is bit-reproducible (R-6) and hash-verifiable in the registry (R-4). Scripting fails R-6 quietly: Lua 5.4 and LuaJIT disagree on integers, strings, metatables; Python adds hash randomization and interpreter drift. `linker.rs` resolving by exact symbol match is the same trade — deterministic beats clever-but-flaky.
2. The workload is already rules-not-code. Evidence §2: guests are ~97% generated manifest, ~3% logic — precisely the stack-graphs shape (data rules, small executable core). A manifest layer is runtime-agnostic; the interpreter choice only governs the 3%. Swapping runtimes for the 3% is tail-wagging-dog.
3. R-1's convergence is wasm-internal. `NativeCustomRules` (`crates/specforge-emitter/src/compile.rs`) already implements the `WasmValidationRuntime` seam natively — invert the C6-11 workaround: re-express those four rules as guest `validate__*` exports, delete the ~1,000-line mirrors in `specforge-emitter/src/builtins/`, and one mechanism remains. A scripting tier adds a fourth implementation, not convergence. C7-03's missing IDL is orthogonal: an IDL over `call_export` improves every candidate.

## Biggest risk in my verdict
Authoring ergonomics for AI agents (§1.5: the primary author). Rust plus a `wasm32` target is the hardest surface to agent-author reliably; if the registry's third-party bet never materializes, wasmtime's 485→511 dependency weight and the unexercised perf machinery (C7-02/08/10) buy nothing.

## What would change my mind
Measured evidence that agents author working Lua/QuickJS plugins at materially higher success rates, with a vendored-C embed meeting R-3 and version-stable semantics passing R-6 snapshot tests. MULTI never — it recreates C7-11 by design.
