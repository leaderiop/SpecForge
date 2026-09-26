# Position — Bruno Oliveira (pytest lead maintainer; plugin ecosystem steward)
**Verdict:** LUA
**Confidence:** 3
## Arguments
1. Ecosystem math. The registry is a bet on third-party authors, yet evidence §1.5 shows the only authors today are ourselves, and authoring demands a Rust toolchain plus the `wasm32-unknown-unknown` target. pytest's ecosystem exists because a plugin is plain source discoverable by entry point — no cross-compilation. `.lua` scripts are the closest analog; the guest payload is ~97% generated manifest anyway (evidence §2), and the workload — per-entity rule checks over ~1.7k-entity graphs — is script-shaped.
2. Disproportionate machinery: ~9,350 host LOC in `crates/specforge-wasm/` executes 629 guest lines, and audit C7-02/C7-08/C7-10 show the knobs are vaporware — the engine pool is a ledger, `max_execution_ms` never enforced. mlua's instruction-count hook is simpler and actually enforceable for hermetic R-6 runs.
3. R-4 reproducibility: vendored blobs need guard tests (`builtin_blob_sync`) because three parallel implementations already drifted (C7-03/C7-11). Source-distributed scripts pinned to one interpreter are reproducible by construction; pytester-style testing reduces to feed-JSON, snapshot-diagnostics.
## Biggest risk in my verdict
mlua's sandbox is blacklist-shaped (strip `os`/`io`), weaker than wasm memory isolation; a hostile script or interpreter bug escapes, and porting the 478-line `@specforge/formal` passes to Lua forfeits Rust type safety.
## What would change my mind
A demonstrated mlua sandbox escape under deny-by-default plus instruction budget; proof that plugin logic grows into algorithmic passes where Rust-authored wasm wins; or deno_core's permission model at acceptable binary size with TS-authoring data favoring AI agents.
