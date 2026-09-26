# Position — David L. Parnas (information hiding; requirements as tables)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. By the 1972 criterion the runtime is a hidden decision behind the extension interface (`__describe`, `__pass_*`, `validate__*`). C7-03 shows the real defect: no IDL — stringly JSON over `call_export`, drift already observed. Specify the interface; don't relocate the secret into every plugin's source.
2. R-2 concerns authority, not syntax. Wasm guests possess no ambient fs/network/process capability unless the host imports one; C7-04 (fs allow-by-default) is a mis-wired host default, fixable inside the model. No interpreter gives structural zero-ambient-authority; CPython's pip ecosystem is ambient capability by design.
3. C7-11 is the true decomposition failure: one decision (the four builtins' contributions) realized three ways — native mirrors in `crates/specforge-emitter/src/builtins/`, vendored blobs, guest crates — synchronized by guard tests, not design. R-1 demands one mechanism; that is repair, not replacement.
4. R-6 determinism underwrites the traceability chain (`crates/specforge-emitter/src/trace.rs`, artifact→span) and snapshot tests; wasm is deterministic and version-stable where CPython/V8 drift threatens reproducibility. Guests are ~97% generated manifest, ~3% logic: engine swaps optimize the wrong module; the Rust-toolchain authoring barrier is attackable at the SDK.

## Biggest risk in my verdict
KEEP_WASM retains 9.4k LOC of machinery and a Rust-only authoring surface misaligned with the AI-agent audience; the registry bet dies if non-Rust authors never come.

## What would change my mind
A scripting tier matching R-2's guarantee strength (instruction-bounded, zero ambient I/O), or guest logic growing far past 3% with non-Rust author demand.
