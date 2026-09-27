# Hot-reload patterns in real projects (R-5 evidence)

Batch-2 fleet research for the SpecForge plugin-runtime decision. Covers: how esbuild/devd/nodemon
reload, how Envoy hot-restarts wasm filters, how Redis handles `SCRIPT FLUSH`, how Obsidian reloads
plugins, and the fastest reload loop per runtime (wasm/Lua/Python/TS). Companion to
[dimensions/D11](../dimensions/D11-hot-reload-devloop.md) (which covers SpecForge's internal watch
mechanics); this file supplies the external prior art and measured reload costs.

Method: primary docs/specs read directly (2026-09-27); micro-benchmarks run locally on the M3 Pro
host (wasmtime 43.0.2 release build, Lua 5.4 + LuaJIT, CPython 3.14, Node 22.22, Deno, Bun 1.3.4;
representative ~140-line plugin with 120 rules; sources linked inline). Numbers marked [INFERENCE]
are derived, not measured.

## 1. The four reload architectures

### 1.1 esbuild — content rebuild, plugins fixed at context creation

- The unit of incremental work is the **context**: "All builds done with a given context share the
  same build options, and subsequent builds are done incrementally (i.e. they reuse some work from
  previous builds)" ([api docs](https://esbuild.github.io/api/)). Three incremental APIs: watch,
  serve+live-reload, and manual `rebuild()`.
- `rebuild()` caches two things across builds; the first is explicit: "Files are stored in memory
  and are not re-read from the file system if the file metadata hasn't changed since the last
  build" — i.e. an mtime-keyed content cache ([api #rebuild](https://esbuild.github.io/api/#rebuild)).
- **Plugins are never hot-reloaded.** They are part of the build options passed at context
  creation; `setup` "is run once for each build API call" but the plugin *set* is immutable for the
  context's life ([plugins docs](https://esbuild.github.io/plugins/)). Changing a plugin = dispose
  the context, create a new one. esbuild is the world's fastest rebuild loop and it still treats
  "reload the plugin host's plugins" as out of scope.
- Live reload of the *served app* is not an esbuild feature either: "There is no esbuild API for
  live reloading directly. Instead, you can construct live reloading by combining watch mode and
  serve mode plus a small bit of client-side JavaScript" ([api #live-reload](https://esbuild.github.io/api/#live-reload)).
- Watch has a `delay` knob (default 0) explicitly for "a tool that regenerates multiple source
  files very slowly" — debounce is recognized as per-trigger-source, exactly D11's path-class point.

**Lesson:** the incremental cache is keyed by input content identity (mtime/hash); the plugin
registry is immutable per context; reload applies to *inputs and outputs*, never to the plugin
mechanism itself.

### 1.2 devd (+modd) — reload the content, never the server

