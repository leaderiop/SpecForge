# Position — Tony Hoare (CSP)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. Wasm is the only candidate with a normative formal semantics. Interpreter sandboxes enumerate denials — C7-04's allow-by-default `file_system_access` is the symptom — whereas wasm isolation is structural: memory isolation plus capability imports denies by construction. A provable invariant beats a remembered one.
2. R-1 forbids tiers; MULTI reintroduces them. The audit's findings — C7-10 (`max_execution_ms` never enforced), C7-02 (cache is a byte-copy), C7-08 (EnginePool is a ledger), C7-11 (three implementations) — are undischarged obligations of one mechanism, not defects of it. Switching engines discards a 9.4k-LOC runtime passing 3,070 tests to re-learn each obligation elsewhere.
3. The workload is analysis, which must be deterministic and statically checkable (R-6). `extensions/formal/src/lib.rs` holds the only real guest logic: four ~478-line `#[compiler_pass]` functions over the entity graph, plus E042's composition-cycle rule in `describe_validation_rules.json`. Rewriting passes in a dynamically typed interpreter degrades the proof-friendly core RES-25 points toward; guests are 97% manifest, so engine weight hits hosts, not authors.

## Biggest risk in my verdict

One runtime, one point of failure. If wasmtime's dependency weight and per-call compile cost survive the C7-02/C7-08 fixes, or Rust-to-wasm32 authoring deters the third parties the registry bets on, KEEP_WASM owns a dead ecosystem.

## What would change my mind

Evidence that C7-04 cannot be made deny-by-default, or pooling/AOT cannot meet analyze latency on 1.7k-entity graphs. A second source language targeting the same wasm protocol would fix ergonomics without a second runtime — still KEEP_WASM.
