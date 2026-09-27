# Startup Latency & Per-Call Overhead — Benchmarked Numbers (C3 performance input)

All five candidate runtimes measured **first-hand, in-process** on this machine, plus
published numbers from engine vendors cited inline. Purpose: give the
`KEEP_WASM / LUA / PYTHON / TYPESCRIPT / MULTI` decision real startup-latency and
per-call-overhead data instead of folklore.

## Method

- Machine: Apple M3 Pro (arm64), macOS (Darwin 25.6.0), rustc 1.98.1, release builds, medians over 50–10,000 iterations; several runs per figure (ranges shown where runs disagreed).
- Harness: throwaway Rust benches written for this analysis (not committed):
  wasmtime **43.0.2** (the same version SpecForge pins), mlua **0.11.6** with vendored
  **Lua 5.4.8** and **LuaJIT 2.1** (2.1.1765228720), PyO3 **0.26** against system
  **CPython 3.14.7**, rquickjs **0.9** (QuickJS), rusty_v8 **0.106** (V8).
- Measurement floor: a single `Instant::now()` pair costs ~25–40 ns here, so raw
  "call" timings all showed ~42 ns. True per-call figures below come from **batched**
  timing (1,000 calls per timed region, median of 50 regions).
- Process-spawn row: median of 60 `subprocess.run` launches from a Python driver
  (floor for that driver: `/usr/bin/true` ≈ 6.5 ms — so spawn numbers are upper bounds).

## First-hand measurements (this machine, 2026-09-27)

### Startup / initialization

| Operation | Median | Notes |
| --- | --- | --- |
| **wasmtime** `Engine::new` | 0.11–3.9 ms | first engine in a process is the expensive one |
| **wasmtime** compile, 1-func module (WAT) | ~0.4–4.3 ms | noisy on a laptop; order: low ms |
| **wasmtime** compile, 256-func module (WAT) | 11–34 ms | |
| **wasmtime compile, real SpecForge blob** `specforge_ext_formal.wasm` (415,605 B) | **77–110 ms** | 3 runs: 110.1 / 91.1 / 76.7 ms |
| **wasmtime compile, real SpecForge blob** `specforge_ext_product.wasm` (370,362 B) | **~103 ms** | |
| **wasmtime** instantiate (fresh `Store`, on-demand allocator) | **0.6–1.4 µs** | tiny module; real blobs import extism PDK so bare instantiate N/A (see below) |
| **wasmtime** instantiate (pooling allocator) | **0.6–1.3 µs** | |
| **Lua 5.4** new `lua_State` (mlua) | 25–53 µs | |
| **LuaJIT** new `lua_State` (mlua) | ~38 µs | |
| **QuickJS** runtime + context create (rquickjs) | ~105 µs | |
| **CPython** interpreter init (PyO3 first attach, in-process) | **8.7–19 ms** (typ. ~15) | single shot; init once per process |
| **V8** platform init (once per process) | ~1.2 ms | |
| **V8** isolate + context create (embedded snapshot) | **~1.1 ms** | fresh isolate per iteration |

Process-level cold start (median spawn wall, includes ~6.5 ms harness floor):

| Runtime | `runtime -e ''` |
| --- | --- |
| lua 5.4.8 | 13.5 ms |
| luajit 2.1 | 14.3 ms |
| deno 2.9.6 (V8) | **26.0 ms** |
| node 22.22 (V8) | 40.2 ms |
| python3 3.14.7 | **41.1 ms** (`-I -S`: 35.4 ms) |

### Per-call overhead (batched, true per-call)

| Engine | call `add(1,2)` via embedding API | script-load+call (compile+run each event) |
| --- | --- | --- |
| wasmtime 43 (`TypedFunc::call`) | **13–31 ns** | — (compile is the event, see above) |
| QuickJS (`Function::call`) | **43 ns** | eval `"1+1"`: **1.6 µs** |
| Lua 5.4 (mlua) | **26 ns** | load+call chunk: **1.6–3.1 µs** |
| LuaJIT (mlua) | **25 ns** (first-100 calls ≈ warm calls — C-API cost masks JIT warmup) | load+call chunk: **1.1 µs** |
| V8 (`Function::Call`) | **40 ns** | eval `1+1` compile+run: **0.5 µs** |
| PyO3 (`call1` on a Python lambda) | **22 ns** | — |
| PyO3 GIL re-acquire | ~4 ns | — |

### The SpecForge-specific number

Compiling the **actual vendored builtin blobs** (`extensions/*/wasm/*.wasm`) with
wasmtime 43 costs **~77–110 ms per blob** on this M3 Pro. The blobs import
`extism:host/env::alloc` (extism PDK), so bare-wasmtime instantiation was skipped,
but per the vendor numbers below instantiation is µs-scale once compiled —
**compilation, not instantiation, is the cold-plugin cost**. Four builtins × ~100 ms
= **~0.4 s of pure Cranelift compile per binary run** if nothing is cached — exactly
the failure mode the audit flagged as C7-02 (AOT cache is a byte-copy) and C7-08
(EnginePool is a ledger, no warm instances). With a working AOT cache + pooling +
`InstancePre`, wasmtime's residual per-plugin cost drops to the µs range.

