# Position — Lee Byron (GraphQL co-creator; schema introspection & evolution)

**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. **The schema is the product; the runtime is plumbing.** Evidence.md §2: guests are ~97% generated manifest, ~3% logic. Plugins publish a declarative, introspectable contract — the `describe_*` manifest — the `__schema` precedent: one queryable meta-schema, every consumer (CLI export, LSP, the ~30 tools in `crates/specforge-mcp/src/tools/`) introspecting independently. Wasm is the only language-agnostic substrate, so any authoring language targets it without changing the contract. Lua/TS/Python collapse "many authors, one schema" into "one language, forked ecosystem."
2. **The defect is the contract, not the runtime.** C7-03: no IDL, stringly-typed JSON over `call_export`, drift already happened — the ad-hoc-endpoint disease GraphQL was invented to kill. Swapping interpreters leaves it open; an IDL'd manifest protocol fixes it inside KEEP_WASM.
3. **Evolution demands one tier.** R-1/R-4 mirror deprecate-don't-version: consumers never see a two-class schema. Native mirrors prove the failure mode — C7-11 duplication and C6-11 custom rules firing natively in `crates/specforge-emitter/src/compile.rs`, bypassing guest `validate__*`. A scripting tier re-creates that split by design; MULTI institutionalizes it.
## Biggest risk in my verdict
Authoring friction: Rust + `wasm32-unknown-unknown` toolchains may starve the registry bet — a technically pure but empty ecosystem.
## What would change my mind
If the ~3% logic share inverts (logic-bearing passes/rules dominate) and a fixed IDL (C7-03) still leaves authoring hostile, a TypeScript-to-wasm authoring layer could win — but that amends the authoring surface over wasm, never replaces the runtime.
