# Position — Walden Yan (context engineering; Cognition co-founder)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. A plugin is a pure context function: graph snapshot in, diagnostics out. Evidence.md §2 shows the guest payload is ~97% generated manifest, ~3% real logic (`@specforge/formal`'s 478-line lib.rs is the only logic-bearing guest). The context boundary — JSON over `call_export` — is already correct; the defect is C7-03 (no IDL), a protocol fix, not a runtime change. Switching languages swaps the container, not the contract.
2. WASM is the only candidate meeting R-2/R-6 as-built: memory isolation plus capability imports, deterministic output. C7-04 (allow-by-default fs) and C7-10 (`max_execution_ms` unenforced) are in-model config bugs — deny-by-default plus wasmtime fuel/epoch. mlua and V8 "sandboxes" are conventions, not isolation; PyO3 fails R-2 and R-3 outright.
3. The thesis (vision/north-star.md H2, README "Agents Are First-Class Consumers") bets the ecosystem on community extensions over one substrate. MULTI fragments what makes a context source trustworthy — one sandbox semantics, one reproducibility story under R-4 — and institutionalizes the C7-11 three-implementations drift the audit diagnosed.

## Biggest risk in my verdict
Authoring ergonomics: the SDK demands a Rust toolchain + wasm32 target from the AI-agent and hobbyist authors H2 depends on. If that friction suppresses third-party plugins, the registry bet fails while I defended the wrong invariant.

## What would change my mind
Proof that wasm toolchain friction blocks adoption even after an IDL (C7-03 fix) ships — then a scripting tier behind the same protocol, only if guest logic outgrows manifests.