## Published numbers (corroboration)

- **Wasmtime 1.0: A Look at Performance** (Chris Fallin, Bytecode Alliance, 2022-09-06): instantiation of a multi-MB `SpiderMonkey.wasm` went from **~2 ms to 5 µs (400×)** via pooling + copy-on-write heap images + lazy table/function-object init; "instantiation from milliseconds to microseconds". https://bytecodealliance.org/articles/wasmtime-10-performance
- **Wasmtime book, "Tuning Wasmtime for Fast Instantiation"**: the official recipe — pooling allocator, `memory_init_cow`, `InstancePre` to pull import resolution out of the critical path. https://docs.wasmtime.dev/examples-fast-instantiation.html
- **Lucet announcement** (Pat Hickey, Fastly, 2019): "Lucet can instantiate WebAssembly modules in **under 50 microseconds**, with just a few kilobytes of memory overhead. By comparison, Chromium's V8 engine takes about **5 milliseconds**, and tens of megabytes of memory overhead, to instantiate JavaScript or WebAssembly programs." https://www.fastly.com/blog/announcing-lucet-fastly-native-webassembly-compiler-runtime
- **V8 custom startup snapshots** (v8.dev, 2015): snapshot deserialization cuts context creation "from **40 ms down to less than 2 ms**" on desktop, "**270 ms to 10 ms**" on phones; custom snapshots can additionally shave ~100 ms of library-script init. https://v8.dev/blog/custom-startup-snapshots — consistent with my measured 1.1 ms isolate+context (modern hardware, embedded snapshot) and deno 26 ms vs node 40 ms process starts.
- **Cloudflare Workers — How Workers works**: an isolate "can start **around a hundred times faster** than a Node process on a container or virtual machine" and uses "an order of magnitude less memory" on startup — production proof that V8 isolates are viable per-plugin sandboxes. https://developers.cloudflare.com/workers/reference/how-workers-works/
- **QuickJS** (Bellard, current page): "Fast interpreter with very low startup time… The complete life cycle of a runtime instance completes in **less than 300 microseconds**"; few C files, no external dependency, **367 KiB** x86 hello-world. https://bellard.org/quickjs/ — matches my measured ~105 µs runtime+context create.
- **LuaJIT performance page** (Mike Pall, archived 2020): JIT warmup is fast — LuaJIT 2 traces compile at the **57th loop iteration**, compile times "microsecond to millisecond range", VM startup "< 100 µs … negligible". https://web.archive.org/web/20200916225033/http://luajit.org/performance.html
- **CPython**: Python 3.11 whatsnew reports 10–60% faster (1.25× geomean) from the Faster CPython work; interpreter *init* remains ~10–40 ms class (my measurements: 8.7–19 ms in-process, 35–41 ms process). https://docs.python.org/3/whatsnew/3.11.html

## Reading for the decision

1. **Per-call overhead is a non-issue everywhere**: 13–43 ns across all five runtimes on this API surface. Any runtime choice is justified on grounds other than call overhead. The decision-matrix claim that quickjs is "10–50× slower compute" applies to compute-heavy loops (interpreter vs JIT), not to host↔plugin call mechanics.
2. **Cold start differs by 4–5 orders of magnitude**: compiling the real wasm blobs costs ≈10⁸ ns (≈100 ms) per plugin; script engines load the same "plugin" in ~1–3 µs (Lua/QuickJS chunk) or ~1 ms (V8 isolate); CPython init is ~15 ms once per process.
3. **KEEP_WASM's latency problem is entirely fixable inside the wasm model** (AOT cache that isn't a byte-copy, pooled warm instances, `InstancePre`) — and it is *already required* by audit items C7-02/C7-08. If those fixes land, wasmtime instantiation (0.6–1.3 µs measured; 5 µs vendor-confirmed on multi-MB modules) is competitive with script engines.
4. **Hot reload (R-5)**: script engines re-read source per event at µs cost; wasm hot reload today means re-Cranelift at ~100 ms per blob unless the AOT path is fixed.
5. **PYTHON is the only startup outlier**: ~15 ms interpreter init (once per process, GIL-reentrant after) + 35–41 ms process-level start; fine for a CLI pass, poor for per-event embedding.

## Bottom line

On latency alone: Lua (5.4 or LuaJIT) starts 3–4 orders of magnitude faster than the
current per-compile wasm path and matches it per-call; V8 isolates are the strongest
script-engine start with snapshot support; QuickJS is µs-cheap but slow at compute;
wasm is only competitive if the open C7-02/C7-08 fixes (real AOT + pooling) ship.

**Verdict:** LUA
**Confidence:** 4
**One-line rationale:** First-hand numbers show equal per-call overhead (~tens of ns) for every runtime, so cold start decides: ~100 ms Cranelift compile per wasm blob today vs 1–3 µs Lua script load and ~15 ms one-time CPython init — KEEP_WASM only reaches parity after fixing C7-02/C7-08, which is new work, not existing behavior.
