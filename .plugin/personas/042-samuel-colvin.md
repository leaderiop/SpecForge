# Position — Samuel Colvin (Pydantic creator; validation-core architecture)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. SpecForge already implements the pydantic-core split: declarative, host-executed rule patterns (`ValidationPatternKind::FieldConstraint`, `Cycle`, `FileExists`, `ConditionalFieldRequired`… in `crates/specforge-registry/src/compilation/validation_engine.rs`) interpreted by compiled Rust, with `Custom` + `wasm_function` as a narrow escape hatch. Pydantic v2's lesson: keep the vocabulary small and orthogonal; the escape hatch, not the core, needs a runtime. That surface is tiny — evidence.md: guests are ~97% generated manifest, ~3% logic; only `@specforge/formal` (478 lines, 4 passes) bears real logic.
2. R-3 eliminates Python (PyO3 embed-and-ship is fragile, wants a system interpreter) and burdens V8; mlua sandboxes only at interpreter level, no hard memory isolation. Wasmtime alone ships memory isolation + capability imports (R-2), deterministic metering (R-6), and static single-binary linking (R-3).
3. The actual wound is C7-03: no IDL, stringly `call_export`, drift already happened — exactly the failure a compiled-schema core exists to prevent. Swapping runtimes fixes none of it; a typed protocol, enforced `max_execution_ms` (C7-10), deny-by-default fs (C7-04), one implementation (C7-11) do. Fix inside the model.

## Biggest risk in my verdict
Authoring demands a Rust toolchain + `wasm32-unknown-unknown` target; if third-party adoption stalls, we polished a runtime few write for. AI agents as primary authors mitigate — they handle Rust/wasm readily.

## What would change my mind
If real plugin logic grows far past formal's 478 lines while pooling/AOT stays vaporware (C7-02, C7-08) and R-5 hot reload stays slow, a Lua tier behind the typed protocol (MULTI) wins on ergonomics.
