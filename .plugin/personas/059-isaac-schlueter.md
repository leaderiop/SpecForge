# Position — Isaac Schlueter (npm creator, registry trust)

**Verdict:** KEEP_WASM

**Confidence:** 4

## Arguments

1. **The lockfile argument.** `crates/specforge-wasm/src/lock_file.rs` already pins per-extension version and integrity hash: the package-lock.json pattern applied to plugin artifacts. A wasm blob is an immutable, content-addressable tarball: verify once at install (`add.rs` → `HttpRegistryClient`, sha256 + key pinning), runs identically forever, interpreter-independent. LUA/PYTHON/TYPESCRIPT distribute *source*, so R-4 reproducibility gains a second resolution axis: interpreter build and stdlib — the "works on my machine" world lockfiles kill.

2. **Sandbox architecture (R-2).** Wasm isolation + capability imports is deny-by-default *by construction*; C7-04 (fs allow-by-default) is a config bug — fix the allowlist. CPython has no sandbox and pip is ambient-network by design: a plugin install that shells out to pip is a supply-chain hole. Lua/QuickJS sandboxes are historically leaky (ffi/env escapes).

3. **R-3 and the audit.** Wasmtime is statically linked, no system packages. PyO3's system-Python requirement is disqualifying. The real wasm failures (C7-08 EnginePool ledger, C7-02 fake AOT, C7-10 unenforced `max_execution_ms`, C7-11 triple implementation) are all fixable inside this model; KEEP_WASM already promises exactly that.

## Biggest risk in my verdict

Performance and weight: wasmtime dominates the dep tree; with no warm pooling (C7-08), per-call compile could lose to `mlua`. Ship the fixes or the verdict is nostalgia.

## What would change my mind

A scripting runtime shipped as a pinned, integrity-hashed artifact with capability-scoped imports matching wasm's guarantees, plus evidence the Rust-toolchain SDK friction blocks real adoption. Ergonomics could then outweigh the artifact model.
