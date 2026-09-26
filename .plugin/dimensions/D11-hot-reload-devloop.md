# D11 — Hot reload & developer loop (R-5)

**R-5 says:** edit plugin → re-run analyze without restarting the host. This dimension asks which runtime makes that loop fast, observable, and testable — for the only plugin authors who exist today (this project, per evidence §1.5) and for the AI agents the product targets.

## What watch does today (grounded)

`specforge watch` (crates/specforge-cli/src/watch.rs) cold-builds once through the standard compile pipeline (`crate::pipeline::compile`, which loads extension manifests), bakes plugin-derived state into `GraphConfig` (installed keywords, bidirectional pairs, body-parser suppressions — watch.rs:25–72), seeds `IncrementalPipeline::from_cold_build` (watch.rs:95–101), and then the loop (watch.rs:153–208) only re-parses `.spec` files and rebuilds the graph. **No wasm call happens in the watch loop.** Custom rules execute natively in the host (`NativeCustomRules`, evidence §1.2), so the loop never needs the runtime — which is exactly why R-5 is unimplemented, not merely buggy.

Three structural facts constrain every candidate:

1. **The pipeline has no plugin seam.** `graph_config` is a private field set only at construction (pipeline.rs:84–104); there is no config-swap API, so even a declarative manifest change (new keyword) cannot enter a running watch session. Any runtime needs: artifact watching, a reload trigger, manifest re-derivation, and a `graph_config` swap path.
2. **The watcher cannot see plugins.** `SpecWatcher` filters events to `ext == "spec"` (watcher.rs:85). Plugin artifacts are invisible.
3. **Debounce reality (C14-09).** Two debounce implementations exist: the tested, configurable `Debouncer` (debounce.rs) is dead code; the live `SpecWatcher::debounce_loop` hand-rolls a Vec and hardcodes a 50 ms window (watcher.rs:45). 50 ms suits editor saves; it is wrong for reload events, which are expensive (recompile + re-handshake + re-describe + full re-validate) and which arrive from toolchains (`cargo build` writing a blob) rather than keystrokes. Reload also needs path-class routing (spec change vs plugin artifact change), and out-of-root plugin dirs hit C14-08's absolute-path fallback.

## Per candidate

### KEEP_WASM

**Reload mechanics: accidentally reload-first.** Because every call already builds a fresh `Plugin` from raw bytes (C7-08: "full Wasmtime compile + instantiate before each handshake/describe exchange"), no state lives in a plugin instance between calls — so swapping a module is trivially safe. The primitive exists: `ExtismRuntime::load_module_bytes`/`load_module` re-instantiate under the plugins mutex (runtime.rs:58–60, 80–97). The *costs* are the gap: C7-02 means no AOT (the `.aot` cache is a byte-copy; `_aot_cache_path` is literally an ignored parameter, runtime.rs:84) and C7-08 means no warm instance to swap into — every reload pays a full Cranelift compile of a 324–415 KB blob. Fine per reload; irrelevant per call.

**The authoring loop is the real problem.** Plugins are Rust: edit → `cargo build --target wasm32-unknown-unknown` → blob → reload. A compiler sits inside the inner loop, seconds at best. The builtin loop is worse: blobs are committed, embedded via `include_bytes!` (crates/specforge-extism/src/builtins.rs), and policed by `builtin_blob_sync` + `extension_json_sync` guard tests — so editing a builtin today means rebuild blob, rebuild the *host binary*, keep two guard tests green. A dev-loop mode that loads from disk ahead of the embedded bytes (composite.rs tries builtin first, so precedence work is needed) is mandatory before R-5 is honest.

**Debugging: the weakest surface in the codebase.** The entire trap story is `WasmTrapInfo { kind, message, export_name }` with `message = e.to_string()` from extism (runtime.rs:140–147) — no line numbers, no guest backtrace, no source mapping. The host exposes exactly three functions to guests (`emit_diagnostic`, `read_file`, `query_graph`, host_context.rs:56–61); there is no log/console channel, so authors debug by emitting fake diagnostics. Fixable inside the KEEP_WASM option (wasmtime has `WasmBacktrace`; a log host fn is easy), but it is new host work.

**Test loop: the strongest.** Determinism is structural (fixed module bytes), the differential-verification culture already exists (`verify_incremental` cold-vs-incremental comparison, pipeline.rs:402–429), and watch's JSON events (`{"event":"rebuilt",...}`) are ready for a machine-consumable `plugin_reloaded` event.

### LUA

Reload = re-read the script and `load` a fresh chunk; a fresh Lua environment per reload is natural, which also cleanly satisfies R-6 (no ambient state) and R-2 (sandbox reset per reload). No toolchain in the loop: edit → save → result is sub-second. Debugging is dramatically better out of the box: `mlua` errors carry chunkname:line tracebacks — the difference between `call_failed: <string>` and a pointer at the failing rule. The 50 ms debounce becomes tolerable because reload is cheap. Same watch-side plumbing as wasm (item 1 above), minus AOT/warm-engine work.

### PYTHON

Best tracebacks in the industry, but reload is the worst-behaved: `importlib.reload` leaves stale `sys.modules` and module-level state; the safe alternatives (subinterpreters — still flagged experimental in CPython — or a per-reload fresh interpreter) are heavy, and the GIL serializes analyze-time calls. Embed-and-ship fragility (evidence §4) compounds under reload, and system-Python drift makes the loop's output environment-dependent — hostile to R-6 snapshots.

### TYPESCRIPT

`deno_core` reload = fresh isolate + transpile-per-reload; V8 stack traces with source maps are good, and the permissions model maps onto R-2. But the reload path carries the most machinery (module loader invalidation, TS transpile), the dep weight is the largest, and a QuickJS variant needs an embedded transpiler. A Node sidecar makes reload trivial (respawn) but breaks R-3 distribution.

### MULTI

The watch-side plumbing (artifact watch, path-class debounce, config swap, structured reload events) is runtime-neutral — the existing `ValidatorDescriptor`/`plan_incremental_dispatch` types (dispatch.rs) already are — and would be shared. But each runtime adds a reload backend *and* a distinct debug surface, and C7-03's stringly-typed no-IDL protocol multiplies worst in an N-runtime world: reload bugs become protocol-drift bugs.

## The Hashimoto lens

Terraform designed the reload problem away: plugins are separate processes speaking a versioned protocol, so the dev loop is "restart the plugin" — cheap because the boundary is cheap, and the plugin surface stays data-like (Ghostty's live config reload is fast for the same reason: config is data, not compiled code). The lesson for SpecForge: the one cost no host-side engineering removes is a compiler inside the plugin loop. Rust→wasm puts one there; that is a permanent dev-loop tax, and it lands precisely on the product's primary authors — AI agents iterating through watch + MCP, for whom loop latency and structured, file:line diagnostics *are* the product.

Fixing C7-08/C7-02 (AOT-serialized modules, real instance pool) and adding a from-disk override + backtrace host fn would make KEEP_WASM's reload *acceptable* — but it is more host engineering than the reload plumbing itself, spent to stay merely adequate on this dimension.

## Verdict

**Verdict:** LUA
**Confidence:** 3
**One-line rationale:** R-5 is decided by the plugin author's inner loop — interpreter reload (re-read script, fresh state, file:line tracebacks, no toolchain in the loop) beats wasm's compile-in-the-loop authoring even after C7-02/C7-08 are fixed; the watch-side plumbing is runtime-neutral and required regardless.
