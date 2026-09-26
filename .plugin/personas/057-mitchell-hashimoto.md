# Position — Mitchell Hashimoto (registry & extension-model engineer)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The artifact channel is the decision: wasm ships one versioned, hash-pinned, signed binary per plugin. `crates/specforge-cli/src/add.rs:132` sha256-verifies the blob, pins the publisher key TOFU-style, and records `wasm_hash` + `key_id` in the lock file — the verify→install→pin chain Terraform's Registry institutionalized. Lua/Python/TS plugins are source plus an ambient package ecosystem; I left Rubygems in 2013 because foreign dependency resolution destroys reproducibility (R-4).
2. Sandbox and distribution are structural: wasmtime static-links with zero system packages (R-3); PyO3 fails R-3; Lua sandboxes are interpreter flags with a long escape history, versus wasm memory isolation plus capability imports (R-2).
3. Audit findings (C7-04 fs allow-by-default, C7-08 ledger-only pool, C7-10 unenforced timeout, C7-03 no IDL) are implementation debt *inside* the wasm model; a runtime swap re-earns every one. R-1 convergence means deleting the `crates/specforge-emitter/src/builtins/` mirrors (C7-11) and routing `NativeCustomRules` through guest `validate__*` exports — not swapping runtimes.

## Biggest risk in my verdict

The registry bets on third-party authors, yet authoring demands a Rust toolchain plus wasm32 target. If SDK weight starves the registry, a pristine artifact channel protects an empty marketplace.

## What would change my mind

Evidence that plugins stay ~97% generated manifest with trivial logic would make 9.4k LOC of wasmtime machinery bad value — capability-scoped embedded Lua with manifests-as-data could satisfy R-2/R-6 cheaper. And if non-Rust wasm32 guest SDKs (TinyGo, AssemblyScript) prove impractical, a scripting tier becomes the only multi-language authoring path, forcing MULTI.
