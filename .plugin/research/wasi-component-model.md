# Research: WASI Preview 2/3, the Component Model + WIT — readiness, tooling, language support, timeline

**Analyst:** research-wasi-component-model · **Date:** 2026-09-27 · **For:** SpecForge plugin-runtime decision (KEEP_WASM vs LUA/PYTHON/TYPESCRIPT/MULTI)
**Method:** primary-source reads (wasi.dev, Bytecode Alliance articles, docs.rs, GitHub repos). Web-search provider was degraded for part of the session; every load-bearing claim below is grounded in a directly read primary source. Unverified inference is marked `[INFERENCE]`.

---

## 1. TL;DR

- **WASI 0.2 (Component Model 0.2) is 2.5 years into production** with strong backward-compatibility guarantees; the Bytecode Alliance states P1 modules and P2 components "still work" and platform providers depend on this (BA, "The Road to Component Model 1.0", 2026-06-08).
- **WASI 0.3 was ratified 2026-06-11** (native async: `async func`, `stream<T>`, `future<T>` in the canonical ABI; `wasi:io` removed). Wasmtime 46+ enables it by default; 0.3.1 shipped 2026-08-11 with `map<K,V>` and `implements`/`external-id` annotations; 0.3.x patches run every 2 months through 2027.
- **Component Model 1.0 has no committed date.** It requires a lazy ABI (blocked partly on LLVM multivalue C-ABI upstreaming), native implementation in two browser engines (Mozilla experimenting, Chrome has an open evaluation issue), and a "good parts only" spec rewrite. Realistically 2027+.
- **Non-Rust component authoring works today for WASI 0.2 worlds**: Rust (mature), TS/JS via jco + componentize-js (usable, still self-labeled experimental, ~8 MB SpiderMonkey per component), Python via componentize-py (working, CPython bundled into each artifact), Go via componentize-go (new, Go 1.25.5+) and TinyGo. 0.3-native async bindings are rolling out over the coming months (BA, June 2026).
- **The entire multi-language investment of the wasm ecosystem lands on the component side.** An embedded interpreter (Lua/CPython/V8) caps plugins at exactly one language forever; KEEP_WASM inherits every future guest toolchain for free.
- **Important refinement for SpecForge:** the flexibility is in *Wasmtime + Component Model*, not in *Extism's core-wasm PDK model*. Extism today documents only core-wasm PDKs and no Component Model support. The upgrade path — define a WIT world, use `wasmtime::component` (stable, default-on feature) — directly fixes audit findings C7-03 (no IDL), C7-02 (real `Engine::precompile_component` AOT), C7-08 (`InstancePre`), C7-10 (enforceable traps/epoch), and gives C7-11 a single convergence target.

---

## 2. State of the standards and the timeline

### 2.1 Release history and schedule (wasi.dev/roadmap, read 2026-09-27)

| Milestone | Date | Status |
| --- | --- | --- |
| WASI 0.2.0 (CM 0.2, first component-based WASI) | Jan 2024 | Shipped; twelve patches through 0.2.12 |
| WASI 0.3.0 **ratified** (WASI Subgroup vote) | **2026-06-11** | Shipped, stable — "programs you compile for it today are guaranteed to keep working" |
| WASI 0.3.1 (`map<K,V>`, `implements`, `external-id`) | 2026-08-11 | Shipped |
| 0.3.2 … 0.3.9 | Oct 2026 – Dec 2027 | Planned release train, every 2 months |
| Component Model 1.0 (formal spec) | **undated** | Five workstreams in flight (below) |
| WASI 1.0 | after CM 1.0 | Explicitly "follows from and depends on" CM 1.0 |

Runtime support: **Wasmtime 46+ enables WASI 0.3 and `component-model-async` by default**; Wasmtime 43–45 implement the `0.3.0-rc-2026-03-15` snapshot behind flags; jco supports 0.3 with default-on release coming; conformance runs via shared `wasi-testsuite` on Wasmtime and jco across Linux/macOS/Windows (wasi.dev/releases/wasi-p3). SpecForge pins wasmtime 43.0.2 (via extism 1.30.0) — one major-line behind the 0.3-default release.

