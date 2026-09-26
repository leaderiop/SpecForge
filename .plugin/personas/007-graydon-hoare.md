# Position — Graydon Hoare (Rust creator; systems discipline & diagnostics)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. R-2 and R-6 are structural in wasm, mere policy in interpreters: mlua/quickjs share the host heap behind toggles; CPython embedding is "notoriously fragile" and pip is an ambient-capability ecosystem (evidence §4). Signed, vendored blobs (`extensions/*/wasm/`, sha256-verified — evidence §1.4) make R-4 reproducibility a *content* property; `.py`/`.lua` sources drift with interpreter versions.
2. The workload cannot fund a runtime swap: guests are ~97% generated manifest, ~3% logic (629 lib.rs LOC; only `@specforge/formal`'s 478 lines are real — evidence §2). C7-03 no-IDL, C7-04 fs-allow-by-default, C7-08 EnginePool ledger, C7-10 unenforced `max_execution_ms`, C7-11 triple implementation are host-discipline failures; fixing them in place satisfies R-1, policed by the existing `extension_json_sync` guard test.
3. `spec/product/journeys.spec:97` promises "errors printed in rustc style with suggestions," and `spec/glossary.spec:93` already partitions diagnostic codes between core and extensions. Span-carrying diagnostics across a wasm boundary force exactly the IDL C7-03 flags as missing — extend `specforge-extension-sdk-protocol-types` so traps become teachable errors, not stringly JSON.
4. AI-agent authoring ergonomics do not require a scripting tier: diversify front-ends that lower to one wasm ABI. MULTI forks the sandbox story per tier and re-opens R-2.

## Biggest risk in my verdict
If AI agents cannot bear a Rust+wasm32 toolchain per plugin (evidence §1.5), the registry bet dies with the runtime — and wasmtime's weight (485→511 locked deps) persists until C7-02/C7-08 make AOT/warm engines real.

## What would change my mind
Benchmarks showing a Lua/JS tier passing the same R-2 capability-denial and R-6 determinism conformance suite with materially better cold start, plus a concrete C7-11 convergence plan — capability data, not taste.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Wasm is the only candidate where isolation, reproducibility, and determinism are structural rather than policy, and every open C7 finding is host-side discipline fixable without a runtime swap.
