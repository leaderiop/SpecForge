# Position — Joseph Birr-Pixton (rustls lead maintainer)
**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. **Only candidate with a real sandbox.** wasmtime gives memory isolation plus capability-gated imports (R-2); alternatives inline a C interpreter (mlua, QuickJS) or ambient-capability runtime (CPython) into the host to execute untrusted plugins. My rustls decade exists because memory-unsafe C parsing untrusted input is unfixable by review; a C interpreter running untrusted bytecode reintroduces that class, in-process.
2. **Supply-chain and R-3 posture already lean pure Rust.** wasmtime 43.0.2 is static, no system packages; the registry path is pure Rust (`default-features = false, features = ["rustls-tls"]`, workspace `Cargo.toml:72`, inherited by `crates/specforge-registry/Cargo.toml:19`). Wasm blobs are sha256-pinned, signed, reproducible (evidence 1.4, R-4); source-distributed scripts move trust to author machines, discarding the proven live blob-integrity model.
3. **The audit record indicts host policy, not the runtime.** C7-04 (fs allow-by-default), C7-10 (`max_execution_ms` never enforced), C7-08 (engine pool ledger), C7-03 (no IDL) are all fixable inside KEEP_WASM; a runtime swap fixes none and discards 3,070 passing tests including wasm round-trip suites (evidence 2).

## Biggest risk in my verdict
KEEP_WASM becomes an excuse: C7-04/C7-10 stay open and the ship carries wasmtime's full weight (485→511 locked deps) while enforcing nothing — heavyweight runtime, zero sandbox.

## What would change my mind
Proof that capability enforcement is unfixable on extism 1.30/wasmtime 43, or demonstrated AI-agent authoring demand for non-Rust languages (SDK requires Rust toolchain plus wasm32 target, evidence 1.5). Then deno_core's permission model or Luau instruction-budget hooks — never C Lua or CPython.