### 2.2 What WASI 0.3 actually changed (wasi.dev + BA article)

- `async func`, `stream<T>`, `future<T>` are **first-class in the canonical ABI**; the runtime owns the single event loop shared by all components ("sandwich problem" solved — async now composes across component boundaries).
- `wasi:io` is **removed entirely**; `wasi:http` restructured (`proxy` world → `service`/`middleware` worlds, direct in-process component chaining); sockets consolidated; near-mechanical migration otherwise.
- Bindings generators emit **idiomatic native async**: `async fn` (Rust), `Promise` (JS), coroutines (Python), and Go's goroutines map as **stackful** coroutines onto the same ABI (BA explicitly designed for both).
- Migration from 0.2 is **not required**: `wasmtime serve` runs 0.2 and 0.3 components side-by-side, and implementations may polyfill 0.2 in terms of 0.3. Components can link across compatible versions via canonical interface names — though "not all tools support this version-aware linking yet" (wasi.dev).

### 2.3 The road to Component Model 1.0 (BA article, 2026-06-08 — Luke Wagner/Alex Crichton)

Five workstreams, none with a date:

1. **Lazy ABI** (replaces eager `cabi_realloc` copying): ships as opt-in in a 0.3.x release, becomes default at 1.0 with an adapter tool. Longest lead item: **LLVM doesn't yet support multivalue at the C ABI level** (tool-conventions PR #268); LLVM releases every 6 months, Rust takes ~9 weeks to stable. Also bundles error-context in every result error case.
2. **Browser path**: CM 1.0 formally requires ≥2 browser engines. Mozilla presented performance results (Ryan Hunt, hacks.mozilla.org 2026-02) and jco's transpiled components now emit a `"use components"` telemetry marker (renamed from `"use jco"` in jco 1.16.8); **Chrome/V8 opened an evaluation issue (chromium #474661098)**. "Not commitments."
3. **Easier implementation**: 1.0 spec = "good parts" of P3 only; guest + host C-ABIs via `wit-bindgen` headers; a proposed `lower-components` tool to "smash" components into single core modules.
4. **Ecosystem**: docs push, stable P3 support upstream in Rust/Tokio/LLVM/CPython (tracked in `awesome-wasm-components`), `wac` composition, `wkg` OCI publishing, record/replay WAVE debugging.
5. **WIT expressivity gaps** (relevant to any long-lived IDL — see §6): optional imports, callbacks, resource/function subtyping, enhanced import names, getters/setters, `map<K,V>` (landed 0.3.1), runtime instantiation. "Not all of these will land before 1.0."

Plus: **cooperative threads** (pthreads in wasi-libc largely done, LLVM patches landed, Wasmtime behind a flag) and **stream splicing** ship in early 0.3.x follow-ups.

**Timeline synthesis for planning:** 0.2 is safe to build on now (with compat guarantees); 0.3-native guest toolchains land through late 2026–2027; a formal 1.0 is a 2027+ event at the earliest `[INFERENCE from stated dependencies: LLVM cadence + 2 browser engines]`. Crucially, the compatibility strategy ("P1 modules still work, P2 components still work … maintained since P1 using semver, side-by-side implementations, and Wasm-to-Wasm adapters") means building on 0.2/0.3 now is not a bet that gets stranded at 1.0.

---

## 3. Production readiness

- BA (Luke Wagner, CM 1.0 article): the Component Model and WASI are "**already heavily used in production**"; platform providers and embedders give strong backwards-compat guarantees; P1/P2 artifacts keep working.
- Wasmtime itself: Cranelift-backed, 24/7 Google OSS-Fuzz coverage, published stability policy (`docs.wasmtime.dev/stability-release.html`), Spectre mitigations, formal-verification collaboration. `wasmtime::component` is **no longer experimental**: the module is "Available on crate features `component-model` and `runtime` only… which is enabled by default" (docs.rs, latest). Async-specific API (`Accessor`, streams/futures, concurrent calls) is gated behind the separate `component-model-async` feature.
- Guest-side component toolchains in production use today: jco/componentize-js and componentize-py are BA projects with release automation and example suites; StarlingMonkey (the SpiderMonkey-based JS runtime underlying componentize-js) targets WASI 0.2.0 builtins and passes Web Platform Tests subsets.
- Registry story: **warg (the dedicated wasm registry) is gone** — `github.com/warg-systems/warg` returns 404 (org deleted; BA wound it down in 2025 `[INFERENCE from repo removal + ecosystem direction]`). The mainline is **OCI registries via `wkg`** (wasm-pkg-tools): `wkg get/publish`, `wkg.toml`/`wkg.lock` pinning, digest-pinned pulls, `/.well-known` registry metadata, OCI annotation conventions. SpecForge's own signed registry (R-4) is unaffected either way — and a useful verifiability trick: **a component binary embeds its WIT package**, so `wasm-tools component wit <blob>` extracts the exact interface a plugin implements, letting the registry verify/display the contract, not just a hash.

