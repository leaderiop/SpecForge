# Position — Tom Preston-Werner (SemVer author; version-compatibility policy)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. **The version contract is only mechanically enforceable at a language-agnostic boundary.** `validate_protocol_version` (crates/specforge-wasm/src/protocol/host.rs:49) parses both sides with `semver::Version`, enforcing same-major = compatible. Lua/Python/TS explode the surface into interpreter-version × host-API-version × package-version; semver negotiation loses its single choke point. A precompiled artifact has one version.
2. **R-4 reproducibility: a `.wasm` blob *is* the resolved lockfile output.** The registry pins sha256, signature, and key_id per blob (INTEGRITY_VIOLATION on mismatch, evidence §1.4). Source scripts reintroduce dependency resolution: ranges (`peer_dependencies`, E027 in lifecycle.rs/upgrade.rs) terminate deterministically only in a locked artifact. Evidence §4 flags pip's ecosystem as ambient-capability — source-distributed Python contradicts R-2 by construction.
3. **The open audit defects are contract-discipline failures, not wasm failures.** C7-03 (no IDL), C7-02 (byte-copy "AOT"), C7-08 (EnginePool ledger), C7-10 (unenforced `max_execution_ms`) persist under any runtime; swapping substrates re-learns them while discarding working machinery — handshake versioning, peer-dep cycle detection (toposort.rs), uninstall dependents (uninstall.rs).

## Biggest risk in my verdict

Authoring ergonomics: guests are ~97% generated manifest, ~3% logic (evidence §2), yet the SDK demands a Rust toolchain + wasm32 target. That friction threatens the third-party registry bet — and wasmtime's 485→511 locked deps must keep earning their weight.

## What would change my mind

Fix C7-03 with a real IDL and versioned schemas so runtime choice becomes invisible at the handshake; then a capability-scoped Lua tier behind it (MULTI) is defensible. Or evidence that wasmtime embedding breaks R-3 binary budgets irrecoverably.
