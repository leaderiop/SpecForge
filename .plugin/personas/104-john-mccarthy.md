# Position — John McCarthy (Formal communication languages; shared vocabulary)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. CBCL's premise: independent parties interoperate through a declared vocabulary, never ambient convention. Wasm's import model *is* that vocabulary — fs/network/process appear as imports to grant or deny, exactly R-2's capability scoping. Lua/Python/TS embed runtimes whose side effects are ambient (`io`, pip, npm): undeclared defaults — what CBCL set out to abolish. C7-04 shows even wasm leaks (fs allow-by-default); interpreters start further away.
2. The evidence says the bottleneck is the contract, not the carrier: guests are ~97% generated manifest, ~3% logic (`@specforge/formal`, 478 lines). C7-03 (no IDL, stringly `call_export`, drift already happened) is where interop actually breaks. Fixing the declared vocabulary (`schema/specforge.schema.json` Graph Protocol, the 9 describe manifest categories in `crates/specforge-wasm`) buys more than any runtime swap.
3. R-4/R-6 reproducibility: vendored 324–415 KB blobs are closed artifacts, sha256-pinned in the signed registry, byte-compared by `builtin_blob_sync`. A scripting tier makes the interpreter version an undeclared party — CBCL's nonmonotonic trap — silently breaking analyze snapshot determinism across machines.

## Biggest risk in my verdict
R-1 means every author pays wasm's toll: Rust toolchain + wasm32 target. If third parties never arrive, the registry bet dies on ergonomics.

## What would change my mind
A scripting runtime behind a declared envelope — interpreter version hashed into the registry artifact, capability imports specified as data — plus evidence that plugin logic outgrows manifests and R-5 iteration favors scripts.
