# Position — John MacFarlane (markup semantics; pandoc/CommonMark lead)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. One parse, one canonical form. Evidence §1.1 + C7-06/C7-11: the same contributions exist three ways — native mirrors in `crates/specforge-emitter/src/builtins/*.rs`, vendored wasm blobs, SDK crates — held together only by guard tests. Semantics depending on rendering is exactly the ambiguity CommonMark exists to kill. Converging on the compiled artifact as the single canonical form is the fix; MULTI institutionalizes the ambiguity.
2. The workload already separates data from logic, like djot's block/inline. Guests are ~97% generated manifest, ~3% real logic (evidence §2; only `@specforge/formal`'s 478-line guest computes). Manifests are typed graph data — host-executed, snapshot-testable (R-6). The small logic core needs hard isolation: only wasm gives memory isolation by default (R-2). C7-04's allow-by-default is a configuration bug, not a model flaw. PyO3 embeds with no sandbox by default; Lua/V8 lack comparable guarantees.
3. Blobs are reproducible, byte-verifiable, signed artifacts (R-4); wasmtime statically links (R-3). PyO3 embedding is notoriously fragile (evidence §4); a `.lua` source is not a verifiable build artifact.

## Biggest risk in my verdict
Authoring barrier: the SDK demands Rust + `wasm32-unknown-unknown`, the highest-friction option. With the project itself as sole plugin author (evidence §1.5), KEEP_WASM may starve the third-party registry bet.

## What would change my mind
Formalize the IDL and the manifest/logic split first (fixes C7-03), then demonstrated third-party demand for hot-iterated logic Rust's loop genuinely blocks — a capability-scoped scripting tier behind the same protocol, not a replacement.
