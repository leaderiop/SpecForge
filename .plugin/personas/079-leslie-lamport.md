# Position — Leslie Lamport (Formal methods / TLA+)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. R-2 is a safety property: "no plugin action outside granted capabilities." Wasm enforces it structurally: an ungranted import is absent from the guest's reachable state machine. PyO3 fails it structurally (ambient stdlib, process) and fails R-3; Lua and QuickJS deny only by interpreter hygiene: convention, not mechanism. Specify properties first; pick the runtime where it is a theorem.

2. R-6 demands a deterministic state machine; the wasm path has one: 3,070 passing tests include protocol round-trips, and `@specforge/formal`'s four `#[compiler_pass]` exports (extensions/formal/src/lib.rs) shuffle `after` constraints so the host's toposort defines a unique total order, snapshot-testable by construction. Interpreter determinism (CPython GIL, version drift) must be maintained; wasm's is inherited.

3. Evidence.md §2: guests total 629 lines, ~3% real logic; the runtime is over-provisioned and replacement buys little. Fix violated invariants in place: C7-04 deny-by-default fs, C7-10 enforce max_execution_ms via wasmtime fuel/epoch (liveness: runaway plugins must eventually yield), C7-03 an IDL is the missing protocol spec: write it.

## Biggest risk in my verdict

KEEP_WASM without converging the three parallel implementations (C7-11: native mirrors, vendored blobs, SDK crates) leaves R-1 unsatisfiable: a spec with no valid implementation.

## What would change my mind

A mlua/Luau build enforcing capability denial at interpreter construction (io/os stripped, instruction-count fuel covering C7-10) with equal determinism, plus proof wasmtime's dependency weight (485→511 locked deps) breaks R-3 targets, or C7-08 confirmed unfixable: per-call compile dominating analyze latency makes zero-compile scripting decisive.
