# Position — Gojko Adzic (Specification by Example; living documentation)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. This is a traceability-trust decision: `specforge trace` (crates/specforge-cli/src/trace.rs) and specforge-report.json mean something only if collectors can't fabricate outcomes; collectors are the one plugin class touching the outside world. Under R-1 every plugin is untrusted, so R-2 is what makes the report living documentation instead of marketing. Wasm isolation plus capability-scoped imports is the only candidate where "collectors read only host-passed directories" is structural, not convention.
2. R-6 demands byte-stable, snapshot-testable output; wasm delivers today (evidence.md: 3,070 passing tests, round-trip suites included). Python fails structurally (GIL, ambient pip, version drift); Lua/TS need discipline wasm gives by construction, keeping R-4 reproducibility verifiable rather than pinned-interpreter folklore.
3. C7 findings are fixes inside the model, not model failures: C7-04 allow-by-default sandbox, C7-10 unenforced max_execution_ms, C7-08 ledger EnginePool. The guest payload is ~97% manifest, ~3% logic (formal, 478 lines; evidence.md) — converging the three parallel implementations (C7-11) into one sandboxed path is bounded.

## Biggest risk in my verdict

Authoring friction: Rust + wasm32 toolchains are the worst surface for AI-agent co-authoring, and the registry bets on third-party authors. If nobody pays that cost, a technically sound runtime dies unused.

## What would change my mind

Measured evidence that AI agents author sandboxed Lua/TS plugins at substantially higher success rates while mlua/quickjs capability denial meets R-2 and byte-determinism meets R-6 — then a single scripted runtime beats wasm. Or if AOT/pool fixes can't cut per-call overhead.
