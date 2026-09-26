# Position — Harlan Mills (Cleanroom; correctness by specification)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. SpecForge already inverted debugging the way Cleanroom inverted programs: the declarative manifest (9 `describe_*.json` categories) is the specification, and evidence.md shows the wasm guest payload is ~97% generated manifest, ~3% real logic. The correctness argument lives in `specforge-validator`'s declarative validation engine, not runtime code; the runtime carries only a tiny, isolated escape hatch.
2. R-2 and R-6 are certification requirements. Wasm memory isolation is the only candidate where sandboxing is a structural property rather than interpreter convention; deterministic snapshot output is already certified — 3,070 passing tests include wasm protocol round-trips. Switching discards that certification evidence.

3. The C7 findings are defects against the specified intended function, not refutations of the model: C7-04 (allow-by-default fs) and C7-10 (unenforced `max_execution_ms`) are increments to fix; C7-03's missing IDL is the black-box specification Mills demands first. `NativeCustomRules` in `crates/specforge-emitter/src/compile.rs` is the real R-1 violation; converge it onto the guest path under one sandbox.
## Biggest risk in my verdict
If AI-agent authors push real logic past the current 3%, the Rust/wasm SDK raises authoring friction while wasmtime keeps its heavy dependency — C6-11 showed the wasm path was never truly exercised, so certification claims outrun coverage.
## What would change my mind
A specified IDL (fixing C7-03) plus a sandboxed mlua port of the 4 formal passes re-certifying byte-identical snapshot output under the existing suite; switch only as an increment specified and certified before cutover.
