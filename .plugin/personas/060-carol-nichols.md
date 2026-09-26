# Position — Carol Nichols (crates.io co-builder — registry storage/index design)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The registry is already an immutable-binary-artifact system, and that is its strength. `crates/specforge-registry-server/src/storage.rs` commits `{package}/{version}.wasm` atomically (temp+fsync+rename, C8-06 fix); handlers SHA-256-verify downloads against the DB and publishes are signed with client-pinned keys. A compiled blob is bit-identical at every install — R-4 reproducibility for free. Script plugins are source text: reproducibility then hinges on interpreter versions and install-time dependency resolution (pip/npm), precisely the ambient-capability surface R-2 forbids.
2. R-3: wasmtime statically links — proven single-binary today. PyO3 needs system/bundled CPython; mlua/QuickJS vendor C toolchains with per-platform build risk.
3. Cheapest C7-11 convergence: keep one runtime, delete the native mirrors (`crates/specforge-emitter/src/builtins/`), route `NativeCustomRules` through the existing `validate__*` contract. Switching runtimes rewrites 4 guests + SDK + wire protocol and still leaves every C7 fix undone.
4. Authoring friction (Rust + wasm32 target) is real, but the primary author is AI agents — strong Rust producers — and crates.io's lesson applies: fix ergonomics at the SDK/macros layer, never by weakening the artifact trust chain.

## Biggest risk in my verdict
Unfixed audit debt (C7-04 fs allow-by-default, C7-10 unenforced timeouts, C7-08 no warm engines) leaves wasm's sandbox/perf story marketing while paying wasmtime's weight — and third-party authoring may never materialize, starving the registry.

## What would change my mind
Evidence that AI-agent authors cannot reliably land wasm32 guests, or a scripting design achieving equal signed, bit-identical, interpreter-pinned artifacts at materially lower authoring cost.
