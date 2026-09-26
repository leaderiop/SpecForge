# Position — Holger Krekel (pytest creator; plugin/hook architecture)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The contract is the product, not the runtime. pluggy's lesson: hookspecs outlive implementations—JUnit XML outlived pytest across CI. SpecForge's equivalents, the nine `describe_*.json` manifest categories and `specforge-report.json` (`integrations/rust/specforge-test/src/report.rs`, `schema_version "1.0"`), are already language-neutral JSON. Switching churns plumbing the contract doesn't need.
2. The workload is ~97% generated manifest, ~3% logic (evidence.md: 629 guest LOC; only `@specforge/formal`'s four passes, ~478 lines, are real). Authoring ergonomics barely matter when plugins are mostly data; the one logic-bearing guest is already deterministic wasm—R-6 for free.
3. Only wasm gives memory isolation under R-2. Interpreter sandboxes (mlua, deno permissions) are enforced-by-convention with engine-version drift; PyO3 embedding is fragile for single-binary R-3. Wasm blobs are reproducible, hash-verifiable registry artifacts; collectors—untrusted code scraping arbitrary test formats, exactly what the `@specforge/pytest` adapter emits—are where isolation beats trust.
4. C7's findings are integration bugs: C7-04 allow-by-default sandbox, C7-10 unused `max_execution_ms`, C7-08 ledger engine pool—fixable inside wasm; a runtime swap resets 3,070 passing tests without touching C7-03's IDL debt every runtime needs.

## Biggest risk in my verdict

KEEP_WASM inherits an allow-by-default sandbox (C7-04): R-2 fails in practice until enforcement lands; that fix is unpromised work.

## What would change my mind

A Lua/TS runtime demonstrably running formal's four passes with enforced CPU/memory limits and reproducible registry artifacts—a sandbox as verifiable as wasm's; or a shift toward API-glue plugins where Rust-only authoring blocks AI agents.
