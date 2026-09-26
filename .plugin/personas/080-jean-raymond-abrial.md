# Position — Jean-Raymond Abrial (Event-B / formal refinement)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **R-6 is a determinism obligation; wasm meets it by construction.** The `@specforge/formal` guest — the only logic-bearing plugin (478 lines: condition_check, layering_verify, event_graph_analyze, coverage_tracking, evidence.md §1.1) — computes over graph snapshots where identical input must yield identical diagnostics; RefinesTo/E041 checks live there. Wasm has no ambient fs/net/clock; CPython's import surface (§4) is ambient capability, making snapshot tests probabilistic.
2. **The audit findings are undischarged obligations, not model failures.** C7-02 (byte-copy "cache"), C7-08 (EnginePool ledger), C7-09, C7-10 (`max_execution_ms` never enforced) are declared-but-unenforced invariants in the *host*, not in wasm. Event-B refinement discipline: every step carries its obligations forward — switching preserves those debts, discharges none, resets 9.4k LOC (`specforge-wasm`) and vendored-blob distribution.
3. **C7-11 convergence is the real refinement step.** Three parallel implementations must collapse into one mechanism (R-1). Collapsing *inside* the wasm model — native mirrors (`crates/specforge-emitter/src/builtins/`) die, guests become single truth — is a small step already guarded by `extension_json_sync`; MULTI multiplies obligations instead of discharging them.

## Biggest risk in my verdict
Fuel-metering: C7-10 proves limits are currently convention, not enforcement. Wasm isolation without an enforced instruction budget leaves a nonterminating plugin as a permitted DoS that any runtime, wasm included, must discharge.

## What would change my mind
A candidate whose resource limits are provably enforced (Luau-style instruction hooks with host-checked fuel) and deterministic enough for R-6, plus a migration discharging C7-11 without a fourth implementation.
