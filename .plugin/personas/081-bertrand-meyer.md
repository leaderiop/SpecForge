# Position — Bertrand Meyer (Design by Contract, formal methods)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. A plugin boundary is a contract; today's has no enforceable terms. C7-03: stringly JSON over `call_export`, no IDL — host/guest obligations are unstated, so violations surface as traps, not diagnostics. Wasm's typed imports/exports plus an IDL is the only model where preconditions (capabilities granted) and postconditions (typed results) are machine-checkable — the property `condition_check` enforces on specs via W036–W040.
2. R-2 is DbC's obligation asymmetry made literal: the callee receives exactly the capabilities the caller grants. Wasm imports give deny-by-default structurally (no import = no capability); C7-04 shows the host violating its own precondition by allow-defaulting `file_system_access`. Lua/QuickJS must police a stdlib surface; CPython cannot sandbox — unverifiable by construction, W037 incarnate.
3. R-1/C7-11 is one contract with three suppliers (native mirrors in `crates/specforge-emitter/src/builtins/`, vendored blobs, SDK crates). The fix is convergence under one mechanism — delete the mirrors, route `validate__*` through guests — not a fourth runtime. The checker itself (`@specforge/formal`'s 478-line condition_check guest) must run where its determinism (R-6) is checkable.

## Biggest risk in my verdict
Enforcement never lands: no IDL, C7-10's `max_execution_ms` stays unenforced — wasmtime remains a heavy dependency (485→511 locked deps) whose stated obligations stay vapor, as C7-02/C7-08 already are.

## What would change my mind
Proof wasmtime cannot enforce obligations here (fuel/epoch limits, capability imports), or evidence AI-agent authors genuinely need a dynamic tier — acceptable only if MULTI keeps the typed boundary for logic-bearing passes.
