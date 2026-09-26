# Position — Eric Evans (DDD; ubiquitous language & bounded contexts)

**Verdict:** KEEP_WASM
**Confidence:** 3

## Arguments
1. The host/guest boundary is a bounded-context contract that wasm enforces mechanically: everything crossing is data (`__handshake`/`__describe`, 9 manifest categories; evidence §1.1). The core's `zero_domain_knowledge_core` invariant — named in `spec/extensions.spec` — survives only if extension logic cannot smuggle imperative context past the handshake. An embedded interpreter (Lua/Python/TS) dissolves the published language into host-API calls.
2. The manifest is the ubiquitous language and dominates: ~97% of guest payload is generated manifest, ~3% logic (evidence §2). KEEP_WASM preserves that priority — declare kinds, edges, fields, terms; logic is the exception. A scripting runtime invites the inverse: vocabulary buried in code.
3. C7-11's three parallel implementations plus the `extension_json_sync`/`builtin_blob_sync` guard tests are translation debt — one model in three dialects, drifting until guards were built. MULTI institutionalizes dialects; R-1 demands convergence. Fix the wasm model, don't multiply it.
4. R-2 is a context map between untrusting parties. Wasm's isolation plus capability imports is the only real no-ambient-access story (evidence §4); Python's pip ecosystem is ambient-capability.

## Biggest risk in my verdict
Rust-only authoring taxes the actual audience: AI agents and non-Rust humans. `@specforge/formal`'s ~478 logic lines show logic will grow, and proc-macro authoring will chafe.

## What would change my mind
A formal IDL fixing C7-03 plus logic-dominated plugin payloads — then TypeScript (deno_core permissions) carrying a byte-identical describe contract could win. Or proof Lua hosts the unchanged manifest without becoming a second dialect.