**Net:** the host side (Wasmtime + `wasmtime::component` + `bindgen!` + wasm-tools) is production-grade *now*. The guest-toolchain side is production-grade for Rust, usable-with-caveats for TS/JS and Python, and brand-new for Go.

---

## 4. Tooling maturity (host-side, what SpecForge would touch)

| Tool | Role | Maturity signal |
| --- | --- | --- |
| `wasmtime::component` | Embedding API: `Component`, `Linker`, `bindgen!`, `TypedFunc`, `InstancePre` | Feature **enabled by default**; mirrored on the core API; `bindgen!` generates typed Rust host bindings from the same WIT the guests use |
| `Engine::precompile_component` / `Component::deserialize` | True AOT: serialize compiled artifact once, mmap/load with zero compile at load time | Verified in docs.rs — directly replaces SpecForge's C7-02 "byte-copy `.aot` cache" |
| `InstancePre` | Pre-instantiation (imports pre-supplied, instantiation cheap) | Verified in docs.rs — the real implementation of the C7-08 "warm engine" story |
| `wasm-tools` | Component binary format, `component new` (wrapping + adapters), `component wit` (extract embedded WIT) | The reference implementation; preview1→0.2 adapter modules published with each Wasmtime release |
| `wit-bindgen` | Guest bindings: **Rust, C, C++, C#, Go in-repo**; D crate present; more via ecosystem | 1.4k★ BA project; P3 async bindings work in flight |
| `jco` (+ `componentize-js`) | JS/TS: transpile to JS+core-wasm glue (browser), or componentize via SpiderMonkey/StarlingMonkey embedding (~8 MB/component, Wizer pre-init, optional weval AOT) | jco supports WASI 0.3; componentize-js README: "**experimental project, no guarantees**"; async exports syncify via in-component event loop; **async imports blocked pending CM async** (now unblocked by 0.3 — tool support landing) |
| `componentize-py` | Python app → component; bundles CPython; requires Python 3.10+ at **build** time only | Active BA project; **P3 examples already in-tree** (`cli-p3`, `http-p3`, `tcp-p3`, `streams_and_futures` tests); known limitation: top-level imports only (issue #23) |
| `componentize-go` | Standard Go → component (generates `//go:wasmexport` bindings; **requires Go 1.25.5+**) | Official component-docs path for Go; BA project; P3 examples (goroutines as stackful coroutines at ABI boundary) |
| `wkg` / wasm-pkg-tools | Publish/fetch components + WIT packages from **OCI** registries; `wkg.lock` | Active; successor direction after warg's removal |
| `wac` | Component composition/linking language | Ecosystem stage per CM 1.0 article |

---

## 5. Language support matrix — "when can plugins be authored in non-Rust?"

The question has three answers depending on which substrate SpecForge sits on.

### 5.1 Today, inside the *current* Extism runtime (core wasm + PDK ABI)

Extism ships PDKs for **Rust, JS, Python, Go, Haskell, AssemblyScript, .NET/C#, C, C++, Zig** (extism README, read 2026-09-27) and dylibso's xtp-bindgen generates plugin scaffolding from a schema for TS/Go/Rust/Python/C#/Zig/C++. So strictly speaking, non-Rust authoring *against Extism* exists today. Caveats that matter to SpecForge: each PDK has its own memory/JSON conventions (no shared typed IDL — C7-03 persists), interpreter-language guests embed their engine into the blob (large artifacts), and maintenance depth is uneven across PDKs `[INFERENCE: based on ecosystem pattern; per-PDK health not individually audited]`. This is the status-quo multi-language story; it does not improve with the ecosystem.

### 5.2 The Component Model path (the strategic one)

| Language | Toolchain | Status (2026-09) | 0.3-native async |
| --- | --- | --- | --- |
| **Rust** | `wit-bindgen` (guest) + `wasmtime::component::bindgen!` (host) | **Mature, production** — the reference language of the ecosystem | In progress |
| **TypeScript/JS** | `jco` + `componentize-js` (SpiderMonkey embedding ≈ **8 MB/component**, Wizer snapshot → fast cold start, optional weval AOT) | **Usable today for 0.2 worlds**; README still says experimental, no stability guarantees | jco supports 0.3; componentize-js async-import support pending |
| **Python** | `componentize-py` (CPython bundled into artifact; build needs local Python 3.10+) | **Working today for 0.2 worlds**; artifacts large (interpreter embedded `[INFERENCE: tens of MB, mirroring the JS 8 MB engine + stdlib pattern]`); top-level-import limitation | **P3 examples already shipping in-repo** |
| **Go** | `componentize-go` (std Go, `go:wasmexport`, **Go ≥ 1.25.5**) or TinyGo | **Real but brand-new** — Go 1.24 (Feb 2025) only added the raw `go:wasmexport` + reactor primitives; no component ABI in the Go stdlib; the BA toolchain is the component story | Goroutines map to stackful CM async (BA-highlighted design win); P3 examples in-repo |
| C/C++ | `wit-bindgen` c/crate + wasi-sdk | In-repo, functional | C listed in BA's in-progress async bindings |
| C#/.NET | `wit-bindgen` csharp (incl. async support files in-tree) | In-repo | Listed in BA's in-progress set |
| Others (Kotlin/Wasm, Teavm-Java, D…) | ecosystem projects | D crate present in wit-bindgen; others community-maintained `[INFERENCE]` | — |

**Answer to "when":** For **TS/JS and Python, non-Rust component authoring is available now** against WASI 0.2 worlds — good enough to prototype a second-language plugin path this quarter, with honest caveats (8 MB+ artifacts for JS, larger for Python, experimental labels). **Go** became practical only in 2025–2026 and should be treated as early-adopter. **0.3-native async bindings across all of these land through late 2026–2027** (BA: "guest toolchains next… coming weeks and months", June 2026). **Formal CM 1.0: 2027+**, and nothing in the plan breaks 0.2/0.3 artifacts before then.

### 5.3 The embedded-interpreter alternatives (LUA/PYTHON/TYPESCRIPT options)

Fixed at one language by construction. The component ecosystem's answer to "author in language X" is a *guest toolchain for X targeting the same world* — a mechanism the embedded-interpreter options structurally cannot inherit. Every new language demand under LUA/CPython/V8 embedding is a new integration project; under CM it is (eventually) someone else's release notes.

---

## 6. What this means for SpecForge (mapping to constraints and audit)

**The workload fits the component model unusually well.** SpecForge plugins are pure-compute guests: manifest metadata + validation/passes/collectors receiving a graph snapshot and returning diagnostics. They need **no WASI ambient capability at all** — R-2 is satisfied more strongly than Extism's configurable sandbox by a world with **zero imports beyond the typed host interface** (canonical ABI only; nothing to misconfigure, C7-04's `file_system_access` allow-by-default becomes structurally impossible). Traps are scoped to one call (a failing rule aborts itself, not the host), wasmtime epoch/fuel make the C7-10 `max_execution_ms` promise enforceable, and R-6 determinism is preserved (single-threaded, fuel-bounded, snapshot-comparable).

