# Position — Henry Andrews (JSON Schema vocabulary & modularity lead)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. The binding failure is the contract, not the runtime. `crates/specforge-emitter/src/schema.rs` already ships `SchemaVersion`, `SchemaCompatibility{requested, resolved}`, and `negotiate_version` for exports — a producer/consumer dialect negotiation. The plugin wire got none of that: evidence.md confirms C7-03 (no IDL, stringly JSON over `call_export`) with drift "already happened", patched by guard tests (`extension_json_sync`, `builtin_blob_sync`) byte-comparing mirrors in `crates/specforge-emitter/src/builtins/`. C7-11's three parallel implementations are the pre-2019-09 disease: one contract, three unvalidated copies. Switching interpreters fixes none of this; versioning and validating the protocol dialect does.
2. The workload is ~97% generated manifest, ~3% logic (evidence.md §2) — declarative metadata is a schema-dialect problem. Wasm's isolation + capability imports (R-2), static embedding (R-3), sha256-verified signed-registry blobs (R-4), and determinism (R-6) map directly onto the hard requirements.
3. MULTI is a stealth first-party tier: the vendored `extensions/*/wasm/` builtins would stay wasm while community plugins take the scripting path — betraying R-1. A second runtime is a second dialect consumers must negotiate, multiplying exactly the drift surface that guard tests are papering over.

## Biggest risk in my verdict
KEEP_WASM as status quo cements the stringly-typed contract; my verdict is conditioned on an IDL-first protocol repair (C7-03/C7-11), not on defending today's code.

## What would change my mind
Evidence that AI-agent authors cannot reliably target `wasm32-unknown-unknown` (registry adoption collapses on toolchain friction), or a runtime-agnostic, versioned IDL adopted first — making MULTI a pure adapter problem and voiding my objection.
