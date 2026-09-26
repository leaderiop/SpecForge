# Position — Austin Wright (JSON Schema author; revival-era spec editor)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Structural beats promised isolation. Wasm satisfies R-2 by construction — memory isolation plus capability imports; nothing is reachable unless the host exports it. Interpreter sandboxes (mlua, PyO3, deno permissions) are policy the host must remember to enable. Evidence §1.3 shows what policy-forgetting costs: C7-04 shipped `file_system_access` allow-by-default. Changing engines moves that failure class from patchable to architectural.
2. The real contract is already JSON. Guests are ~97% generated manifest (evidence §2); the interface is nine `describe_*.json` categories plus naming conventions. C7-03 ("no IDL; drift already happened") is a contract defect, not a runtime defect. Fix: publish the protocol as draft 2020-12 schemas with `$id` addressing, exactly like `schema/specforge.schema.json` (`https://specforge.dev/schema/specforge.json`, emitted via `specforge schema --publish`, `crates/specforge-cli/src/export.rs:103`) — versioned like the draft lineage, CI-checked like `extension_json_sync`. Any runtime carries that; write it once, now.
3. R-3/R-6: wasmtime is statically linked and deterministic for snapshots; PyO3 needs ambient Python, and pip is ambient-capability — fatal under both.

## Biggest risk in my verdict
KEEP_WASM preserves C7-11 (native mirrors + embedded blobs + SDK) and C7-06's false "one runtime" claim if convergence stalls — it fails if the triad survives.

## What would change my mind
A published IDL plus proof that `@specforge/formal`'s four passes are the last logic plugins need — then the runtime collapses toward declarative data and Lua's per-call cost wins.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Wasm is the only candidate whose sandbox is structural rather than host-enforced policy, and the protocol's real fix is a versioned, `$id`-addressed schema contract — not a new engine.
