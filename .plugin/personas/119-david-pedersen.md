# Position — David Pedersen (axum creator; extractor-typed handler design)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Sandbox is a layer, not author discipline.** In axum, cross-cutting concerns live in tower layers, not handler bodies. R-2 is that concern: a wasm guest *cannot* touch fs/network/process unless the host imports the capability (evidence.md §4); Lua/Python sandboxing is blocklists with known escapes — CPython via PyO3 is effectively unsandboxable. Wasmtime gives deny-by-default structurally; C7-04 is a default flipped in host code, not a runtime defect.
2. **Plugins are already static artifacts with a working pipeline.** `crates/specforge-registry-server/src/handlers.rs` `publish_package` (temp+fsync+DB-arbitrate+rename) plus sha256-verified downloads passed live e2e (evidence.md §1.4) — R-4 solved for blob-shaped plugins. C7-03's missing IDL is a host-API gap fixable in place; the SDK's `#[specforge_extension]` macros are scaffolding, and swapping runtimes rebuilds SDK, registry install, and cache for zero capability gain.
3. **R-1 converges cheapest here.** Guests are ~97% generated manifest, ~3% logic — 629 lines total (evidence.md §2). Deleting the C7-11 mirrors in `crates/specforge-emitter/src/builtins/` and the host-native `NativeCustomRules` bypass is small; moving four builtins to mlua/PyO3/deno_core rewrites tooling for nothing.

## Biggest risk in my verdict
Wasmtime (485→511 locked deps) is heavy ballast if third parties never author Rust-for-wasm — AI agents fluently write TS/Lua, rarely Rust.

## What would change my mind
Proof that agent-authors are far more productive in TS at this boundary — then TYPESCRIPT (deno_core's permission model is the nearest deny-by-default analog), but only with a typed host API closing C7-03 and enforced limits closing C7-10.
