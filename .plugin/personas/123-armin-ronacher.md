# Position — Armin Ronacher (insta creator; core-plus-extensions DX)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. R-2 cannot be retrofitted. Wasmtime gives memory isolation plus capability-scoped imports; mlua is interpreter-level policy with a LuaJIT escape history, deno_core drags V8 into R-3's single binary, PyO3 fails R-2 and R-3 outright. MULTI just duplicates whichever sandbox you pick and doubles the audit surface.

2. The open C7 findings — EnginePool a ledger, `max_execution_ms` unenforced, fs allow-by-default (evidence §1.3) — are implementation debt in `crates/specforge-wasm/src/`, not model failure. And the measured ~97%-manifest / ~3%-logic guest split means the authored surface is SDK macros + describe JSON, not wasm code; swapping runtimes buys almost zero ergonomics.

3. R-6 is my home turf: determinism. The `extension_json_sync` / `builtin_blob_sync` guard tests in `crates/specforge-extism` plus insta goldens are exactly how you keep a sandboxed runtime honest, and signed, sha256-verified blobs (registry, evidence §1.4) are more reproducible than source scripts whose behavior rides on the interpreter's version and ambient capabilities.

## Biggest risk in my verdict

Wasmtime's dependency weight (485→511 locked deps) plus the iteration tax — every plugin edit needs a Rust→wasm32 compile — may starve the third-party, AI-agent authoring bet the registry exists for.

## What would change my mind

Measured proof that mlua (Lua 5.4, no JIT) matches capability-scoped sandboxing and ports `formal`'s 478-line passes cleanly, while timing data shows wasm compile latency genuinely suppressing agent-authored contributions.
