# Position — Colin McDonnell (Zod creator; TS schema validation & DX)

**Verdict:** TYPESCRIPT
**Confidence:** 3

## Arguments
1. The disease is C7-03's missing IDL, not the engine. Drift already forced two guard tests (`extension_json_sync`, `builtin_blob_sync`) to police three parallel implementations (C7-11). A TS runtime makes types the schema: one typed manifest contract, parse-don't-validate at the host↔plugin boundary, replacing stringly JSON over `call_export` — and it generates `integrations/vscode/schemas/specforge.schema.json` from that source, not a hand-maintained 636-line file.
2. The workload fits: guests are ~97% declarative manifest, ~3% logic (`@specforge/formal`'s four `#[compiler_pass]` functions). Pure transforms over a graph snapshot are exactly what a permission-scoped JS isolate (deno_core: no fs/net/process unless granted → R-2) does deterministically (R-6), with trivial hot reload (R-5) and static single-binary embed (R-3).
3. Authors are AI agents; the wasm SDK demands a Rust toolchain plus `wasm32-unknown-unknown` — a hostile authoring loop. TS is the surface agents write best, and typed manifests give instant feedback Lua cannot. LUA's dynamism worsens the schema problem; PYTHON fails R-3/R-2 (ambient pip); MULTI entrenches C7-11.

## Biggest risk in my verdict
Swapping wasmtime for deno_core trades one heavy dependency for another and discards the published SDK (crates.io v0.1.0); V8 startup/memory cost per analyze run is unmeasured.

## What would change my mind
An IDL-first fix inside the current runtime — one generated schema driving guests, mirrors, and editor validation — plus profiling showing per-call cost is payload marshaling, not execution. Then the 97%-manifest workload no longer justifies a runtime switch: KEEP_WASM.

## Verdict

**Verdict:** TYPESCRIPT
**Confidence:** 3
**One-line rationale:** The real gap is an untyped plugin contract; a TS runtime collapses schema, authoring surface, sandbox, and hot reload into one typed boundary that AI-agent authors can actually iterate on.
