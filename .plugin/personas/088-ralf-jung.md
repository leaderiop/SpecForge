# Position — Ralf Jung (formal methods; semantic soundness, testing-as-oracle)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. R-1 demands one security model for all plugins; only wasm makes isolation a machine-checked property of the runtime (memory isolation + capability imports), not an interpreter convention. Lua/Python sandboxes are enforced-by-convention (restricted builtins, env tables) with a long escape history; R-2 there rests on trust — exactly what R-1 forbids.
2. The audit's findings are claimed-vs-enforced drift — my oracle's home turf: C7-04 (fs allow-by-default), C7-10 (`max_execution_ms` unenforced), C7-09 (`query_scope` ignored), C7-02 (AOT cache is a byte-copy). All fixable inside KEEP_WASM; switching runtimes discards the oracle — 3,070 passing tests plus `extension_json_sync`/`builtin_blob_sync` pin current semantics — leaving any replacement's sandbox claims unverified.
3. R-6: wasm32-unknown-unknown is deterministic by construction (no ambient clock/randomness unless imported), so snapshot oracles over analyze stay cheap; CPython injects nondeterminism (hash randomization, GC) into any oracle.
4. Evidence.md: guest payload is ~97% generated manifest, ~3% real logic (only `@specforge/formal`, 478 lines, already Rust) — scripting's ergonomics win is marginal; soundness differences are decisive.

## Biggest risk in my verdict

KEEP_WASM inherits open findings and wasmtime's weight (485→511 locked deps, 1.4 MB blobs). Endorsing it without closing C7-04/C7-10 blesses a false claim (C7-06): fix the drift, not swap the runtime.

## What would change my mind

A scripting runtime with verified deny-by-default capability enforcement plus enforced fuel/epoch interruption that still meets R-3/R-4 — or evidence that AI-agent authors cannot sustain the Rust + wasm32 toolchain for the ~3% logic-bearing surface.
