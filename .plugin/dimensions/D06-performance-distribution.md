# D06 — Performance, Startup & Distribution

Analyst: 056 Matt Klein (Envoy). Lens: operational performance — what the runtime costs
at process start, per call, and per megabyte shipped. Everything below is measured or
code-cited; estimates are marked.

## 1. What compilation actually costs today

The host JIT-compiles every plugin on every process start. `ExtismRuntime::instantiate`
(`crates/specforge-extism/src/runtime.rs:80-97`) builds a fresh `PluginBuilder` per
module; extism 1.30 creates a new `wasmtime::Engine` per Plugin build (extism
`src/plugin.rs:74`, `Engine::new(&config)`). The four builtin blobs are 324–415 KB each
(`extensions/*/wasm/`, 1.4 MB total). Cranelift compiles on the order of tens of MB/s
on Apple Silicon, so each blob costs roughly **10–30 ms to JIT** [ESTIMATE] — call it
**~40–120 ms of pure compilation plus 4 Engine setups on every `specforge` CLI
invocation, every LSP start, and every watch-mode reload**. There is no amortization:
builtins load via `load_module_bytes` (`builtins.rs:34`), which always instantiates.

That penalty is real but modest against the workload: analyze over ~1.7k-entity graphs
dominates any reasonable per-plugin work, and a per-call compile *never happens* —
compilation is per-load, calls go through the already-compiled module.

## 2. C7-02 is worse than "a byte-copy": the excuse is false for the pinned version

`crates/specforge-wasm/src/cache.rs:16-19` claims true AOT is "deferred until the
Extism runtime exposes a compile-to-native API." **The pinned extism 1.30.0 exposes
exactly that**: `PluginBuilder::compile() -> CompiledPlugin` (plugin_builder.rs:203)
with `Plugin::new_from_compiled(&CompiledPlugin)` (plugin.rs:448) for in-process
compile-once/instantiate-many, and `PluginBuilder::with_cache_config(dir)`
(plugin_builder.rs:156) wiring wasmtime's on-disk code cache for cross-process reuse
(`wasmtime-internal-cache` is already in the lock graph). Meanwhile the cache layer is
decorative end to end: `with_aot_cache_dir` has zero production callers (tests only),
`instantiate` discards `_aot_cache_path` (runtime.rs:84), and even the lifecycle
cache-hit path (`specforge-wasm/src/lifecycle.rs:74-81`) hands the `.aot` path to a
parameter that is thrown away. A cache-hit costs one extra `exists()` stat and then
recompiles anyway.

So my dimension's headline: **the wasm startup tax is self-inflicted and one builder
call away from largely disappearing.** Any verdict that switches runtimes to escape a
cost that the pinned dependency already lets us delete is deciding on a bug.

## 3. Engine pooling (C7-08): a ledger where an engine should be

`crates/specforge-wasm/src/engine_pool.rs` is a `VecDeque<WarmInstance>` of two plain
strings — no `Plugin`, no `Engine`, no instances; "warm" is bookkeeping. Two concrete
consequences:

1. **Four Engines per process**, each with its own code cache and runtime setup, where
   one shared Engine would amortize all of it. extism's own `Pool`/`PoolBuilder`
   (`src/pool.rs`) exists unused.
2. **`call_export` serializes the entire process** — the `Mutex<HashMap>` in
   `ExtismRuntime` (runtime.rs:118-127) is held for the whole `plugin.call`, so a
   `__pass_layering_verify` call blocks a concurrent collector call even on different
   plugins and threads (LSP/MCP hit this first). Per-plugin locks or extism's Pool fix
   the ceiling without touching the protocol.

## 4. Binary size and R-3 distribution

No release build exists here (debug `target/debug/specforge` is 50 MB — not
representative), so sizes are reasoned from the dependency graph: wasmtime 43.0.2
drags **42 wasmtime/cranelift-family crates** with full machine-codegen for both
targets; the audit records the lock moving 485→511 deps. Consistent with the brief's
figures, wasmtime adds **~20–35 MB** to a release binary [ESTIMATE]. Vendored guest
blobs add 1.4 MB — noise.

Against the field:

| Runtime | Binary delta | R-3 (single static binary) | Cold start |
| --- | --- | --- | --- |
| wasmtime (current) | ~30 MB | clean (static, macOS arm64 + Linux x64 tier-1) | JIT ~40–120 ms today; near-zero after §2 fix |
| mlua (Lua 5.4/LuaJIT) | ~1 MB | clean (vendored C) | state creation µs–ms; parse trivial |
| quickjs | ~2 MB | clean (vendored C) | context ~1 ms |
| deno_core (V8) | ~30 MB+ | clean but heavy | isolate ~30–80 ms |
| PyO3 (CPython) | system python **fails R-3**; bundled adds ~30–50 MB | fails/painful | interpreter init 15–50 ms + GIL |

Two readings of that table. The narrow one: wasmtime weighs as much as V8 and 30× Lua
— if download size were the binding constraint, LUA wins my dimension outright. The
operational one: **every candidate that passes R-3 statically does so equally well**;
wasmtime's extra megabytes tax the download, not the runtime contract, while PYTHON
is disqualified on R-3 grounds before performance is even discussed. R-4 is
runtime-agnostic — wasm bytes hash and sign like any other artifact.

## 5. Per-call overhead and determinism (R-6)

Wasm's linear-memory boundary is the *cheapest* marshaling of any candidate: one
memcpy in/out of guest memory. Lua/V8 marshaling of a 1.7k-entity graph means building
interpreter objects node by node — strictly more CPU than the copy. Execution speed:
Cranelift output runs ~1–2× native; plain Lua ~30–70× slower, QuickJS similar —
irrelevant at current graph sizes, but wasm carries headroom the interpreters don't.
Determinism (R-6) is unaffected by the choice here, but note `with_fuel_limit`
(plugin_builder.rs:168) and `with_wasmtime_config` (epoch interruption) make C7-10's
unenforced `max_execution_ms` a one-line fix *inside the current stack* — the timeout
gap is not an argument for switching runtimes either.

## 6. Hot reload (R-5)

Watch-mode reload re-instantiates edited plugins: one JIT compile per change
(~10–30 ms [ESTIMATE]) — imperceptible. The true reload latency for the builtins is
`cargo build --target wasm32-unknown-unknown` (seconds), which is a property of the
*compiled-guest authoring model*, not of wasmtime; D02/D11 own that tradeoff. A
scripting tier would parse in milliseconds but buys it by re-platforming four guests.

## 7. Verdict

The performance story decomposes into one real cost (30 MB binary) and three bugs
(no compile reuse despite APIs present in the pinned extism; a ledger posing as a
pool; no fuel/epoch deadline despite the builder exposing both). All three bugs are
fixable inside KEEP_WASM in roughly a day, and none of the alternates that fit R-3
beats wasm on per-call overhead, marshaling cost, or execution headroom. V8 matches
wasmtime's weight; Python fails R-3; Lua's 29 MB saving does not justify abandoning
the guest SDK, the signed-blob pipeline, and the memory-isolation sandbox. Operate
the fast path you already have: wire `with_cache_config`, adopt
`CompiledPlugin`/`new_from_compiled`, replace the ledger with extism's Pool, enforce
the deadline via fuel or epoch.

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The measured startup/JIT costs are real but self-inflicted and
fixable with APIs already present in pinned extism 1.30 (compile(), with_cache_config,
with_fuel_limit), while the only irreducible cost — ~30 MB binary weight — is matched
by V8, dwarfed by Python's R-3 failure, and not worth re-platforming the runtime to save.
