# Position — Leonardo de Moura (SMT/proof-systems engineer)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Determinism is the contract.** R-6 demands snapshot-testable analyze output; a verified rev of the formal passes means something only if re-execution is bit-stable. A compiled wasm blob plus a pinned wasmtime engine gives pure-function semantics: same graph in, byte-identical diagnostics out. Scripting runtimes pin *interpreter version* instead of *artifact*, and semantics drift between releases — a snapshot suite silently rots.
2. **The formal workload is thin glue over host machinery.** evidence.md: guests are ~97% manifest, ~3% logic; the only logic-bearing guest is `extensions/formal/` (~478 lines). When `analyze --prove` lands, condition_check becomes SMT-LIB emission — text transformation in Rust, solver in host. No plugin needs Python's ecosystem; Lua/TS buy nothing this workload uses.
3. **Termination and isolation are structural.** Wasm traps give total, enforceable fuel/epoch preemption — C7-10's unenforced `max_execution_ms` is one host wire-up from real. LuaJIT/CPython cannot safely kill runaway plugin code; PyO3's zero sandbox and ambient pip capabilities fail R-2 outright; a Python sidecar breaks R-3. C7-04 is a capability-wiring bug in the host layer — fix the elaborator, don't replace the kernel.

## Biggest risk in my verdict
Authoring friction: Rust plus `wasm32-unknown-unknown` is a heavy toolchain for the AI-agent/human authors the registry bets on, and wasm debugging is opaque.

## What would change my mind
Evidence third parties cannot carry the toolchain, or plugins needing rich external-format libraries (collectors parsing arbitrary test formats) that capability-scoped scripting sandboxes serve safely.
