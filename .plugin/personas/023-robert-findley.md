# Position — Robert Findley (gopls co-lead; incremental compilation, LSP, diagnostics UX)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments

1. **One implementation, every plugin equal (R-1).** gopls serves every editor through one server; VS Code is not first-party. So C7-11's three parallel implementations (`crates/specforge-emitter/src/builtins/` native mirrors, vendored wasm blobs, SDK crates) are the disease — and C7-03 shows drift already happened. KEEP_WASM converges inside one mechanism; MULTI institutionalizes drift. The codebase already agrees: `crates/specforge-lsp/src/backend.rs` routes extensions through the same wasm `ProtocolHost` as the CLI, explicitly "so LSP diagnostics match."

2. **R-2 has exactly one credible answer: memory isolation + capability-scoped imports.** C7-04 (`file_system_access` allow-by-default) is a config bug, fixable in a day. mlua/QuickJS sandboxes are stdlib discipline, not isolation — one bytecode-loading or allocation-bomb escape breaks the whole signed registry's trust model. Python via PyO3 fails R-2 and R-3 outright: ambient pip capabilities, system interpreter dependency.

3. **Determinism and watch-loop cost favor wasm once the engineering is finished.** Watch validation dispatches per-delta (`plan_incremental_dispatch`, `ValidatorInput` in `crates/specforge-watch/`), so the no-warm-instance reality (C7-08, C7-02 byte-copy "AOT") hurts — but those are unfinished work *inside* the model, not evidence against it. Wasm execution is deterministic; Lua's unspecified `pairs` order is a snapshot hazard for diagnostics ordering under R-6. The ~97%-manifest guest payload is runtime-agnostic anyway; the decision is only about formal's 478 lines — already written and passing.

## Biggest risk in my verdict

Authoring ergonomics. A Rust toolchain plus `wasm32` target per edit is a slow loop for AI-agent co-authors, and wasmtime's weight (485→511 locked deps) buys ~9.4k host LOC serving four plugins. If third-party authoring never materializes, we paid heavily for it.

## What would change my mind

Measurements showing watch re-analysis cannot meet its latency target even with real pooling/AOT, or a fuzz-hardened mlua sandbox (instruction-count hooks, memory caps, bytecode loading disabled) proving R-2 parity — then Lua's instant edit-run loop wins on the axis I care most about.
