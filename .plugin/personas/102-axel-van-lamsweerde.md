# Position — Axel van Lamsweerde (KAOS requirements engineer)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. Treat R-1..R-6 as a goal model, refine per candidate. WASM structurally refines R-2 (memory isolation, explicit capability imports — evidence §4), R-3 (static single binary), R-4 (hashable vendored blobs). Alternatives promise what they cannot structure: CPython ships no sandbox and needs a system interpreter; deno_core permissions are a config layer — and audit C7-04 (fs allow-by-default) shows capability models fail through defaults, not design.
2. Obstacle analysis sorts the C7 findings: C7-04/C7-08/C7-10 are unoperationalized requirements (flip a default, finish the pool, enforce the deadline), C7-11's mirrors retire by converging one mechanism — none obstructs WASM itself. Switching runtimes to escape an implementation backlog abandons the only structure meeting the hard goals.
3. Operationalization check: the feature→behavior→invariant descent runs through the plugins themselves — `@specforge/formal`'s four `#[compiler_pass]` functions (~478 lines, `extensions/formal/`) maintain invariants over the graph snapshot; R-6 determinism is the analyze pipeline's invariant. Deterministic wasm execution matches; GIL/interpreter drift is nondeterminism at requirements level. Guests are ~97% generated manifest — scripting ergonomics buy ~3% of the payload.

## Biggest risk in my verdict

Structure alone is not a satisfaction argument: if C7-02/C7-08 stay vaporware, R-5 hot reload fails and KEEP_WASM becomes an undemonstrable goal model still paying wasmtime's weight.

## What would change my mind

Evidence that a pinned-semantics Lua runtime enforces R-2 via instruction-counting hooks, preserves R-6 determinism and the R-4 signed-vendored chain, and cuts wasmtime's dependency weight materially.