devd ([cortesi/devd](https://github.com/cortesi/devd)) is a zero-config dev HTTP server. Its reload
story never touches its own process or any plugin mechanism ([README](https://raw.githubusercontent.com/cortesi/devd/master/README.md)):

- It injects a script before `</head>` (first 30 KB only) that "listens for change notifications
  over a websocket connection, and reloads resources as needed". If *only* CSS changed, it reloads
  only external CSS; otherwise a full page reload — **minimal-scope reload by path class**.
- Watches can point at trees that are not served (`-w ./src` reverse-proxy mode) — watch scope and
  serve scope are decoupled.
- `SIGHUP` triggers a livereload notice to all connected browsers, "allowing external tools, like
  devd's sister project modd, to trigger livereload" — reload is an externally-triggerable event,
  not an internal loop ([modd](https://github.com/cortesi/modd) does watch→run-command).

**Lesson:** the cheapest reload architecture is one where the thing being reloaded is *data* (the
artifact) and the consumer is notified. The host's code never changes; there is no plugin-reload
problem, only artifact-invalidation.

### 1.3 nodemon — process replacement as the reload primitive

nodemon ([remy/nodemon](https://raw.githubusercontent.com/remy/nodemon/master/README.md)) has no
in-process reload at all:

- On change (default debounce 1 s, `--delay` configurable, "The timeout before checking for new
  file changes is 1 second"), it kills the child process and re-execs it.
- The cleanup contract is a **signal protocol**: nodemon sends `SIGUSR2` on restart; apps install
  `process.once('SIGUSR2', …)` to do graceful shutdown before dying. `--signal SIGHUP` repurposes
  restart into an in-app "reload your configuration" hook — the app decides what reload means.
- `rs` + Enter = manual restart. Everything else (watch lists, `execMap`, polling fallback) is
  plumbing around the one primitive: *replace the process*.

This is the same primitive as HashiCorp go-plugin sidecars and (scaled up) Envoy hot restart: no
stale-state class of bug exists because no state survives. Cost = process startup + drain.

### 1.4 Envoy — two distinct mechanisms; neither mutates a live VM

**(a) Whole-binary hot restart** ([arch overview](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/operations/hot_restart)):

- New process **fully initializes** (config load, discovery, health checks) *before* asking the old
  process for listen sockets over a unix-domain-socket RPC. Old process drains; sockets are passed
  per worker index (`reuse_port`); existing connections are never transferred — they complete or
  die during drain. Counters are shipped old→new over the UDS (gauges except `NeverImport`);
  `server.hot_restart_generation` survives. Designed to work across containers.

**(b) Wasm filter updates via xDS** ([wasm.proto API](https://www.envoyproxy.io/docs/envoy/latest/api-v3/extensions/wasm/v3/wasm.proto), [wasm filter](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/wasm_filter)):

- VM selection is **content-addressed**: `vm_id` "will be used along with a hash of the wasm code …
  to determine which VM will be used for the plugin. All plugins which use the same `vm_id` and
  code will use the same VM." Sharing is a memory optimization with acknowledged "security
  implications" — cross-plugin state sharing is called out as a hazard, not a feature.
- Config update (same code) *reconfigures* the live plugin through the ABI: `configuration` is
  "used to configure or reconfigure a plugin (`proxy_on_configure`)"
  ([proxy-wasm ABI v0.2.1](https://raw.githubusercontent.com/proxy-wasm/spec/master/abi-versions/v0.2.1/README.md));
  a rejected `on_configure` during xDS update NACKs the update.
- Code update (new hash) = **new VM**, with an async-fetch/warming path: `nack_on_code_cache_miss`
  "otherwise fetch the code asynchronously and enter warming state". The old VM is never mutated
  in place; on VM fatal error, `failure_policy: FAIL_RELOAD` + `ReloadConfig.backoff` (default 1 s
  base) creates a fresh plugin instance for new requests.
- `allow_precompiled` exists but is flagged: "precompiled code is not verified" — trusted-only,
  same trust boundary SpecForge's R-1/R-2 forbids assuming.
- Envoy's official build ships V8 as the wasm runtime (search order "v8 -> wasmtime -> wamr";
  wasmtime/WAMR "not enabled in the official build"). Filters get one VM per worker; only
  non-filter WasmServices can be `singleton`.

**Lesson:** Envoy — the flagship wasm-plugin host — reloads by *swap-and-drain over a
content-addressed cache*, with reconfigure-vs-recode as distinct events, a warming state, and a
failure-reload backoff. Nothing mutates a live VM. R-5's honest implementation is this vocabulary.

### 1.5 Redis — the reload-hostile case designed away: stateless code, volatile cache

- Script cache is **always volatile by design**: "It isn't considered as a part of the database and
  is **not persisted**. The cache may be cleared when the server restarts, during fail-over … or
  explicitly by `SCRIPT FLUSH`. … the cache's contents can be lost at any time"
  ([eval-intro](https://redis.io/docs/latest/develop/programmability/eval-intro/)). The application
  owns reloading: `EVALSHA` → `NOSCRIPT` error → `SCRIPT LOAD` → retry. Staleness is a *protocol
  signal*, not a crash.
- `SCRIPT FLUSH` is O(N) over cached scripts, `ASYNC|SYNC` since 6.2 (default from
  `lazyfree-lazy-user-flush`) ([script-flush](https://redis.io/docs/latest/commands/script-flush/)).
  `SCRIPT KILL` can only interrupt a script that "did not modify the dataset".
- Redis 7 **functions** flip the ownership: libraries are "first-class software artifacts of the
  database … persisted to the AOF file and replicated" ([functions-intro](https://redis.io/docs/latest/develop/programmability/functions-intro/)).
  Reload = `FUNCTION LOAD REPLACE` — and the unit of replacement is the **whole library**
  ("libraries are updated as a whole with all of their functions together in one operation";
  partial updates are impossible). Same engine as SpecForge's model: Lua, deterministic effects
  (effects-replication default since 5.0; verbatim replication removed in 7.0), atomic blocking
  execution, one embedded interpreter (Lua 5.1).

**Lesson:** Redis is the cleanest statement of the property R-6 wants: *code units are immutable,
content-addressed, and safe to drop at any time; all state lives in the host dataset; reload is a
cache miss.* The interpreter is never "reloaded" — only the compiled chunk cache is invalidated.
The registry already sha256s blobs (evidence §1.4), so SpecForge can get Redis semantics for free
on the wasm path (and any other).

### 1.6 Obsidian — lifecycle-hook reload in a shared, unsandboxed context

The de-facto mechanism is the community [hot-reload plugin](https://raw.githubusercontent.com/pjeby/hot-reload/master/README.md)
(itself a plugin — no host support):

- Watches `main.js`/`styles.css` of plugin dirs marked by `.git` or `.hotreload`; "automatically
  disables and re-enables that plugin once changes have stopped for about three-quarters of a
  second" — opt-in per plugin via marker file, ~750 ms quiet-period debounce.
- Reload = `disable → enable`: unload hooks run, then the module re-executes fresh.
- The correctness burden is pushed to the plugin: "it's your *plugin's* job to properly clean up
  after itself. If you're not making good use of `onunload()` and the various `registerX()` … you
  may leave Obsidian in an unstable state." A broken load loop self-heals by re-saving the file
  (hot-reload retries enable).
- Marketplace installs of the same plugin never hot-reload (no marker file) — dev-mode reload is
  deliberately distinct from distribution.

**Lesson:** in a shared-memory JS context, reload safety = lifecycle-hook discipline plus fresh
module execution; and opt-in markers + short debounce make the dev loop feel instant without ever
affecting installed-plugin behavior. (Note for R-2: Obsidian shows what *not* to do for SpecForge —
plugins run with full app access, no isolation; fine for a personal tool, disqualified as a model.)

## 2. Fastest reload loop per runtime (measured, M3 Pro / macOS arm64)

Representative plugin: 120-entry rule table + `validate()` (~140 lines, 8 KB TS / 3.4 KB Lua).
Host-side costs, in-process unless noted. Script: see §Method; wasmtime bench used a 1.15 MB
compiled module (~3× SpecForge's 324–415 KB blobs) plus a 50 KB one.

| Runtime | Compile/load per reload | Fresh state | Call (warm) | Process spawn (sidecar floor) |
| --- | --- | --- | --- | --- |
| **wasm** (wasmtime 43.0.2) | Cranelift compile: **11 ms** (50 KB module) → **233 ms** (1.15 MB) ⇒ ~65–80 ms at SpecForge blob size [INFERENCE, linear]; **AOT deserialize: 16 ms** (1.15 MB) ⇒ ~5–6 ms at blob size | **instantiate: 0.74 µs** (fresh Store+Instance) | **13 ns** | wasmtime CLI not measured; host-embedded ⇒ n/a |
| **Lua 5.4** | `load`: **91 µs** | exec chunk 25 µs | 7.4 µs | 14.6 ms (`lua -e ''`) |
| **LuaJIT** | `load`: **81 µs** | exec chunk 6 µs | 0.31 µs | 13.0 ms |
| **Python 3.14** | `importlib.reload`: **361 µs**; fresh import 363 µs | (state-leak trap: reload keeps `sys.modules` side effects — D11) | ~µs order [INFERENCE] | **45.7 ms** (`python3 -c pass`); subinterpreters heavier, still experimental [INFERENCE] |
| **TS/JS** | Bun.Transpiler (SWC-class) transpile: **134–139 µs** (8 KB TS) | `eval` of transpiled JS: 44 µs (same isolate); fresh V8/QuickJS isolate ~1–5 ms [INFERENCE] | ~ns–µs | Bun 13.7 ms / Deno 31.5 ms / Node 42.5 ms |

Reading, per R-5's *edit → re-run analyze* loop with a warm host:

1. **Lua wins the reload event**: ~0.1 ms to fresh, callable code, no toolchain, and (per Redis/Obsidian
   practice) a fresh chunk/environment per reload gives R-6 determinism structurally.
2. **TS is a close second on the reload event** (~0.14 ms transpile + isolate churn), but the host
   carries a transpiler + engine and the reload machinery is the bulkiest of the four (D11).
3. **Python's in-process reload is fast (~0.36 ms) but a trap**; every serious design routes around
   it (fresh interpreter/process: 46 ms floor, GIL-serialized calls) — the worst real-world loop.
4. **wasm has a split personality.** Per-call it is the fastest thing on the page (13 ns) and
   instantiation is effectively free (0.74 µs) — but the *reload* pays Cranelift: ~65–80 ms today
   (C7-02's fake AOT = full compile each time), dropping to ~5–6 ms with a *real* AOT cache
   (deserialize), which is the single highest-leverage fix available on the wasm path. And the
   **authoring** loop is a different axis: Rust→wasm puts `cargo build --target wasm32` (seconds)
   inside it — the one cost no host-side engineering removes (D11's compiler-in-the-loop tax).

Cross-check vs prior art: all four surveyed hosts keep the *host* warm and swap content-addressed
artifacts (esbuild mtime cache, Envoy code-hash VM cache, Redis SHA1 script cache, Obsidian
re-require). None reloads by recompiling the host, and none mutates a live code unit.

## 3. What SpecForge should copy (runtime-neutral watch plumbing)

1. **Content-addressed artifact cache + volatile-cache semantics** (Redis/Envoy): key loaded
   modules by sha256 of the blob/script; reload = drop cache entry, reload on next use; registry
   already pins sha256s. `NOSCRIPT`-style staleness errors instead of silent stale serves.
2. **Swap-and-drain, never in-place mutation** (Envoy): new code ⇒ new instance/VM, old one finishes
   in-flight analyze calls, then drops. Config-only change ⇒ `proxy_on_configure`-style reconfigure
   (SpecForge analog: re-describe manifest) without new code.
3. **Path-class debounce + marker-file dev mode** (esbuild `delay`, Obsidian `.hotreload`, devd
   CSS-vs-page): 50 ms debounce is right for `.spec` saves; reload events need their own quiet
   window (~750 ms observed in the wild) and an opt-in dev-mode marker so installed plugins never
   hot-reload. Emit a structured `plugin_reloaded` event (nodemon's signal protocol is the app-facing
   analog; D11's JSON events are the surface).
4. **Lifecycle cleanup contract** (Obsidian `onunload`): whichever runtime, the reload-safety burden
   must be a documented plugin contract, enforced by the SDK (Extism/SDK or mlua sandbox-reset-per-
   reload make it mechanical; Python's `importlib.reload` makes it manual and treacherous).
5. **Failure-reload with backoff** (Envoy `FAIL_RELOAD` + `ReloadConfig`): a plugin that traps on
   reload should not wedge watch; reload per request/iteration with 1 s-class backoff.
6. **Keep the plugin set immutable per context** (esbuild): changing the *set* of installed plugins
   is a context rebuild, not a patch. Reload applies to one plugin's artifact.

## Bottom line

Every mature host converges on the same three-part pattern: **content-addressed compiled-artifact
cache (drop-safe at any time) + swap-and-drain with an explicit cleanup contract + debounced,
path-classed watch events**. esbuild refuses to reload plugins at all; devd/nodemon never reload
code in-process (content notification / process replacement); Envoy and Redis — the two hosts that
*do* run third-party code against state — both make code units immutable, hash-keyed, and
replaceable whole, with the interpreter/state never mutating. On measured reload-event cost the
ranking is Lua (~0.1 ms) < TS (~0.14 ms + isolate) < wasm with real AOT (~5–6 ms) / wasm today
(~65–80 ms, C7-02) < Python-in-process (fast but state-unsafe; honest loop = 46 ms process). wasm
keeps the fastest warm call (ns-class) but is the only candidate with a compiler inside the
authoring loop; the fixable half of its reload tax (fake AOT) is worth fixing regardless of the
runtime decision.

## Verdict

**Verdict:** LUA
**Confidence:** 3
**One-line rationale:** Prior art is unanimous — reload = drop-safe, content-addressed artifact swap with a fresh execution environment; Lua gets that loop for ~0.1 ms with no toolchain and structurally fresh state per reload, while wasm's reload tax (65–80 ms compile today, seconds of cargo in the author loop) is the pattern every other host engineered away.
