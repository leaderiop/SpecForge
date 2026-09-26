# Position — Simon Brown (C4 model creator; diagrams-as-code)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. SpecForge already proves my model/view separation: `crates/specforge-cli/src/model.rs` builds one `ModelIntermediate` from the resolved schema, and `render` in `crates/specforge-emitter/src/model/mod.rs` extracts Markdown/Mermaid/Dot/JSON/DBML views from it. Apply the same discipline to plugins: the *model* is the declarative manifest; logic is a thin extract. Evidence: the guest payload is ~97% generated manifest, ~3% real logic (evidence.md §2). Don't swap foundations for the 3%.
2. R-1 demands convergence, not tiering. C7-06: the "wasm is the only runtime" claim is already false three ways (native mirrors, vendored blobs, SDK). MULTI formalizes that duplication as policy — two runtimes mean two host APIs, two security models, double the audit surface. Structurizr keeps one DSL precisely so views cannot fork from the model.
3. R-2/R-6: wasmtime isolation is structural and execution deterministic; interpreter capability limits are advisory; CPython offers neither by default. With AI agents co-authoring untrusted registry plugins, sandboxing must be architecture, not convention.

## Biggest risk in my verdict

Guest authoring needs a Rust toolchain plus the `wasm32-unknown-unknown` target. If that friction stalls third-party authoring, the registry bet — the product's growth story — fails while KEEP_WASM stays technically correct.

## What would change my mind

Evidence that plugins become logic-dominant rather than manifest-dominant, or that a single Lua tier meets R-2 sandboxing, R-6 determinism, and single-binary R-3 simultaneously — then pick LUA, never MULTI.
