# Position — Yehuda Katz (registries, packaging & supply-chain trust)

**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. The lockfile already encodes the Bundler/Cargo contract: `LockFileEntry` in `crates/specforge-wasm/src/lock_file.rs` pins `name`, `version`, `source`, `wasm_hash`, and publisher `key_id`. Resolution yields one content-addressed artifact per extension; install re-verifies sha256 against the registry DB (INTEGRITY_VIOLATION on mismatch) — reproducibility under R-4 for free. A `.lua`/`.py` plugin has no such identity: behavior depends on interpreter version *and flavor* — LuaJIT vs 5.4 are semantically different languages. The lockfile would need a second resolution axis (runtime × interpreter × version) with per-platform drift, exactly the nondeterminism R-6 forbids.
2. R-1 favors artifact-uniformity. Builtins and third parties are the same object: one wasm blob flowing through one publish→verify→install pipeline (evidence.md §1.4, keys pinned at install). A scripting tier invites a "host-native scripts" trusted tier — the exact thing R-1 forbids.
3. MULTI is the worst outcome from a resolver standpoint: runtime requirements multiply the diamond-conflict surface and double the signed-verification surface. Ship one unit kind.
## Biggest risk in my verdict
Authoring ergonomics: the SDK demands a Rust toolchain plus `wasm32-unknown-unknown`, and evidence.md shows SpecForge itself is the only plugin author so far. If AI-agent authors can't carry that toolchain, KEEP_WASM preserves a registry nobody publishes to.
## What would change my mind
A single scripting runtime (not MULTI) with interpreter version pinned in `lock_file.rs` the way `wasm_hash` is today, capability imports matching the handshake contract, and evidence that Rust/wasm tooling blocks AI-agent authorship more than it helps determinism.
