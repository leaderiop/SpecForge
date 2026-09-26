# Position — Ian Sommerville (requirements engineering & validation discipline)
**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. Derive the runtime from validated requirements, not authoring taste. R-2 (capability-scoped sandbox), R-3 (single binary, no system packages), R-4 (verifiable, reproducible artifacts), R-6 (deterministic, snapshot-testable output) are jointly met only by wasm: memory isolation plus capability imports, static linking, hash-verified blobs, deterministic execution (evidence.md §4). PyO3 structurally fails R-2 (pip is ambient capability) and R-3; V8/quickjs erode R-3's binary budget and R-6's determinism.
2. The audit indicts implementation, not abstraction. C7-04 (sandbox allow-by-default), C7-10 (`max_execution_ms` unenforced), C7-09 (`query_scope` ignored) are declared invariants — precisely the features→behaviors→invariants chain my discipline lives in — that the host fails to enforce. That is a verification gap fixable inside `crates/specforge-wasm/` (sandbox, engine_pool); switching runtimes re-elicits these same requirements into a weaker sandbox.
3. Ergonomics is a weak driver: the primary authors are AI agents, and builtins are ~97% generated manifest via SDK macros (`specforge-extension-sdk`), ~3% real logic. Language choice barely moves the workload.
4. MULTI multiplies the validated surface — every runtime re-carries R-2..R-6 — cost with no stated requirement behind it.
## Biggest risk in my verdict
Third-party authors may be non-Rust (the registry's entire bet), and KEEP_WASM is out of compliance with R-2 as built; my verdict assumes C7-04/C7-10 actually close.
## What would change my mind
Elicitation showing real plugin authors are human scripters, or a demonstrated Lua sandbox meeting R-2 with enforced capability and CPU bounds while keeping R-3/R-4/R-6 cheaper to validate than wasm.
