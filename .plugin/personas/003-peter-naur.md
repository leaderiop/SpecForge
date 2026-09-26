# Position — Peter Naur (theory-building; ALGOL 60 report editor)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The ALGOL 60 report proved independent implementations agree only through an exactly defined shared artifact. SpecForge's defect is not the metal but the missing "language report": no IDL, stringly-typed `call_export` JSON, drift already occurred (evidence §1.1, C7-03). Swapping runtimes restarts the agreement problem; an exact protocol definition fixes it within the current model — and wasm isolation/determinism serve R-2 and R-6 structurally.
2. Theory building: the builtins' theory is already externalized as declarative manifest, not imperative code — guests are ~97% generated manifest, ~3% logic across 629 lib.rs lines (evidence §2). That is README.md's thesis ("Intent is trapped in prose"; "The graph is the product") applied to plugins. A scripting runtime invites logic where declarative data suffices — regression toward prose carriers agents must re-interpret.
3. R-1 is the ALGOL ideal: one definition, no privileged implementation. The tree violates it three ways (C7-11): native mirrors in `crates/specforge-emitter/src/builtins/`, vendored blobs, SDK crates — synchronized only by the `extension_json_sync` guard test, tribal knowledge in code form. README.md even documents a "composite runtime dispatching builtin-first" — the first-party tier R-1 forbids. Converge to one wasm mechanism.

## Biggest risk in my verdict

The Rust + wasm32 toolchain is an authoring barrier (evidence §1.5). If north-star.md's H2 ecosystem — twenty-plus community extensions, first-party a minority — never materializes because authoring is too hard, the theory-sharing thesis dies regardless of runtime purity.

## What would change my mind

Measured proof that AI agents cannot author wasm plugins at ecosystem rate even after C7-03 closes with a real IDL — then a scripting tier behind that same exact protocol (MULTI) is defensible: the report, not the metal, carries agreement.
