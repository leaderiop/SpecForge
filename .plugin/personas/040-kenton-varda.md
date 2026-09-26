# Position — Kenton Varda (schema evolution & serialization)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Wasm's imports-as-capabilities model is the only candidate that *structurally* satisfies R-2: linear-memory isolation plus an empty default import set means no ambient fs/network/process. C7-04 (fs allow-by-default) is a host config bug in the `sandbox` module of `crates/specforge-wasm/src/`, not a property of the runtime. Lua/Python sandboxing is interpreter convention with long escape histories; convention is not enforcement.
2. The real disease is C7-03: no IDL — stringly JSON over `call_export`, and drift already happened. Switching runtimes keeps stringly JSON, just across a different FFI. The fix is the field-number discipline SpecForge already practices: `SchemaVersion`/`negotiate_version`/`SchemaMigrationChange` in `crates/specforge-emitter`. Define a versioned, compact wire schema (postcard is already in the extism path) for the plugin boundary.
3. R-3/R-4/R-6 line up: wasmtime links statically (no system packages), blobs are sha256-verified through the signed registry, execution is deterministic. PyO3 fails R-3 outright (system Python or bundled distro); deno_core swaps one heavy dep for another. Evidence: guests are ~97% generated manifest by LOC — the cost center is manifest *data*, deserving a compact encoding (precedent: `schema/specforge-binary-report.schema.json`), not a new runtime.

## Biggest risk in my verdict
The performance story is currently vaporware — C7-02 AOT is a byte-copy, C7-08 EnginePool is a ledger, C7-10 `max_execution_ms` unenforced. If pooled warm instances never land, per-call compile dominates analyze latency and wasm becomes a liability I defended.

## What would change my mind
A scripting tier that verifiably enforces R-2 (Luau-style instruction caps plus independent sandbox-escape audits) and evidence the ~3% logic workload — collectors scraping external test formats — needs an ecosystem Rust guests cannot reach.
