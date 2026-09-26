# Position — Shreya Rajpal (Guardrails AI co-founder; LLM output validation)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. Determinism is the validator's contract. Plugin checks must be deterministic and snapshot-testable (R-6) and reproducible through the signed registry (R-4); wasm blobs are version-pinned artifacts, sha256-verified at install (evidence §1.4). Embedded Python reintroduces environment drift into the very layer whose job is deterministic checks; Lua fragments across interpreter versions.
2. The damning fact — custom validation rules E004/E006/E010/W010 run natively via `NativeCustomRules` in `crates/specforge-emitter/src/compile.rs`, bypassing guest `validate__*` exports after C6-11 ("can never fire / fail silent") — indicts the wiring, not the runtime. A fail-silent guard is the cardinal sin; the fix is honest dispatch plus enforcing C7-04 (deny-by-default sandbox) and C7-10 (unused `max_execution_ms`), all inside the wasm model.
3. Workload reality: guests are ~97% generated manifest, ~3% logic (`@specforge/formal`'s 478-line passes are most of it). Authoring dominated by declarative JSON manifests doesn't justify new embedding surface; PyO3 fails R-2 (ambient capabilities) and R-3 (system deps) outright.

## Biggest risk in my verdict

C7-08: EnginePool is a ledger — if per-call compilation dominates `specforge watch`, the PRD-007 validate-fix-repeat loop slows, and the forcing function of agent iteration degrades.

## What would change my mind

Proof that fail-silent custom-rule dispatch cannot be made trustworthy under wasm, plus a Lua runtime demonstrating CPU-limit hooks and byte-identical snapshots across versions — Hub-style inspectable scripts would then beat opaque blobs.
