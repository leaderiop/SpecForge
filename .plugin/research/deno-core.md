# Embedding JavaScript/TypeScript in Rust: `deno_core` (V8) vs quickjs-ng (`rquickjs`) vs `boa_engine`

**Researcher:** `research-deno-core` · 2026-09-27 · Feeds the plugin-runtime decision (`.plugin/decision-brief.md`, options `KEEP_WASM | LUA | PYTHON | TYPESCRIPT | MULTI`). This file evaluates the three realistic Rust-embeddable JS engines for the `TYPESCRIPT` option against SpecForge's hard requirements R-2 (sandboxable), R-3 (single binary), R-5 (hot reload), R-6 (determinism).

## Measured on this machine (Apple M3 Pro, macOS arm64, release build, thin LTO, strip, codegen-units=1)

Minimal hello-world embeds — create engine, evaluate `fib(30)` (same recursive JS in all three), 11 runs, medians. Benchmark scaffolding: `/tmp/js-embed-bench/{deno-core,rquickjs,boa}` (throwaway).

| | `deno_core` 0.412.0 | `rquickjs` 0.14.0 (quickjs-ng) | `boa_engine` 0.22.0 |
|---|---|---|---|
| Binary size of host + engine | **44.2 MB** | **1.37 MB** | **7.98 MB** |
| Engine init (runtime/context create) | 8.0 ms | **0.21 ms** | 0.46 ms |
| Eval `fib(30)` (incl. parse/compile) | **10.4 ms** | 71.2 ms | 411.3 ms |
| Total per-call (init+eval) | 19.0 ms | 71.4 ms | 411.7 ms |
| Clean build time | 1 m 56 s | 1 m 17 s | 4 m 46 s |

Takeaways: V8 costs **32× the binary** and **~38× the init time** of quickjs-ng, and buys **~6.8× faster** pure-JS throughput; boa is ~39× slower than V8 and ~5.8× slower than quickjs-ng. For SpecForge's workload (plugins are mostly JSON marshalling over host ops, per `prds/PRD-001-extension-protocol.md`: `query`, `emit_diagnostic`, `resolve_ref`, `read_file` imports; validators/collectors), the throughput gap matters less than binary size, init cost (R-5 hot reload = engine re-init per edit), and sandboxing.

## deno_core (V8)

