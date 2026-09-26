# Position — Omar Khattab (declarative LM-pipeline optimization, DSPy)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments

1. The decision is smaller than it looks: evidence.md §2 shows the guest payload is ~97% generated manifest, ~3% logic — only `@specforge/formal` (478 lines, four `#[compiler_pass]` functions) carries real code. DSPy's lesson applies: the declarative layer (describe manifests, Kind/Field/Edge registries, host-executed rules via `NativeCustomRules` in `crates/specforge-emitter/src/compile.rs`) is the real program; deepen it rather than re-platform the tiny imperative kernel.

2. Wasm uniquely satisfies R-2–R-6 together: wasmtime memory isolation with capability imports (strictly stronger than interpreter-level default-deny), static single binary with vendored blobs (`extensions/*/wasm/`), sha256-verified reproducible artifacts for the signed registry (§1.4), and determinism already test-enforced by `builtin_blob_sync`; 3,070 workspace tests pass.

3. The audit gaps are implementation debt, not model failure: C7-02/C7-08 (AOT byte-copy, ledger engine pool) are fixable with real wasmtime AOT plus pooling; C7-04 is a posture flip. C7-03 (no IDL) survives every runtime swap — only an IDL fixes the JSON-marshaling drift.

## Biggest risk in my verdict
Authoring ergonomics: a Rust+wasm32 toolchain is the worst authoring surface for AI-agent plugin authors; if third parties never materialize (§1.5: builtins are the only plugins today), the registry bet dies regardless of sandbox strength.

## What would change my mind
Profiling showing per-call compile (C7-02/C7-08 unfixed) leaves analyze latency-bound over 1.7k-entity graphs, or a manifest-only authoring path covering ~90% of plugin needs — then a vendored-C interpreter (quickjs/Lua 5.4) behind one protocol wins on ergonomics at acceptable sandbox cost.
