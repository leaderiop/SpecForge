# Position — Benjamin Eckel (Extism co-author; PDKs, manifests & typed conversion)

**Verdict:** KEEP_WASM

**Confidence:** 4

## Arguments

1. R-1 is the wasm bet. `crates/specforge-wasm/src/manifest_bridge.rs` validates `ManifestV2` identically for builtins and third parties — schema, peer dependencies, structural-keyword collisions. C7-11's "three parallel implementations" is migration debt, not a runtime defect: delete the native mirrors in `crates/specforge-emitter/src/builtins/`, keep the sync tests, done. A scripting runtime re-creates that duplication or collapses builtins into host code, violating R-1.
2. Sandbox: wasm memory isolation is structural; C7-04's `file_system_access` allow-by-default is a policy bug in our bridge, fixable in-model. Lua offers interpreter conventions, not capability imports; Python fails R-3 outright (system interpreter); V8 bloats the binary.
3. Distribution: vendored 324–415 KB blobs under `extensions/*/wasm/` prove R-3/R-4 end-to-end — static binary, sha256-verified registry blobs, no user toolchain.
4. Workload fit: evidence shows guests are ~97% generated manifest, ~3% logic. `docs/extension-sdk.md`'s macros already generate descriptors from declarative Rust — the ergonomic win is XTP-Bindgen-style schema→descriptor codegen for the 11 categories, not a language swap. AI agents author Rust fine.

## Biggest risk in my verdict

C7-02/C7-08: the "AOT cache" is a byte-copy and EnginePool keeps no warm instances — per-call instantiation may make analyze slower than mlua on 1.7k-entity snapshots, and I'm defending an unmeasured performance story.

## What would change my mind

Real third-party registry authors who aren't Rust/AI-agent-shaped, plus measurements showing instantiation dominates analyze latency after fixing C7-04/C7-10 — then MULTI (a Lua tier behind `call_export` with extism-convert-style typed marshalling) beats KEEP_WASM.
