# Position — Chase Wilson (compiler infrastructure / interning)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. **The boundary, not the runtime, is the bottleneck.** Sym's serde resolves to `String` on serialize and re-interns on deserialize (`crates/specforge-common/src/interner.rs:120-130`): every pass payload pays resolve/re-intern at the plugin wall regardless of runtime. C7-03 is a missing IDL, not a wasm defect. An interned-handle protocol (numeric handles over per-plugin string tables) fixes payload cost inside wasm; sharing `Sym` directly, as an embedded interpreter would, forfeits R-2 isolation.

2. **Isolation and determinism for untrusted guests (R-2/R-6).** wasm linear memory is the only candidate with real isolation, deterministic execution, and zero system packages (R-3); PyO3 fails R-3 outright; mlua runs untrusted code in the host address space, one interpreter bug from escape. C7-04's fs allow-by-default is a host config bug, not a wasm flaw.

3. **Payoff asymmetry.** Guests are ~97% manifest, ~3% logic (evidence.md §2); swapping runtimes re-incurs ~9.4k LOC of lifecycle/registry/integrity machinery (`crates/specforge-wasm/`) for gains this workload barely exercises. Real AOT plus a warm pool (C7-02, C7-08) is engineering; a swap is a rewrite.

## Biggest risk in my verdict

Third-party authoring friction: a Rust+wasm32 toolchain per plugin is heavy for the AI-agent authors the registry bets on (evidence.md §1.5); if agents can't close the compile-publish loop cheaply, wasm starves the ecosystem it exists for.

## What would change my mind

Measured AI-agent round-trips failing against the Rust SDK; a demonstrated interned-handle IDL running all four builtins under two runtimes at ≤ JSON cost — making MULTI cheap, not duplicated surface; proof pooling/AOT can't be fixed, making per-call compile structural.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The stringly boundary and unfixed pooling are implementation debt fixable inside wasm; only wasm gives memory isolation plus static single-binary distribution for genuinely untrusted plugins.
