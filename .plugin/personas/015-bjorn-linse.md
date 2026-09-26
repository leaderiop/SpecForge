# Position — Björn Linse (Neovim core; tree-sitter & incremental reparse)

**Verdict:** LUA
**Confidence:** 3

## Arguments
1. Hot reload is the loop I optimize: routing R-5 through a Rust→wasm toolchain rebuilds per edit — exactly what incremental parsing exists to eliminate (crates/specforge-watch debounce.rs, delta.rs). With Lua the file is the artifact; reload is in-process re-instantiation. C7-08 shows wasmtime pays per-call instantiation anyway (EnginePool is a ledger); an embedded VM makes that cost the design, not a bug. AI-agent authors get sub-second edit→analyze.
2. Evidence §2: guests are ~97% generated manifest, ~3% logic — `@specforge/formal`'s 478 lines is the only real payload. The expensive artifact is the host: `crates/specforge-wasm` ≈9.4k LOC plus wasmtime's dep growth 485→511 locked deps. Port 629 guest lines to Lua tables/functions; delete a runtime. mlua's vendored C keeps R-3's single binary on macOS arm64/Linux x64, no system packages.
3. Determinism and enforcement (R-6, C7-10): one pinned interpreter inside the binary gives identical semantics cross-platform; debug/instruction hooks are an enforceable CPU budget — what `max_execution_ms` never was — and mlua errors return as recoverable Rust Results, so a bad plugin cannot kill the watch daemon mid-debounce.

## Biggest risk in my verdict
R-2. mlua sandboxing (stripped `io`/`os`/`ffi`, memory caps) is best-effort interpreter policy, not memory isolation; wasm linear memory plus capability imports is categorically stronger against a hostile registry plugin exploiting an interpreter bug.

## What would change my mind
Real third-party volume with genuinely hostile plugins, or a sandbox-proven countable VM (Luau) whose authoring story matches AI agents.

## Verdict

**Verdict:** LUA
**Confidence:** 3
**One-line rationale:** Sub-second hot-reload loop and a trivial migration (guests are 97% manifest) beat wasmtime's weight, if mlua's interpreter sandbox is hardened for R-2.
