# Position — Esteban Küber (rustc diagnostics lead)
**Verdict:** KEEP_WASM
**Confidence:** 4
## Arguments
1. The runtime is the plugin's error channel, and traps beat backtraces. A wasm trap (`WasmCallResult::Trap`) is a crisp boundary: the guest died cleanly, no partial host mutation. A Lua/Python/JS failure surfaces an interpreter backtrace after arbitrary host mutation, with no span to attach — what structured diagnostics prevent.
2. R-6 is a diagnostics requirement: snapshot-testable analyze output demands byte-reproducible plugin diagnostics. Wasm is deterministic; CPython (PYTHONHASHSEED, ambient globals) and JS engines (timers, host objects) inject nondeterminism into the one output agents consume.
3. The real gap is typed diagnostics, not engine choice. `crates/specforge-emitter/src/diagnostic_fmt.rs` renders `suggestion: Option<&str>` — flat, no applicability, no labeled secondary spans — and C7-03's stringly `call_export` JSON already drifted. Typed diagnostic payloads (E/W/I code, spans, applicability, did-you-mean as in `crates/specforge-validator/src/file_ref.rs`) fix what engine-swapping cannot; C7-04's fs allow-by-default is likewise an in-model fix.
## Biggest risk in my verdict
KEEP_WASM preserves the Rust-toolchain authoring wall. AI agents are the primary authors, and formal's 478 lines are the sole real logic (~3% of guest payload), so build friction may suppress the third-party authoring the registry bets on; C7-11's three parallel implementations also persist if "fix the gaps" licenses deferral.
## What would change my mind
Evidence that agent-authors reliably fail wasm guest builds while succeeding at script-tier plugins; or a measured latency/binary-size win from mlua/quickjs preserving R-2 capability gating and R-6 determinism.
