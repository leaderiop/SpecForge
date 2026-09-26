# Position — Arnaud Roques (PlantUML creator; text-to-UML, swappable backends)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. PlantUML's lesson: the durable asset is the stable input contract, not the engine behind it — dot → Smetana → ELK swapped behind one seam. SpecForge's seam is the `describe_*`/`call_export` protocol — evidence.md's weak point (C7-03, no IDL, drift already happened). Harden the seam; don't swap it.
2. R-3 is the Smetana story: delegating layout to a system `dot` binary broke distribution; absorbing a pure in-process engine fixed it. wasmtime is static (evidence §4); PyO3 needs system Python — re-introducing the dot mistake.
3. The workload barely needs a language runtime: 629 lines of guest lib.rs, ~97% manifest, ~3% logic (evidence §2). The friction is the Rust+wasm32 toolchain for JSON manifests (9 `describe_*.json` categories). Fix C7-11 with a manifest-only declarative path through the same registry/protocol; keep wasm for the logic (`@specforge/formal`'s passes).
4. Open HIGH findings (C7-02 byte-copy AOT, C7-08 ledger engine pool, C7-10 unused `max_execution_ms`) are engine defects *inside* the wasm model — fixable without re-auditing a new sandbox under R-2.

## Biggest risk in my verdict

The Rust-toolchain authoring wall persists; if third parties won't cross it, the signed-registry bet (R-4) fails. Mitigation: ship the manifest-only path in the same release.

## What would change my mind

Proof that pass logic grows until Rust compile cycles dominate iteration, or authoring attrition from the wasm toolchain — then a Lua tier behind the same IDL-hardened protocol becomes the ELK moment.
