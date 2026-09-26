# Position — Martin Fowler (DSL design & evolutionary architecture)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. SpecForge's own thesis — semantics live in the model, not the grammar — decides this: the typed graph + Kind/Field/Edge registries are the product; the runtime is execution detail. Guests are ~97% generated manifest, ~3% logic (evidence.md §2); the declarative `describe_*.json` contract already carries the vocabulary. Lua/Python/TS add a second language surface needing its own versioning, sandboxing, and authoring docs — the accidental-complexity accretion the DSL book warns against.
2. R-1 is a convergence problem, not a runtime problem: three parallel implementations (`crates/specforge-emitter/src/builtins/*.rs` mirrors, vendored blobs, SDK) plus `NativeCustomRules` host dispatch (C7-11, C6-11; evidence §1.2). Swapping runtimes restarts duplication; deleting mirrors and routing `validate__*` through guests is behavior-preserving refactoring — the discipline `crates/specforge-migrate` operationalizes.
3. Option-preservation lives in the contract, not the engine: a typed IDL over `call_export` (C7-03) makes the runtime a swappable port. C7-04 sandbox defaults, C7-02 fake AOT, C7-08 ledger engine-pool must be fixed inside wasm anyway for R-2/R-5; no engine choice removes them.
4. R-3/R-6 favor wasm: no system packages, deterministic execution; CPython embedding is "notoriously fragile" (§4), LuaJIT risks ambient nondeterminism. TS authoring can later compile to wasm32 without host changes — authoring surface and engine are separable.

## Biggest risk in my verdict

Wasmtime's weight (485→511 locked deps) and per-call compile cost; if convergence stalls, wasm stays a facade over three implementations and R-1 remains violated.

## What would change my mind

Evidence that real plugins need ambient ecosystems (pip/npm libraries) capability-scoped wasm imports can't expose under R-2 — or a MULTI design proving a scripting tier wouldn't recreate a trusted first-party tier.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The semantic model is the product and the manifest is the contract — converge on one wasm mechanism behind a typed IDL rather than adopt a second language that must itself be versioned and sandboxed.
