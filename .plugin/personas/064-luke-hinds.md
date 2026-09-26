# Position — Luke Hinds (Sigstore co-founder; supply-chain trust)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. R-4's trust chain already exists for blobs: `crates/specforge-registry-server` pins SHA-256 at publish, stores signature + key_id (`handlers.rs`), and clients pin keys at install; `crates/specforge-wasm/src/integrity.rs` verifies the artifact. Wasm means what you verify is exactly what executes — bytes frozen at publish, not a script whose semantics drift with the host interpreter's version. Vendored, digest-pinned blobs (the C7-00 fix) keep analyze output reproducible (R-6).
2. R-2 sandboxing is structural in wasm: memory isolation + capability imports. C7-04 (fs allow-by-default) is a host policy bug — fix the table, don't switch runtimes. Alternatives are weaker: CPython has no real sandbox and PyO3 breaks R-3; mlua's isolation is conventional; V8 escapes recur.
3. Migration cost: swapping runtimes orphans ~9.4k LOC of `specforge-wasm` and the signed-blob registry semantics. The open C7 gaps (byte-copy AOT cache, unenforced `max_execution_ms`, no IDL) are fixable in place.

## Biggest risk in my verdict

Guests are ~97% generated manifest, ~3% logic (evidence.md) — wasm overhead paid for a workload scripts carry cheaper. With only four builtins as plugins, the Rust toolchain + wasm32 target requirement is an adoption wall that kills the registry bet.

## What would change my mind

Evidence that source distribution carries equal trust: signed manifests pinning interpreter version + script digest, host-enforced pinning at install, transparency-log-backed publishes (the Rekor model) — matching blob-attestation strength without wasm.
