# Position — Ben Hutton (JSON Schema 2020-12 spec lead)
**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. **The standard is the contract, not the runtime.** C7-03's no-IDL, stringly-typed JSON over `call_export` blocks third parties more than any runtime choice. Publish the wire contract as versioned JSON Schema with fixed `$id`s (`specforge schema --publish` already emits draft 2020-12), plus a Bowtie-style compliance harness running normalized tests against every plugin implementation. A runtime swap fixes none of this; contract formalization is orthogonal and overdue.
2. **Verifiability fits wasm's artifact model.** R-4 is already real: sha256-verified downloads against DB, pinned signing keys. Byte-addressable wasm blobs are reproducible registry artifacts; `.py` plugins with ambient pip capabilities are not, and PyO3/CPython fails R-3 outright (system deps).
3. **The R-1 debt is convergence, not runtime.** C7-11's three parallel implementations (`crates/specforge-emitter/src/builtins/` mirrors, vendored blobs, SDK crates) survive any swap. Delete the mirrors; route `validate__*` through one path — the C6-11 native-dispatch fix proves the wasm path was never trusted; restore trust rather than migrate distrust.
## Biggest risk in my verdict
KEEP_WASM as-is enshrines vaporware: C7-04 allow-by-default `file_system_access`, C7-10 unenforced `max_execution_ms`, C7-08 EnginePool as ledger. This verdict is conditional on fixing these inside the wasm model — otherwise R-2 "sandboxable" is a claim, not a guarantee, and a spec-led standard cannot rest on unenforced contract text.
## What would change my mind
A Luau-style sandbox with CPU-instruction limits and provably stripped `io`/`os` (testably equivalent to wasm isolation for R-2) plus adoption data showing Rust+wasm32 toolchain friction is killing third-party registry authoring — then the smaller footprint wins.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Formalize the wire protocol as published schema and converge the three implementations inside wasm — the byte-addressable artifact model is the only candidate satisfying R-4 verifiability and R-2 sandboxing today.
