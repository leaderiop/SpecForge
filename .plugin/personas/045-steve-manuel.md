# Position — Steve Manuel (Extism creator; plugin-system ergonomics)

**Verdict:** KEEP_WASM

**Confidence:** 4

## Arguments

1. Isolation is the product. R-2 demands capability-scoped, deny-by-default execution of untrusted code; wasm delivers engine-enforced linear-memory isolation plus exactly four imported capabilities (query, emit_diagnostic, resolve_ref, read_file) — host functions as the only authority, per `docs/extension-protocol.md`. C7-04 (`fs_access` allow-by-default in `crates/specforge-wasm/src/sandbox.rs`) is broken enforcement of a sound model, not a refutation of it. Lua/QuickJS sandbox by interpreter convention (escape-prone); CPython offers none, and PyO3 also breaks R-3 (system Python).

2. R-3/R-4 favor static embedding: wasmtime compiles into the single binary; deno_core ships V8. The signed registry already moves .wasm blobs end-to-end — sha256 verify, signing keys, pinning.

3. The runtime isn't the bottleneck; the protocol is. Guests are ~97% generated manifest, ~3% logic (`formal`'s 478 lines is the only real logic). C7-03 (no IDL over `call_export`) and C7-11 (three parallel implementations: `crates/specforge-emitter/src/builtins/` mirrors + `NativeCustomRules` bypassing guest `validate__*`) are fixable inside this model — converge to one mechanism as R-1 demands. MULTI would institutionalize that drift instead of ending it.

## Biggest risk in my verdict

The host is an unenforced promise: C7-08 (EnginePool is a ledger), C7-10 (`max_execution_ms` dead), C7-02 (AOT cache is a byte-copy). Unfixed, KEEP_WASM is security theater whose per-call compile cost also drags R-5 hot reload.

## What would change my mind

Measured evidence that host-boundary capabilities can't enforce scoped fs/network at acceptable cost — or that Rust+wasm32 authoring measurably suppresses third-party registry adoption once real authors appear.
