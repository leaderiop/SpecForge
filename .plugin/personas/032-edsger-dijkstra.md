# Position — Edsger W. Dijkstra (algorithms & separation of concerns)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments

1. The wasm boundary makes core/extension separation mechanical, not disciplinary: SpecForge's core is zero-domain-knowledge, all vocabulary crosses `call_export` as data. An embedded interpreter shares the host address space; its "sandbox" is interpreter flags, a convention bugs revisit. Wasmtime isolation with capability imports is EWD447's separation of concerns, enforced by construction.

2. The audit indicts discipline inside the model, not the model. C7-11/C7-06: three parallel representations — native mirrors (`crates/specforge-emitter/src/builtins/formal.rs`), vendored blobs (`crates/specforge-extism/src/builtins.rs`), SDK crates. R-1 demands one mechanism: destroy duplication, don't replace the only sound boundary. C7-04 (fs allow-by-default) and C7-10 (max_execution_ms ignored) are configuration defects: deny-by-default, wasmtime epoch/fuel.

3. The workload favors compiled purity. Guests are ~97% generated manifest, ~3% logic; the sole logic-bearing guest, `extensions/formal` (478 lines, four `#[compiler_pass]`), maps a graph snapshot to diagnostics — pure, deterministic (R-6). `builtin_blob_sync` byte-verifies artifacts (R-4); wasmtime is static (R-3), where embedded CPython notoriously fails.

## Biggest risk in my verdict

"Fix inside the model" never happening: with C7-10 unenforced and C7-04 allow-by-default, a hostile plugin can hang the host and read files — isolation exists, guards vaporware. C7-03's missing IDL leaves boundary correctness unprovable.

## What would change my mind

Proof that AI-agent authors cannot tolerate the Rust→wasm compile loop under R-5 hot reload, plus a Lua sandbox demonstrably deny-by-default (Luau quotas, no ambient stdlib) — then LUA's economy outweighs wasm's isolation. Never MULTI: C7-11 already prices multiplied representations.
