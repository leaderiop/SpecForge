# Position — Alberto Brandolini (Event Storming & ubiquitous language)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. The plugin system's real language is already declarative: evidence.md measures guests at ~97% generated manifest, ~3% logic. The nine `describe_*.json` categories are the orange stickies — the durable wall. Switching runtimes optimizes the 3% while fragmenting the vocabulary the 97% speak. Keep one wall.
2. C7-11 (three parallel implementations: `crates/specforge-emitter/src/builtins/*.rs` mirrors, vendored blobs, SDK crates) and C7-06 ("wasm is the only runtime" false three ways) are the classic "photo of a wall" failure: the model claims one mechanism, reality diverged, and guard tests (`extension_json_sync`) now babysit the drift. That is a modeling-discipline bug, not a wasm bug. Converge by deleting the mirrors and the native `NativeCustomRules` bypass (evidence §1.2), not by re-platforming.
3. The only logic-bearing guest — `@specforge/formal`'s 4 compiler passes — is pure analysis over a graph snapshot: deterministic (R-6), sandboxable (R-2), dependency-free. That is wasm's sweet spot; no ecosystem pull.

## Biggest risk in my verdict
Participation. The SDK demands a Rust toolchain plus `wasm32-unknown-unknown` (evidence §1.5); Event Storming lives on low barriers. If the registry bet lands and third-party collectors need real parsers or network access, wasm's no-library reality starves the 3% — the wall stops being co-authored.

## What would change my mind
Evidence that logic-heavy plugins become the norm: collectors hand-rolling parsers inside guests, or wasmtime's weight and AOT gaps (C7-02, C7-08) permanently breaking R-3. Then one capability-scoped scripting tier behind the same manifest protocol — MULTI.
