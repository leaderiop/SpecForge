# Position — Gernot Heiser (verified-systems engineer, seL4)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. Verification survives change only when shipped artifacts and sources are machine-checkably in lockstep. SpecForge already has that: the `builtin_blob_sync` and `extension_json_sync` guard tests assert every vendored `extensions/<name>/wasm/*.wasm` blob embeds the current payloads — a reproducible, hashable artifact chain the signed registry (R-4) can verify per release. Script languages distribute source; runtime semantics then drift with interpreter versions, quietly breaking reproducibility.
2. R-2 is Wasm's native model: memory isolation plus no ambient imports — capabilities granted explicitly. Evidence §4: CPython sandboxes nothing by default and pip is ambient-capability; Lua interpreter sandboxes have a long escape history. Python also fails R-3 (system or bundled interpreter).
3. The audit rot — C7-02 AOT byte-copy, C7-08 engine ledger, C7-10 unused deadline, C7-11 three parallel implementations — is implementation debt fixable inside this runtime, exactly like retiring a stale proof model: converge the native mirrors away and one mechanism remains. With guests ~97% generated manifest (evidence §2), authoring ergonomics are a minor cost.

## Biggest risk in my verdict

Today's "verified" story is guard tests, not enforcement: C7-04 sandbox is fs allow-by-default and C7-03 lacks any IDL. KEEP_WASM ships unverifiable claims unless these close; wasmtime's weight remains a tax.

## What would change my mind

If R-5 hot reload can't reach acceptable latency because C7-02/C7-08 stay vaporware — per-edit re-verification too slow — a capability-hardened Lua with instruction-counting hooks becomes the honest, lightweight alternative.
