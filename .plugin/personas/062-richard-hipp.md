# Position — D. Richard Hipp (embedded storage & supply-chain trust)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The registry already contracts in digested binaries. `crates/specforge-registry-server/src/db.rs` pins `sha256`, `signature`, `key_id`, `manifest` per version; `handlers.rs` serves only blobs hashing to the recorded digest (`INTEGRITY_VIOLATION` otherwise) and `storage.rs` commits via atomic rename. That chain attests *behavior* only if the artifact is a fixed compilation target. Distribute Lua/Python/TS source and sha256 verifies bytes while the local interpreter version decides semantics — R-4 reproducibility and R-6 snapshot determinism die at the digest.
2. R-3 is unconditional: wasmtime is static, zero system packages. PyO3 needs a Python distribution (fails outright), deno_core/V8 is heavy, LuaJIT vs 5.4 forks semantics per build. One binary, one behavior.
3. The audit indicts implementation, not format: C7-04 is a default to flip, C7-10 a missing meter, C7-08/C7-02 unbuilt engine work, C7-11 mirrors to delete. You don't abandon a file format because the pager has bugs — you get branch coverage until they close.

## Biggest risk in my verdict
Wasmtime's weight plus the vaporware perf story (C7-02, C7-08): if `watch` (R-5) latency or memory stays unacceptable and pooling proves unfixable, pressure for a lighter scripting tier grows.

## What would change my mind
A source runtime restoring digest-pins-behavior — vendored interpreter, frozen stdlib, semantics pinned per version — plus evidence the wasm pool/AOT gaps won't close; or a binary-size mandate wasmtime cannot meet.
