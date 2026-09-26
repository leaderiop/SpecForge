# Position — Alice Ryhl (Tokio core maintainer; async runtime performance)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Execution-limit enforcement is only mechanically real under wasm. C7-10: `max_execution_ms = 30_000` is never enforced, so a runaway guest stalls a host thread indefinitely. Wasmtime provides epoch interruption/fuel — a hard, preemptible bound; CPython cannot preempt the GIL holder, and Lua debug-hook limits are advisory-grade. My rule (ryhl.io, *Async: What is blocking?*): what you cannot preempt eventually blocks your reactor.
2. R-2's boundary exists only here. C7-04 — sandbox allow-by-default for `file_system_access` — is a config bug inside a real isolation model (memory isolation + capability imports). Interpreter sandboxes (mlua, QuickJS) are historically escapable; PyO3 documents no sandbox at all.
3. Measured perf failures are host-side, not runtime-side. C7-08 (EnginePool is a ledger, no warm instances) and C14-03/C14-04 (blocking walkdir/rusqlite inline in async) fix via warm pooling + `spawn_blocking` — what my anchors `specforge-watch/src/debounce.rs` and `specforge-lsp/src/backend.rs` need for predictable latency. Switching languages fixes none of them.
4. evidence.md: guests are ~97% generated manifest, ~3% logic — the ergonomics case for scripting is thin, and Rust guests deliver R-4/R-6 (signed reproducible blobs, deterministic output) for free.

## Biggest risk in my verdict
KEEP_WASM without landing the C7-03/C7-08/C7-10 fixes re-commits to vaporware — "stay" must mean repair — and AI-agent authors still shoulder the Rust+wasm32 toolchain burden.

## What would change my mind
Proof that fuel/epoch cannot bound the `@specforge/formal` guest's four passes, or that collectors need ambient fs/network the capability imports cannot scope.
