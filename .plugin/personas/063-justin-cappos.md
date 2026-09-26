# Position — Justin Cappos (supply-chain integrity, TUF/in-toto)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The distributed unit must stay an opaque, content-addressed binary — that is what makes R-4 achievable. `crates/specforge-wasm/src/install.rs` verifies SHA-256 before atomic placement (E032 on mismatch), `lock_file.rs` pins `wasm_hash` plus publisher `key_id`, and `lifecycle.rs` re-verifies the on-disk binary against the lockfile pin at every load. Swap to Lua/TS source scripts and "what runs" becomes interpreter-version-dependent per machine; a digest then attests text, not behavior. Python is worse: PyO3 implies system Python (R-3 fails) and pip's ambient-capability ecosystem (R-2 fails).
2. R-1 is already true on the wasm path: the four builtins are vendored blobs (`extensions/*/wasm/`, embedded via `include_bytes!`) passing the same E032/E028/W027 gates in `integrity.rs` as any registry plugin. The real defect is C7-11's three parallel implementations — a convergence chore, not a runtime swap. Reproducibly-buildable vendored blobs are also exactly what in-toto-style attestation needs to prove builtin bits match source.
3. wasmtime 43 statically embeds memory isolation plus capability imports with zero system packages (R-3).

## Biggest risk in my verdict
Install-time verification is not runtime confinement. With C7-04 (`file_system_access` allow-by-default) and C7-10 (`max_execution_ms` never enforced) open, KEEP_WASM defends a paper guarantee; wasmtime's dependency weight compounds it.

## What would change my mind
A MULTI tier preserving chain-of-custody: script sources published with reproducible-build attestations, compiled at install behind the same lockfile digest discipline. Absent that machinery, any runtime swap destroys the integrity substrate.