**WIT is the missing IDL (C7-03).** The current `call_export(name, export, &payload)` stringly JSON protocol — where drift already happened — is exactly the problem WIT exists to solve. A `specforge:extension@1.0.0` world (`describe`, `validate`, `run-pass`, `collect` exports; entity-graph types) gives: compile-time-checked host and guest bindings from one source of truth (`bindgen!` host-side, `wit-bindgen` guest-side), version-evolvable interfaces, and embedded-in-binary contract extraction for the registry (R-4 verifiability). This converges C7-11's three parallel implementations onto one definition: the world *is* the plugin contract; the manifest JSON becomes generated code from it, not a hand-maintained mirror.

**Audit-gap fixes are one wasmtime-version bump away, not a rewrite.** `Engine::precompile_component` (C7-02), `InstancePre` (C7-08), epoch interruption (C7-10). Extism is not load-bearing for these — `specforge-wasm`'s 9.4k LOC already implements its own sandbox/cache/pool/lifecycle, and wasmtime is already in the tree; dropping the Extism layer for direct `wasmtime::component` removes a dependency boundary rather than adding one `[INFERENCE on LOC, grounded in evidence.md's architecture description]`.

**Honest costs and counter-evidence:**

1. **Per-call overhead is currently worse than core-wasm.** Component-model calls carry async task infrastructure: Nick Fitzgerald measured **~3.5× overhead on purely synchronous call paths** (BA CM 1.0 article); the fix is tracked (wasmtime#12311) for after P3, and the recursion check was already removed during P3. Mitigation for the 1.7k-entity graph workload: design chunky world functions (pass entity arrays in one call, return diagnostic lists) so per-entity guest calls never happen — which the typed world makes natural, unlike the current per-export stringly calls.
2. **Dynamic-language guests are heavy per-component** (≈8 MB SpiderMonkey; CPython bundled for Python) with engine sharing still a plan ("loaded as a shared library of the components" — componentize-js README). Registry blobs grow from ~400 KB to multi-MB for non-Rust guests. Acceptable for R-4 (still verifiable artifacts), but real.
3. **componentize-js self-labels experimental**; Go's componentize-go requires a very recent Go. The mature-now tier is Rust-only. If third-party non-Rust authoring is a day-one requirement, it isn't — it's a 2026–2027 ramp.
4. **WIT evolution is not solved.** Function subtyping (adding params without breaking) is called out as "currently painful in practice" (CM 1.0 article). SpecForge's world needs deliberate versioning design from day one — which is also true of the current JSON protocol, minus the tooling.
5. **CM 1.0 is undated** — but the compat guarantee (0.2 components keep running; adapters; semver) means the hedge for betting now is cheap, and it is the *same* hedge wasmtime's production embedders already run.

**Hot reload (R-5):** unchanged in shape — recompile/deserialize the artifact and re-instantiate; wasmtime's `deserialize_file` path makes it cheap. For dynamic-language guests, componentize's Wizer pre-init happens at *build* time, so watch-mode re-runs stay artifact-swap fast `[INFERENCE grounded in the documented build/runtime split]`.

---

## 7. Is the component-model future what makes KEEP_WASM most flexible?

Yes — with a precise formulation:

- The multi-language future of plugins is being built **specifically as component-model guest toolchains** (Rust/TS/JS/Python/Go/C/C++/C#), funded by the Bytecode Alliance, Mozilla, and now visibly pulling Chrome and CPython upstream ("CPython has first steps underway" — CM 1.0 article). No other plugin substrate has a comparable multi-vendor investment pointed at it.
- **KEEP_WASM-as-Extism-status-quo does not capture this.** Extism's PDK model is core-wasm, pre-component, and its README documents no component support. The flexibility argument therefore justifies a *refined* KEEP_WASM: **stay on Wasmtime, adopt the Component Model + a WIT world, drop the Extism/PDK layer** — WASI 0.3 is one major version ahead (46 vs SpecForge's 43), and the wasmtime component API is default-on, so this is an in-tree evolution, not a platform change.
- Against LUA/PYTHON/TYPESCRIPT: those options buy authoring ergonomics *today* in exactly one language, at the cost of permanent interpreter-embedding and losing the entire component ecosystem's trajectory. Against MULTI-as-scrap: the component model *is* the principled MULTI — one world, many guest languages, one sandbox, one registry artifact format.

The falsifier to watch: if `componentize-js`/`componentize-py` stall (still "experimental"/import-limited in 2027) or CM 1.0 misses its browser-engine requirement, the non-Rust promise defers but KEEP_WASM's Rust-first core remains correct — the downside is bounded, which is what makes the bet rational.

---

## 8. Sources (all primary, read 2026-09-27)

- WASI 0.3 launch — Bytecode Alliance: https://bytecodealliance.org/articles/WASI-0.3 (ratification 2026-06-11; Wasmtime 45/46; jco; guest toolchains next; componentize-go example)
- WASI 0.3 release notes — wasi.dev: https://wasi.dev/releases/wasi-p3 (async primitives; `wasi:io` removed; worlds; Wasmtime 43–46 support matrix; 0.3.1)
- WASI roadmap — wasi.dev: https://wasi.dev/roadmap (release train through 2027; 0.3.x feature candidates; polyfill policy)
- Road to Component Model 1.0 — Bytecode Alliance (2026-06-08): https://bytecodealliance.org/articles/the-road-to-component-model-1-0 (five workstreams; lazy ABI; LLVM multivalue blocker; browser requirements; Mozilla/Chrome signals; ~3.5× sync overhead + wasmtime#12311; WIT expressivity gaps; production-use statement)
- Go: `go:wasmexport` + reactor — Go Blog (2025-02-13): https://go.dev/blog/wasmexport (stdlib primitives only; limitations)
- Go component path — Component Model book: https://component-model.bytecodealliance.org/language-support/building-a-simple-component/go.html (componentize-go; Go ≥ 1.25.5; `go:wasmexport`-based bindings)
- componentize-py — https://github.com/bytecodealliance/componentize-py (CPython bundling; Python 3.10+ build-time; P3 examples; issue #23 import limitation)
- componentize-js — https://github.com/bytecodealliance/componentize-js (experimental label; ~8 MB SpiderMonkey embedding; Wizer; weval AOT; async-imports pending CM async)
- StarlingMonkey — https://github.com/fermyon/StarlingMonkey (WASI 0.2.0 builtins; component event loop; WPT suite)
- wit-bindgen — https://github.com/bytecodealliance/wit-bindgen (guest languages: Rust/C/C++/C#/Go in-repo; component-creation flow; preview1 adapters)
- wasmtime::component docs — https://docs.rs/wasmtime/latest/wasmtime/component/index.html and struct.Component.html (feature `component-model` default-on; `bindgen!`; `precompile_component`; `deserialize`; `InstancePre`; `component-model-async` gating)
- wkg / wasm-pkg-tools — https://github.com/bytecodealliance/wasm-pkg-tools (OCI publishing/fetching; wkg.lock; digest pinning; registry metadata)
- warg — https://github.com/warg-systems/warg (HTTP 404 — project removed)
- Extism — https://github.com/extism/extism (PDK list; xtp-bindgen; no Component Model support documented)
- Wasmtime — https://wasmtime.dev/ (stability policy, fuzzing, language embeddings)

## Bottom line

WASI 0.2 has been production-stable since early 2024; WASI 0.3 (native async) was ratified in June 2026 and ships by default in Wasmtime 46; formal Component Model 1.0 is a 2027+ milestone that will not strand 0.2/0.3 artifacts. Non-Rust component authoring is real today for TS/JS (jco/componentize-js, ~8 MB guests) and Python (componentize-py, CPython-bundled), and new but BA-backed for Go (componentize-go, Go ≥ 1.25.5), with 0.3-native bindings rolling out through 2027. The decisive fact for SpecForge: the whole ecosystem's multi-language investment lands on the Component Model — the one substrate KEEP_WASM already sits on — while an embedded interpreter caps plugins at one language permanently. The correct form of KEEP_WASM is "stay on Wasmtime, adopt Component Model + WIT world, retire Extism/PDK": that single move fixes C7-03 (IDL), C7-02 (`precompile_component`), C7-08 (`InstancePre`), C7-10 (epoch/fuel), strengthens R-2 (import-free world), and inherits every future guest language as toolchains mature. Watch item: per-call overhead (~3.5× until wasmtime#12311 lands) → design chunky, batch-shaped world functions.

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The component-model roadmap is the only plugin substrate with durable multi-language momentum — and it is an in-place evolution of the Wasmtime-based runtime SpecForge already has; embedded interpreters cannot inherit it.
