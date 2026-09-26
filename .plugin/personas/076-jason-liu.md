# Position — Jason Liu (structured outputs & typed LLM interfaces)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. The defect is the contract, not the runtime. SpecForge already bets on typed machine contracts — the Graph Protocol JSON Schema (draft 2020-12) is the project's Pydantic model; agents consume typed objects, not prose. Yet the plugin boundary violates that bet: C7-03 is stringly-typed JSON over `call_export`, and evidence.md notes drift "already happened." The fix — versioned, validated schemas whose specific errors feed back to the authoring agent — is runtime-agnostic; Lua/Python abandon typed exports for dynamic-everything.
2. R-2 is only met by memory isolation. Python scores "none by default" with pip as ambient capability; Lua's interpreter sandbox is convention, not enforcement. C7-04 (fs allow-by-default) is a bug inside wasm's capability model — fix the default, don't downgrade to a runtime where sandboxing is opt-in hygiene.
3. R-6 and migration cost: wasm delivers snapshot-stable analyze output today (3,070 passing tests). C7-11 converges inside this model: delete the `crates/specforge-emitter/src/builtins/*.rs` mirrors and the native `NativeCustomRules` dispatch in `compile.rs` once guest `validate__*` is reliable and testable.

## Biggest risk in my verdict
Agent-authoring ergonomics: a Rust + wasm32 toolchain is a heavy iteration loop for AI co-authors. If the registry yields logic-bearing plugins beyond `@specforge/formal`'s ~478-line passes, TypeScript would win on authoring throughput.

## What would change my mind
A deno_core guest with typed (zod-style) manifests and permission-scoped I/O meeting R-2/R-3 budgets — or plugins becoming logic-dominant, making Rust authoring friction the binding constraint.
