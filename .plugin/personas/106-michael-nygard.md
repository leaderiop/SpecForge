# Position — Michael Nygard (ADR methodology; production stability)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. ADR discipline: `wasm_extism_extension_runtime` (`spec/governance/decisions.spec:171`, accepted 2026-03-03) already evaluated embedded scripting — Lua, Rhai, Starlark — and subprocess JSON-RPC, rejecting both on surface-uniformity grounds that still hold. Reversing requires new facts, not re-audited old ones.
2. The audit record shows *consequence drift*, not decision failure: the ADR promises "AOT compilation caches" and "warm engine instances", while evidence.md documents C7-02 (AOT cache is a byte-copy) and C7-08 (EnginePool is a ledger). Correct ADR practice: fix the implementation or supersede individual consequences — never re-litigate the whole runtime because promises went unkept.
3. R-2 + R-3 + R-4 jointly eliminate every alternative: wasm memory isolation is a true bulkhead, the host stays a static single binary, sha256-pinned blobs reproduce exactly; evidence.md's candidate table scores Python "none by default" sandboxing plus system deps — failing two requirements simultaneously.
4. Convergence (C7-11) is runtime-agnostic: with 629 guest LOC (~97% generated manifest), collapsing the three parallel implementations is owed under any verdict, so it justifies no switch.

## Biggest risk in my verdict
"Keep" read as "change nothing": C7-04 (fs allow-by-default) and C7-10 (`max_execution_ms` never enforced) are live stability holes; the real failure mode is shipping churn while the sandbox stays open.

## What would change my mind
A measured superseding ADR proving a statically embedded, deny-by-default Lua (mlua) tier meets R-2 and R-3 where CPython cannot — or evidence that wasmtime warm pooling is unfixable on LSP hot paths.
