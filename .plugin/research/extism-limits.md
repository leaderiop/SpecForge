# Research: the `extism` crate (1.30) — limitations, API gaps, maintenance status

**Scope.** What the `extism` crate (SpecForge locks 1.30.0, `extism-pdk` 1.4.1 → `wasmtime`/`wasi-common` 43.0.2, see `Cargo.lock`) does **not** provide that a hand-rolled `wasmtime` integration would; its API design issues, community complaints, breaking changes, and maintenance trajectory. All external claims fetched directly from crates.io/GitHub/OSV APIs and the v1.30.0 source tree on 2026-09-27 (web_search provider was down; raw-source reads used instead).

## 1. Maintenance status: alive, but a one-lane road

- **Latest release is 1.30.0, 2026-06-04** — the same version SpecForge is on ([crates.io API](https://crates.io/api/v1/crates/extism)). Totals: 713,936 downloads, 196,722 in the last 90 days, 46 versions. Repo: 5,778 stars, 169 forks, **55 open issues**, last push 2026-09-02, not archived, BSD-3-Clause ([GitHub API](https://api.github.com/repos/extism/extism)).
- **The release cadence is now dependency-driven.** v1.21.0 (2026-03-26) contains exactly one change: "Upgrade wasmtime to v41" ([release](https://github.com/extism/extism/releases/tag/v1.21.0)). v1.30.0 (2026-06-04) contains exactly one change: "Upgrade wasmtime to v43" by a **first-time contributor** ([release](https://github.com/extism/extism/releases/tag/v1.30.0)). The odd version jumps (1.13 → 1.20 → 1.21 → 1.30) exist to signal wasmtime major bumps; wasmtime itself is at 49.0.1 ([crates.io API](https://crates.io/api/v1/crates/wasmtime)), so extism users are perpetually ~2–6 wasmtime majors behind.
- **Slowest-lane signals:** issue [#909 "Release 0.130.1?"](https://github.com/extism/extism/issues/909) has sat unanswered since 2026-07-17; releases are cut by a single core maintainer (@nilslice). Adoption is ~2% of raw wasmtime's recent downloads (196k vs 10.1M/90d) — a thin community compared to `mlua` (1.78M/90d) or `pyo3` (61.1M/90d) ([crates.io APIs](https://crates.io/api/v1/crates/mlua), [/pyo3](https://crates.io/api/v1/crates/pyo3)). The project is tied to Dylibso's commercial XTP tooling ([README §XTP](https://github.com/extism/extism/blob/v1.30.0/README.md)), so community growth is not the core business.

**Net:** maintained, zero direct RUSTSEC advisories against the crate (OSV query returned empty), but the bus factor and the wasmtime-following cadence are real constraints for a product that must ship security fixes on its own schedule.

## 2. The wasmtime pin is now a security-latency problem

`extism 1.30` declares `wasmtime = { version = "43", default-features = false, ... }`, `wasi-common = "43"`, `wiggle = "43"` ([runtime/Cargo.toml@v1.30.0](https://github.com/extism/extism/blob/v1.30.0/runtime/Cargo.toml)). Cargo resolves that to `^43` — **a host cannot `cargo update` its way to a newer wasmtime line without an extism release.**

Check the 2026 advisory record against SpecForge's locked 43.0.2 (all via [OSV](https://api.osv.dev/v1/query)):

| Advisory | What | Fixed in | 43.0.2 status |
| --- | --- | --- | --- |
| RUSTSEC-2026-0269 ([GHSA-vqjp-4c8c-hfgg](https://api.osv.dev/v1/vulns/RUSTSEC-2026-0269)) | **Filesystem sandbox escape** via trailing-slash/symlink paths; CVSS 4.0 `VC:H/VI:H` | 46.0.3 / 47.0.4 | **affected, no 43.x backport** |
| RUSTSEC-2026-0222 ([#910](https://github.com/extism/extism/issues/910), open) | Stores can mix up type indices between engines | 46.0.2 / 47.0.3 | **affected, no 43.x backport** |
| RUSTSEC-2026-0223 | Preemption/traps during bulk operations corrupt VM state | 46.0.2 / 47.0.3 | not affected (introduced 46.0.0) |

The practical exposure of 0269 depends on WASI preopens (SpecForge's wrapper passes no `allowed_paths` — `crates/specforge-extism/src/runtime.rs:87`), so this is latent rather than exploitable today. But the structural fact stands: **the only remediation path is waiting for Dylibso to cut an extism release**, a dependency of your sandbox for a fix to your sandbox. A hand-rolled wasmtime integration bumps wasmtime in a PR.

## 3. Known limitations and community complaints (from the issue tracker)

**Resource-limit semantics are coarse and partially leaky.**
- [#860](https://github.com/extism/extism/issues/860): the manifest `timeout_ms` **cannot interrupt an uninterrupted WASI sleep** (e.g. Python `time.sleep`). The timeout is epoch-based: a single global background thread calls `engine.increment_epoch()` ([timer.rs@v1.30.0](https://github.com/extism/extism/blob/v1.30.0/runtime/src/timer.rs)), and the store's `epoch_deadline_callback` just re-arms `Continue(1)` until the wall-clock deadline ([plugin.rs:941-963](https://github.com/extism/extism/blob/v1.30.0/runtime/src/plugin.rs)). Epoch ticks don't preempt WASI `poll_oneoff`-style waits — for R-6 determinism this means "bounded time" is only bounded when the guest cooperates.
- [#637](https://github.com/extism/extism/issues/637): still no way to manage timeouts from *inside* a host function (open since 2023-12).
- [#900](https://github.com/extism/extism/issues/900) and [#895](https://github.com/extism/extism/issues/895): no `memory_status()`, no exposure of allocated WASM memory — you cannot observe how close a plugin is to its `max_pages` limit, or budget per plugin (R-2 monitoring gap).

**Plugin lifecycle / hot reload.**
- [#890](https://github.com/extism/extism/issues/890): **memory leak in the plugin create/destroy cycle** — `extism_plugin_free` does not fully release memory (open since 2026-02). SpecForge's R-5 hot reload is exactly a create/destroy churn loop, and the wrapper holds `Mutex<HashMap<String, Plugin>>` instances it tears down on reload (`runtime.rs:19`).
- No reload/restart primitive exists at all; you rebuild a `Plugin` and re-run `__handshake`.

**WASI/platform coverage is frozen at preview 1.**
- [#666](https://github.com/extism/extism/issues/666): "runtime: wasi preview2" — open since **2024-01**, 23 comments. No WASI 0.2, no component model, no WIT anywhere in the host API.
- [#357](https://github.com/extism/extism/issues/357): WASI threads, open since 2023-05. [#791](https://github.com/extism/extism/issues/791) (Concurrency, 10 comments) — no story for concurrent calls into one plugin; one `Plugin` = one store, calls are `&mut self` (the wrapper's global mutex serializes everything).

**API/observability friction.**
- [#868](https://github.com/extism/extism/issues/868): no way to enumerate a plugin's exports — SpecForge probes exports by calling them and catching errors.
- [#740](https://github.com/extism/extism/issues/740): runtime error messages are undocumented; `Plugin::call` failures surface as flattened `anyhow` strings (the wrapper maps them to `"call_failed"` + string, `runtime.rs:142-146`).
- [#802](https://github.com/extism/extism/issues/802): the reserved `extism:host/env` namespace is a compat hazard; [#864](https://github.com/extism/extism/issues/864)/[#767](https://github.com/extism/extism/issues/767): `allowed_paths` internal structure churn.

## 4. Fairness: what 1.30 *does* provide (and SpecForge ignores)

The 2026 builder is richer than the audit record assumed ([plugin_builder.rs@v1.30.0](https://github.com/extism/extism/blob/v1.30.0/runtime/src/plugin_builder.rs)): `with_fuel_limit(u64)`, `with_cache_config`/`with_cache_disabled` (wasmtime's on-disk compilation cache), `with_wasmtime_config(Config)` escape hatch, coredump/memdump/profiling options, `compile()` → `CompiledPlugin` + `Plugin::new_from_compiled` (real AOT), a `Pool`/`PoolBuilder` instance pool, and `CancelHandle`. The manifest carries `timeout_ms`, `memory.max_pages`, `allowed_hosts`, `allowed_paths`, per-module `hash` verification ([manifest/src/lib.rs@v1.30.0](https://github.com/extism/extism/blob/v1.30.0/manifest/src/lib.rs)).

SpecForge's wrapper uses almost none of this: `_aot_cache_path` is ignored (C7-02), no timeout or `max_pages` is set (C7-10/C7-04), no `Pool` (C7-08), no fuel (R-6). **Several "extism limitations" in the audit are integration gaps, not upstream gaps.** That said, the escape hatch has a hard floor: `with_wasmtime_config` silently **overwrites** `async_support`, `epoch_interruption` semantics, `debug_info`, `coredump_on_trap`, profiler, tail-call/GC/function-references flags — so the pieces a modern embedding most wants (async, component model) are exactly the ones you cannot turn on.

## 5. What hand-rolled wasmtime gives that extism cannot

1. **Version control.** Track wasmtime 46/47 now; absorb RUSTSEC fixes in hours, not release-cadence months (§2). Also: SpecForge pulls `ureq 3.3.0` via extism's default `http`/`register-http` features (`Cargo.lock`) — an HTTP client and a remote-module loader the host never uses; raw wasmtime trims both.
2. **Component model + WIT.** Typed host/guest interfaces would replace the stringly `call_export` JSON protocol (C7-03) with a compiler-checked contract — structurally impossible on extism 1.30 (core modules + wasi-common p1 only).
3. **Async.** wasmtime `async` support (async host functions, `call_async`) is force-disabled by extism; the wrapper's host functions do blocking I/O inside sync calls (C14-03/C14-04 would become first-class fixable).
4. **Engine sharing and pooling.** extism builds a fresh `wasmtime::Engine` per compiled plugin ([plugin.rs:48,74](https://github.com/extism/extism/blob/v1.30.0/runtime/src/plugin.rs)) — no shared JIT code, per-plugin engines, pooling allocator benefits mostly forfeited. Hand-rolled: one `Engine`, one `Module` per extension (compile once, AOT-serialize for R-4 reproducibility), N cheap instances — the real fix for C7-08.
5. **Fine-grained preemption.** Native epoch-deadline callbacks (yield-based cooperative scheduling, per-call budgets), fuel with per-call differentiation, custom `ResourceLimiter` with telemetry — vs extism's one global timer thread, one wall-clock `timeout_ms`, and fixed `Continue(1)` semantics (§3).
6. **Typed host functions and linker control** (`TypedFunc`, scoped `Linker` defines, per-instance WASI state) instead of `Val`-marshalled functions pinned to extism's ABI and namespace conventions.
7. **Memory introspection** for limits/monitoring (`Store::memory` access, limiter hooks) — the exact asks in #895/#900.

**What you give up hand-rolling:** extism's manifest conveniences (`allowed_hosts`, var store, HTTP capability), multi-language PDKs (17 SDKs), the `extism-convert` ABI, and `CancelHandle`. For SpecForge these are mostly unused: plugins are self-authored Rust blobs and the only capability surface is three custom host functions (`host_context.rs:56-206`).

## 6. Requirement impact

- **R-2 (sandbox):** latent unpatched fs-escape advisory in tree + coarse limits + #860 → sandbox enforceability is weaker than raw wasmtime's, and upgrade latency is controlled by upstream.
- **R-5 (hot reload):** #890 lifecycle leak + no reload primitive = churn risk on the hottest path.
- **R-6 (determinism):** fuel exists but epoch-timeout cannot preempt non-cooperating guests; per-call budgeting not exposed.
- **R-4 (reproducibility):** `CompiledPlugin`/AOT and module `hash` exist but the wrapper uses neither.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 3
**One-line rationale:** The wasm/wasmtime substrate is the only option satisfying R-2/R-3/R-4 outright, but this evidence argues for keeping it *without* extism — or at minimum treating "bump wasmtime within extism" as a blocked dependency whose escape hatch is a hand-rolled wasmtime layer reusing the existing `WasmRuntime` port.

## Bottom line

**KEEP_WASM** (confidence 3/5) — extism 1.30 is alive but adds a wasmtime-major pin (currently carrying two 2026 advisories with no fix in its locked line, including a filesystem sandbox escape), coarse non-preemptive limits, a plugin-lifecycle memory leak that directly threatens hot reload, and no component-model/async path forward; the wasm substrate stays, but the extism layer is replaceable via the existing `WasmRuntime` port without changing the security model.
