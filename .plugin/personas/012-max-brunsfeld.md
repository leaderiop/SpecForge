# Position — Max Brunsfeld (parsing & grammar infrastructure)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Grammar contributions are already in the protocol, and only wasm can carry them.** Every builtin handshake declares `"grammars": false` (`extensions/*/src/handshake.json`); the SDK reserves the flag (`crates/specforge-extension-sdk/src/lib.rs:239-240`). A contributed `.spec` body grammar compiles to a tree-sitter parser, and tree-sitter grammars already distribute as `wasm32` modules (web-tree-sitter). An interpreter runtime either forecloses that reserved capability or reintroduces native dylib loading, violating R-2/R-3. Switching runtimes rips out a slot the protocol was designed around.
2. **Determinism (R-6) needs preemptive bounds.** Analyze output is snapshot-tested; wasmtime offers fuel/epoch interruption for hard execution limits. Audit finding C7-10 (`max_execution_ms` never enforced) is a host bug, fixable with epoch deadlines inside the existing model. Interpreter debug-hooks (mlua) are cooperative and racy; CPython has no usable preemption.
3. **The audit record indicts the host integration, not wasm.** C7-02 (byte-copy "AOT"), C7-08 (EnginePool ledger), C7-03 (no IDL) live in `crates/specforge-wasm/src/` (~9.4k LOC), not in wasm itself. Guests are ~97% generated manifest, ~3% logic (evidence.md §2) — no authoring-ergonomics crisis to fix by switching. Migration is total (SDK, macros, 4 blobs, registry blobs) to escape fixable bugs.

## Biggest risk in my verdict
wasmtime's dependency weight and per-call compile cost persist if pooling/AOT is never fixed — "keep" inherits C7-02/C7-08 indefinitely, and 1.4 MB of vendored blobs keep demanding sync guard tests.

## What would change my mind
Formally dropping grammar/body-parser contributions from the protocol, plus a proven dead end on warm engines — then mlua (vendored, static, instruction-count hooks) becomes the honest low-weight choice; or MULTI keeping wasm strictly as sandbox/distribution format.