- **Binary size / startup.** 44.2 MB marginal cost measured above for a stripped hello world (V8 static library linked in; the build pulls the prebuilt V8 artifact via the `deno_v8`/`rusty_v8` build system). Full Deno the product is 83.6–103 MB across platforms (third-party size table: [ahaoboy/js-engine-benchmark, 2025-07-08](https://dev.to/ahaoboy/js-engine-benchmark-2025-7-8-163b)). Init is ~8 ms/isolate on this machine; `JsRuntimeForSnapshot` + startup snapshots exist specifically to cut boot cost ([deno_core ARCHITECTURE.md](https://github.com/denoland/deno_core/blob/main/ARCHITECTURE.md), [Deno blog pt3](https://deno.com/blog/roll-your-own-javascript-runtime-pt3)).
- **Permission model — can you deny fs/net?** Yes, trivially and by construction: `deno_core` ships **zero** host-capability ops. The embedder registers exactly the ops it defines; "user code can access whatever system services an embedder has decided to provide" ([ARCHITECTURE.md](https://github.com/denoland/deno_core/blob/main/ARCHITECTURE.md)). Deno's `--allow-*` system is a *separate* crate (`deno_permissions`, [crates.io](https://crates.io/crates/deno_permissions), 0.118.0) used by `deno_runtime` — you only need it if you adopt Deno's fs/net API crates. A SpecForge host would register only `query`/`emit_diagnostic`/`resolve_ref`/`read_file`-style ops with capability checks in `OpState` — ambient fs/net is unrepresentable rather than denied. Supabase's edge-runtime demonstrates the full stack: `deno_core` + `deno_ast` (transpiling) + `deno_permissions` ([supabase/edge-runtime Cargo.toml](https://github.com/supabase/edge-runtime/blob/main/Cargo.toml)).
- **Maturity / maintenance.** Actively maintained by Deno Land (updated 2026-09-16) with **400 releases since 2019**; 0.311 → 0.412 in ~24 months ≈ weekly minor bumps, 0.x semver ([crates.io](https://crates.io/crates/deno_core)). Lockstep coupling is real: `deno_permissions` published at the same timestamp as `deno_core` 0.412.0. Docs are a known weak spot — open issues [#237 "`op2` documentation updates"](https://github.com/denoland/deno_core/issues/237) and [#79 "Add documentation on `extension!`"](https://github.com/denoland/deno_core/issues/79); the official "roll your own runtime" tutorial was last retargeted 2024-09-26 ([Deno blog](https://deno.com/blog/roll-your-own-javascript-runtime)). TypeScript *transpilation* is not deno_core's job: embedders add `deno_ast` (SWC-based, 5.3M downloads, [crates.io](https://crates.io/crates/deno_ast)) — the documented approach in [Deno blog pt2](https://deno.com/blog/roll-your-own-javascript-runtime-pt2).

## quickjs-ng via rquickjs

- **Performance.** Interpreter-only: ~6.8× slower than V8 on `fib(30)` here; on the old V8 benchmark suite qjs-ng scores 1,270 vs V8's 44,072 (macOS arm64) — ~35× on JIT-favoring workloads ([ahaoboy table](https://dev.to/ahaoboy/js-engine-benchmark-2025-7-8-163b)). AWS chose it anyway for cold-start latency: LLRT advertises "up to over 10x faster startup" than other JS runtimes on Lambda ([awslabs/llrt](https://github.com/awslabs/llrt)).
- **Safety & sandbox.** Engine is C, but tiny and self-contained ("a few C files, 210 KiB of x86 code for hello world" — [rquickjs README](https://github.com/DelSkayn/rquickjs)); **zero advisories in OSV for quickjs, quickjs-ng, and rquickjs** ([OSV query](https://api.osv.dev/v1/query), 2026-09-27). The safe wrapper exposes first-class limits: `set_memory_limit`, `set_max_stack_size`, and `set_interrupt_handler` — the handler raises an uncatchable exception and returns control to the host, which is exactly the mechanism `max_execution_ms` (audit finding C7-10) needs ([docs.rs Runtime](https://docs.rs/rquickjs/latest/rquickjs/struct.Runtime.html)). No fs/net/globals exist unless the host adds them ([rquickjs README](https://github.com/DelSkayn/rquickjs)). `Send`/`Sync` only behind the `parallel` feature, which upstream flags as experimental — LLRT ships it in production ([llrt_core/Cargo.toml](https://github.com/awslabs/llrt/blob/main/llrt_core/Cargo.toml)).
- **Maintenance.** quickjs-ng is the maintained fork of Bellard's QuickJS led by Ben Noordhuis and Saghul ([README](https://github.com/quickjs-ng/quickjs/blob/master/README.md)); 3,810 GitHub stars. `rquickjs`: 4.56M total / 2.07M recent downloads (recent downloads now **exceed deno_core's** 1.72M), 43 releases, updated 2026-09-18, self-described "feature complete, mostly stable; error handling may change" ([crates.io](https://crates.io/crates/rquickjs), [README](https://github.com/DelSkayn/rquickjs)). Ships prebuilt bindings for macOS arm64/x64 and Linux x64/musl — covers R-3's matrix ([platform table](https://github.com/DelSkayn/rquickjs#supported-platforms)).

## boa_engine

- Pure-Rust engine (7,570 stars, [boa-dev/boa](https://github.com/boa-dev/boa)) — memory-safe by construction, which neither V8 nor quickjs can claim. 13 releases, 0.22.0 on 2026-08-28, 5.09M downloads ([crates.io](https://crates.io/crates/boa_engine)). Conformance was 87.3% Test262 at v0.19 (July 2024, [release post](https://boajs.dev/blog/2024/07/09/boa-release-19/)) — meaningfully below quickjs's "nearly 100%" ES2020 figure and far below V8.
- **Performance is disqualifying for per-request work:** ~39× slower than V8 on `fib(30)` here; the ahaoboy suite scores it 107–188 vs V8's ~44,000 and took 234 s vs 20 s on Ubuntu ([table](https://dev.to/ahaoboy/js-engine-benchmark-2025-7-8-163b)). Binary is a middle 8 MB, init 0.46 ms.
- **Safety record:** 2 OSV advisories, incl. **RUSTSEC-2024-0444 / CVE-2024-43367** — remote-DoS via `AsyncGenerator` state transition, CVSS with A:H, patched in 0.19 ([advisory](https://github.com/boa-dev/boa/security/advisories/GHSA-f67q-wr6w-23jq)). Upstream's workaround guidance (`catch_unwind` around engine calls) tells you panics are an expected containment tool.
- **API design:** ergonomic and improving (`boa_interop`, `ContextData`, `js_class!` — [0.19 post](https://boajs.dev/blog/2024/07/09/boa-release-19/)), but the GC rewrite and register-VM needed for competitive performance are still in progress, and v1.0 is explicitly "some way off".

## Safety & sandboxing under R-2 (cross-cutting)

- **Capability posture is equivalent** across all three: no ambient fs/net in any of them; capabilities are host-registered functions. The difference is what you can *reuse*: deno_core offers the battle-tested `deno_permissions` crate if you adopt Deno's API crates (heavier); rquickjs/boa give you a blank slate (smaller, but DIY audit).
- **Memory-safety depth differs.** V8: 60% of in-the-wild Chrome renderer RCE exploits 2021–2023 originated in V8; the V8 Sandbox (default on 64-bit since ~2022, build flag `v8_enable_sandbox`, ~1% overhead, 1 TB VA reservation) contains corruption spread but is explicitly *not yet* "a strong security boundary" ([v8.dev/blog/sandbox](https://v8.dev/blog/sandbox)). It is a Chrome-attacker-mitigation, not a plugin sandbox; SpecForge's sandbox is the op whitelist plus the host's Rust code.
- **CVE surface:** OSV counts (crates.io ecosystem, 2026-09-27): deno_core 0, rusty_v8 0, rquickjs 0, quickjs/quickjs-ng 0, boa_engine 2, deno (product) 38, wasmtime (incumbent) 83. V8 *engine* CVEs are tracked in Chrome's release channel, not crates.io — the 0 for rusty_v8 reflects ecosystem accounting, not absence of V8 bugs; the sandbox post above is the honest baseline. quickjs-ng's small C core gets continuous OSS-Fuzz-style attention via the ng project.

## Fit to SpecForge's integration surface

- **Host-API protocol** (C7-03 stringly JSON): all three marshal via serde/Rust structs; `rquickjs` has an official serde bridge ([rquickjs-serde](https://github.com/rquickjs/rquickjs-serde)); deno_core has `serde_v8` built in.
- **Hot reload (R-5):** measured init cost per engine instance — 0.21 ms (qjs) vs 8 ms (V8) — makes quickjs re-instantiation effectively free on every file save; V8 wants snapshot warm-up engineering.
- **Determinism (R-6):** no engine gives it for free (`Date`, `Math.random` everywhere); host must inject a frozen clock/RNG into whichever engine. quickjs's interrupt handler + memory limit map 1:1 onto the currently-unenforced `max_execution_ms`/`query_scope` audit findings (C7-09/C7-10, `.plugin/evidence.md`).
- **TS authoring:** all three need a strip-types pass; the Deno ecosystem's `deno_ast` (or oxc/SWC) slots in front of any engine — it is not a deno_core differentiator.
- **Compiler passes** with "several hundred lines of real logic" (`.plugin/decision-brief.md`): V8's 6.8× throughput edge only matters if passes do heavy numeric work; on this workload (graph walking + diagnostics over host ops) the bottleneck is host↔plugin marshalling, where quickjs-ng's small-heap GC performs fine.

## Real projects embedding JS in Rust

- **Deno** — `deno_core`/V8, the reference embedder ([denoland/deno_core](https://github.com/denoland/deno_core)).
- **Supabase Edge Runtime** — Rust host embedding `deno_core` 0.324 + `deno_ast` + `deno_permissions` ([repo](https://github.com/supabase/edge-runtime/blob/main/Cargo.toml)).
- **AWS LLRT** — Rust runtime on quickjs via `rquickjs` 0.14 (`futures`, `parallel`, `rust-alloc`) ([llrt_core/Cargo.toml](https://github.com/awslabs/llrt/blob/main/llrt_core/Cargo.toml)).
- **Servo** — Rust browser embedding SpiderMonkey through the `mozjs` bindings (precedent for engine-in-Rust at scale) ([servo/mozjs](https://github.com/servo/mozjs)).
- **rquickjs-extra + LLRT modules** — community pure-Rust module layer for rquickjs hosts ([rquickjs-extra](https://github.com/rquickjs/rquickjs-extra)).

## Maintenance & risk summary

| | deno_core | rquickjs/quickjs-ng | boa |
|---|---|---|---|
| Release cadence | ~weekly, 400 releases, 0.x churn | 43 releases, steady, "mostly stable" | 13 releases, slow but alive |
| Docs | known gap (open issues #237/#79) | good rustdoc + README platform table | improving, thin in places |
| Coupling risk | lockstep with Deno monorepo crates | engine fork + wrapper, both independent | single upstream org |
| Track record | powers Deno + Supabase in prod | powers AWS LLRT | no marquee production embedder |

## Bottom line

**Verdict:** TYPESCRIPT (via **quickjs-ng/rquickjs** as the engine — not deno_core) · **Confidence:** 4/5 · **Rationale:** quickjs-ng matches SpecForge's hard requirements almost exactly — 1.4 MB binary vs V8's 44 MB (R-3), 0.2 ms re-init for hot reload (R-5), first-class memory/stack/interrupt limits (R-2, R-6) and a clean OSV record — while the throughput V8 would add is mostly unexercised by op-marshalling plugin workloads, and boa's ~39× interpreter deficit plus DoS advisory rule it out.
