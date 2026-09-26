# Position — Rebecca Stambler (gopls lead; LSP/incremental-diagnostics)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Diagnostics must be pure functions of a snapshot.** gopls's core lesson: no hover or diagnostic is trustworthy until it derives from one coherent, immutable view. `crates/specforge-lsp/src/state.rs` embodies this — `LspState` publishes per-URI `Diagnostic`s from an `IncrementalPipeline` over the graph. R-6 (deterministic, snapshot-testable output) is the same requirement. Wasm guests are pure functions over the graph snapshot; an embedded CPython/Lua interpreter drags ambient state (loaded modules, globals, fs) across the snapshot boundary, and PyO3's GIL couples plugin crashes to the host process serving LSP diagnostics.
2. **Consolidate the zoo, don't grow it.** gopls replaced guru/godef/gocode with one server; evidence.md C7-11 shows the same zoo recurring — three parallel implementations, plus `NativeCustomRules` host dispatch bypassing guest `validate__*` (evidence §1.2), which already violates R-1. MULTI institutionalizes fragmentation; KEEP_WASM means deleting `crates/specforge-emitter/src/builtins/*.rs` and converging on one guest path.
3. **The regression harness already exists.** Among 3,070 passing tests are wasm protocol round-trip suites (evidence §2); `extension_json_sync`/`builtin_blob_sync` pin manifests. Switching runtimes discards this harness for unproven equivalents. Guests are stateless, so R-5 reload = fresh instantiation — trivially correct, no stale interpreter globals.

## Biggest risk in my verdict
Authoring friction: Rust + `wasm32-unknown-unknown` toolchains (evidence §1.5) may suppress the registry's third-party bet, and wasmtime's ~26 locked-dep weight bloats the binary.

## What would change my mind
A mlua/quickjs embedding that passes the existing determinism and round-trip suites with zero system deps — or field evidence that AI agents reliably fail to author Rust guests.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Deterministic snapshot-pure diagnostics, one consolidated runtime path, and a green regression harness beat authoring ergonomics for a payload that is 97% generated manifest today.
