# Position — Kent Beck (xUnit & TDD creator)

**Verdict:** LUA
**Confidence:** 3

## Arguments
1. The workload is tiny and the test harness proves the real contract is runtime-neutral: evidence.md says the guest payload is ~97% generated manifest, ~3% real logic (629 lines of lib.rs total), and `integrations/rust/specforge-test/src/report.rs` pins a versioned (`schema_version: "1.0"`), sorted, deterministic JSON report — R-6 satisfied by construction. Any interpreter computing that JSON faithfully passes the test; wasmtime is overkill for it.
2. The red/green loop is the product: R-5 hot reload plus R-6 snapshot diffs means edit-plugin → re-analyze → compare must take seconds. A Lua script reloads by re-reading a file. The wasm fast path is fiction — C7-08: EnginePool is a ledger with no warm instances; C7-02: the "AOT cache" is a byte-copy. Test-first rule: don't keep machinery whose claimed behavior fails its own tests.
3. MULTI recreates C7-11. The `extension_json_sync` and `builtin_blob_sync` guard tests already police three parallel implementations at real cost; a second runtime doubles the sync surface. One runtime, one guard, one report schema.

## Biggest risk in my verdict
Interpreter sandboxing (R-2) is weaker than wasm memory isolation, and LuaJIT's FFI would break it — pin plain Lua 5.4 via vendored `mlua`, no `io`/`os`/ffi, with host-provided capability APIs for collectors' network needs.

## What would change my mind
Measured evidence that `@specforge/formal`'s four passes (~478 lines) need Rust-level throughput over 1.7k-entity graphs, or that capability-scoped host APIs (graph access, network for collectors) cannot be safely mediated in Lua.
