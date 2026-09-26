# Position — Sean McArthur (Rust HTTP infra maintainer; hyper/reqwest)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. reqwest's own history is the R-3 argument: rustls-by-default made `cargo add reqwest` work with zero system packages; native-tls (OpenSSL) was per-platform pain. PyO3 is native-tls again: evidence.md §4 flags Python's "system Python or bundled distribution" problem. Wasmtime is rustls: static, nothing to install.
2. The open audit findings are pooling bugs, not engine bugs — my literal domain. C7-08's EnginePool is "a ledger, no warm instances": a connection pool that never reuses connections. C7-10's unenforced `max_execution_ms` is a wasmtime epoch-interruption patch. C7-04 is a config flip. You fix the pool; you don't swap the protocol.
3. R-1 convergence is deletion, not migration: drop the native mirrors in `crates/specforge-emitter/src/builtins/` (C7-11), route custom rules through guest `validate__*` exports (host-native bypass today, evidence §1.2), keep vendored blobs in `crates/specforge-extism/src/builtins.rs`. Guests are ~97% generated manifest; the only real logic is ~478 lines in formal.
4. R-4 favors compiled artifacts: my anchor `crates/specforge-registry/src/client/http_client.rs` ships sha256-verified blobs; a signed blob is reproducibly verifiable in a way source-language plugins with ambient dependency graphs are not.

## Biggest risk in my verdict

"Keep" gets read as license to leave C7-02/C7-08/C7-10 open: cold starts keep taxing CLI startup, wasmtime's weight never gets trimmed, and scripting wins ergonomics by default.

## What would change my mind

Adversarial proof that an mlua interpreter sandbox (stripped stdlib, instruction hooks) actually meets R-2, plus registry evidence of third-party authoring demand the Rust SDK macros cannot absorb.
