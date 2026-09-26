# Position — Till Schneidereit (Bytecode Alliance co-founder; runtime ecosystem positioning)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The BA thesis applies directly: embed a maintained runtime, never become one. wasmtime 43.0.2 (via `crates/specforge-extism`) puts SpecForge's sandbox TCB under Bytecode Alliance scrutiny; memory isolation plus capability imports satisfies R-2 by construction, where Lua needs interpreter-discipline (bytecode/`string.rep` bombs absent Luau), PyO3 is ambient-capability by design (pip, ctypes), and V8 is a second enormous TCB to audit forever.
2. R-3 and R-6 line up: static single binary, deterministic snapshot-testable execution — evidence.md §4 concedes system Python for PyO3, nothing for wasmtime.
3. The audit convicts the implementation, not the model. C7-04 (fs allow-by-default) is one flag flip in `crates/specforge-wasm/src/sandbox.rs`; C7-02/C7-08/C7-10 are debts the thin `WasmRuntime` trait (`crates/specforge-wasm/src/runtime.rs:38-48`, three methods) lets us pay without protocol churn. Swapping runtimes fixes none of them and re-opens C7-11 with interest.

## Biggest risk in my verdict

Evidence.md §2: guests are ~97% generated manifest, ~3% logic, and the only author is this project. If registry adoption needs AI/third-party authors who won't carry a Rust+wasm32 toolchain, KEEP_WASM is a moat, not a foundation — we'd be preserving the exact fiction C7-06 documents ("wasm only" three ways false).

## What would change my mind

Demonstrated authoring friction: if builtin logic collapses to declarative manifests plus a few `validate__*` functions, a capability-scoped scripting tier behind the same `call_export` protocol (mlua, vendored C, R-3-safe) beats maintaining SDK + toolchain. Also: wasmtime weight becoming prohibitive with no slimming path.
