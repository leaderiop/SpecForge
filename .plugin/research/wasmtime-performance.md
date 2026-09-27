# Research: Wasmtime 43 performance characteristics + Component Model/WIT readiness

> Assignment: instantiation latency, per-call overhead, memory footprint, AOT effectiveness
> (real benchmarks), and a production-readiness assessment of the Component Model + WIT
> roadmap for SpecForge's embedding story. Researched 2026-09-27.
> Local baseline: SpecForge embeds **extism 1.30.0 → wasmtime 43.0.2**
> (`.plugin/evidence.md` §1.1, `Cargo.lock`); wasmtime 43.0.0 released 2026-03-20
> ([release notes](https://github.com/bytecodealliance/wasmtime/releases/tag/v43.0.0)).
> Note: web_search was unavailable this session; all external claims are direct-read
> primary sources (arXiv, GitHub releases, docs.rs, Bytecode Alliance, zed.dev, dprint.dev, lib.rs).

## TL;DR

Wasmtime's numbers are strong where SpecForge needs them and irrelevant where it doesn't.
Pure **instantiation** (post-compile) is microseconds-to-tens-of-µs with KB-scale memory
(Lucet lineage, absorbed into wasmtime); the milliseconds in real cold-start measurements
are **Cranelift compilation**, which AOT serialization eliminates but SpecForge does not
actually use (`C7-02`: its "AOT cache" is a byte-copy). Per-call overhead is officially
"low-overhead" with a typed fast path, but no authoritative ns-figure exists in primary
sources — an open micro-benchmark, not a blocker. The **Component Model is
production-ready for 0.2** (enabled-by-default in wasmtime; Zed ships editor extensions
compiled to `wasm32-wasip2`) and **WASI 0.3.0 ratified 2026-06-11** makes async native to
components — but SpecForge's pinned 43.0.2 predates all of that, and its extism pin is the
gate, not wasmtime.

## 1. Instantiation latency — separate compile from instantiate

The literature consistently shows two regimes, and conflating them is the classic
embedder error:

- **Instantiate-only** (module already compiled): Lucet — the Fastly runtime whose AOT
  compilation and pooling allocator were later merged into wasmtime (EOL notice,
  [lucet README](https://github.com/bytecodealliance/lucet): "In mid-2020, the Lucet team
  switched focus to Wasmtime. We have added all of the features to Wasmtime which
  previously only Lucet had, such as AOT compilation and a pooling userfaultfd-based
  memory allocator") — reported **"instantiate WebAssembly modules in under 50
  microseconds, with just a few kilobytes of memory overhead"**, vs V8 at "~5 milliseconds
  and tens of megabytes" ([Fastly, 2019](https://www.fastly.com/blog/announcing-lucet-fastly-native-webassembly-compiler-runtime)).
- **Cold start including JIT compilation**: the 2025 Limes study (wasmtime-based runtime,
  Ryzen 7 5700U, 1000 iterations) measured `Component::new()` cold starts of **5.6 ms**
  (no-op), **16.9 ms** (Mandelbrot), **188.0 ms** (image-processing, dependency-heavy
  module) — versus Firecracker's flat ~30 ms pre-warmed / ~94–96 ms end-to-end
  ([arXiv:2509.09400](https://arxiv.org/abs/2509.09400), §4.2). Compilation cost scales
  with module complexity; instantiation cost does not. The same study shows
  serialize/deserialize reuse "reducing the initialization times by reusing the previously
  compiled Wasm modules" (§3).

For SpecForge: four vendored blobs totalling 1.4 MB (324–415 KB each) are
`PluginBuilder::build()`-compiled **per process invocation**
(`crates/specforge-extism/src/runtime.rs:80-97`), so `analyze` pays ms-scale compile time
×4 on every run; Limes puts the per-blob compile cost at roughly the no-op→Mandelbrot
range for this payload class. The fix is exactly C7-02: real precompile+deserialize. Note
the Limes guest used components + WIT; wasmtime 43 supports the same path for core
modules via `Module::serialize`/`deserialize`
([docs.rs/wasmtime](https://docs.rs/wasmtime/latest/wasmtime/), crate docs: modules
"can additionally be serialized … to later be deserialized quickly").

## 2. Per-call overhead

No authoritative nanosecond figure exists in the primary sources I could verify — treat
any single number you see (including Lucet-era marketing) as workload-specific. What is
verifiable:

- Wasmtime's stated design goal: "optimized for efficient instantiation,
  **low-overhead transitions between the embedder and wasm**, and scalability of
  concurrent instances" ([lib.rs/crates/wasmtime](https://lib.rs/crates/wasmtime)).
- The API exposes a fast path: `TypedFunc` is documented as "a more efficient calling
  convention" than dynamic `Func::call`; `Store` is "cheap to create and destroy"
  ([docs.rs/wasmtime crate docs](https://docs.rs/wasmtime/latest/wasmtime/index.html)).
- Interruption cost matters more than call cost for sandboxed plugins: fuel is
  documented as "significantly more expensive than epoch checks" in instrumentation
  overhead (same docs.rs source; Limes §3 chose epochs for exactly this reason).
- The Component Model makes an aggressive cross-boundary claim: in-process component
  composition "will reduce the time for calling other microservices from milliseconds to
  nanoseconds: six orders of magnitude"
  ([WASI 0.3.0 release notes](https://github.com/WebAssembly/WASI/releases/tag/v0.3.0)) —
  a roadmap claim, not a benchmark, but directionally consistent with the ABI's goal of
  cheap calls.

For SpecForge the per-call cost that dominates is not the wasm transition: it is
extism's `Plugin::call::<&[u8], Vec<u8>>` marshalling plus serde JSON round-trips per
export (`crates/specforge-extism/src/runtime.rs:140`), amortized over ~1.7k-entity
validation workloads (`.plugin/evidence.md` §1.5). A cheap, honest follow-up: pin the
handshake + `__pass_*` call latency with a criterion bench once, so the fleet stops
reasoning from vibes on this dimension.

## 3. Memory footprint

- Per-instance baseline: "just a few kilobytes of memory overhead" (Lucet/Fastly,
  above) — preserved in wasmtime via the pooled, lazily-committed allocator; the v43
  stability matrix references the pooling allocator as an existing subsystem (e.g.
  "shared memories aren't supported in the pooling allocator",
  [docs/stability-wasm-proposals.md @ v43](https://github.com/bytecodealliance/wasmtime/blob/v43.0.0/docs/stability-wasm-proposals.md)).
- Contrast class: V8 at "tens of megabytes" per instance baseline (Fastly, above) —
  relevant when weighing a `deno_core` TYPESCRIPT future for R-3 (single-binary host).
- Dependency weight (the footprint that actually shows up in SpecForge): the `wasmtime`
  crate itself is 6.5 MB / 112K SLoC, pulling ~13–24 MB / ~503K SLoC of dependencies
  ([lib.rs/crates/wasmtime](https://lib.rs/crates/wasmtime)); locally, wasmtime 43.0.2
  "dominates dependency weight (audit: 485→511 locked deps)"
  (`.plugin/evidence.md` §2). This is the real tax of KEEP_WASM, and it is build-time,
  not runtime.

## 4. AOT compilation effectiveness

AOT is first-class, old, and boring (the best kind):

- The embedding API's official mechanism is `Module::serialize` →
  `Module::deserialize` (docs.rs, above); the CLI ships a transparent on-disk cache of
  compiled artifacts (zstd-compressed, LRU-evicted,
  [docs/cli-cache.md](https://github.com/bytecodealliance/wasmtime/blob/main/docs/cli-cache.md)).
- Effectiveness evidence: Limes uses serialized-module reuse specifically to cut
  initialization latency (arXiv:2509.09400 §3); the broader literature review in the
  same paper cites Kjorveziroski & Filiposka (2023) finding "AOT compiled WebAssembly
  significantly reduces cold starts and improves execution" (§5).
- **Pulley** (portable bytecode interpreter for platforms where a JIT is unwanted) exists
  in-tree but its own README at v43 says it is "very much still a work in progress"
  with bytecode stability explicitly non-guaranteed
  ([pulley/README.md @ v43](https://github.com/bytecodealliance/wasmtime/blob/v43.0.0/pulley/README.md))
  — do not build SpecForge distribution on it.
- One artifact-management caveat: compiled-artifact caches are keyed to the producing
  engine — wasmtime's own cache system exists as engineering around exactly this
  (cli-cache.md describes a version-managed store with cleanup, "implementation detail
  and might change"). A real SpecForge `_aot_cache_path` (fixing C7-02) must key on
  wasmtime version + module hash or it becomes a correctness bug, not a cache.
- Perf hygiene is active, not stalled: a 2026 study diagnosed **six previously unknown
  performance issues in Wasmtime** via mutation-based inference across 12 real issues
  in three runtimes ([arXiv:2604.13693](https://arxiv.org/abs/2604.13693)) — i.e.
  perf bugs exist, get found, and get fixed upstream.

SpecForge status: the current `has_cached_module` checks a `.aot` file that
`instantiate` ignores (`crates/specforge-extism/src/runtime.rs:80-97,150-155` — C7-02).
The measured facts above say this is the single highest-leverage perf fix available in
KEEP_WASM, and it requires no runtime swap.

## 5. Component Model + WIT: production-ready?

**Verdict: production-ready at 0.2; 0.3 ratified and shipping in wasmtime ≥46; CM 1.0 is
the announced destination.** Timeline from primary sources:

- wasmtime v14 (2023-10-20) already carried the component API and WASI-preview2
  machinery — `wasmtime::component::Linker`, `bindgen!`, `wasi:http`, WIT resources
  ([v14.0.0 release notes](https://github.com/bytecodealliance/wasmtime/releases/tag/v14.0.0)).
- At v43 (2026-03-20): component-model is **enabled by default**, implementation
  "finished" with full test coverage, but the stability matrix honestly records it is
  **not yet phase 4** in standardization, C-API gaps remain, and component fuzzing is
  partial (stability-wasm-proposals.md @ v43, above). v43 also added WASIp3 snapshot
  support (rc-2026-03-15) and landed a wave of component-model-async fixes
  ([v43.0.0 notes](https://github.com/bytecodealliance/wasmtime/releases/tag/v43.0.0)).
- **WASI 0.3.0 ratified 2026-06-11**: "async is now native to WebAssembly Components"
  (`stream<T>`, `future<T>`, `async func` in the canonical ABI; start/finish dances
  deleted; the `network` resource deleted — network access granted via world imports);
  "Wasmtime 46 will ship WASI 0.3.0 with Component Model Async enabled by default";
  programs compiled for 0.3 "are guaranteed to keep working"
  ([WASI v0.3.0 release](https://github.com/WebAssembly/WASI/releases/tag/v0.3.0),
  [BA announcement](https://bytecodealliance.org/articles/WASI-0.3)). The BA has
  published "[The Road to Component Model 1.0](https://bytecodealliance.org/articles/the-road-to-component-model-1-0)".
- Shipped production users: **Zed** compiles its extension ecosystem to
  `wasm32-wasip2` — the component-model target — with wasmtime in-host
  ([zed.dev/docs/extensions](https://zed.dev/docs/extensions/developing-extensions.md)).
  **dprint** runs ~20 formatter-language plugins as sandboxed `.wasm` in a
  single-binary CLI ([dprint.dev/plugins](https://dprint.dev/plugins/)) — the closest
  shipped analog to SpecForge's shape. Fastly's edge ran the Lucet/wasmtime lineage at
  request-scale (Fastly 2019, above).
- Adoption pressure: `wasmtime` shows **3.54 M downloads/month, used in 2,298 crates**,
  220 releases with monthly minor + parallel patch lines (49.0.1/48.0.3 both 2026-09-24)
  ([lib.rs](https://lib.rs/crates/wasmtime)).

What it changes for SpecForge's embedding story, concretely:

1. **The hand-rolled protocol (C7-03) has a standards-grade replacement.** WIT + 
   `bindgen!` would type the 11 describe categories and 3 host functions, replacing
   stringly JSON-over-linear-memory with generated bindings and world-level versioning
   (the same path Zed took; see also `.agents/skills/…/048-alex-crichton.md` framing).
2. **Sandbox policy becomes structural.** WASI 0.3 grants network via world imports, not
   ambient resources — `SandboxPolicy`'s deny-by-default map naturally onto a world that
   simply does not import what it must not grant (`crates/specforge-wasm/src/sandbox.rs`).
3. **The gate is extism, not wasmtime.** extism 1.30.0 pins wasmtime ^43 with **no
   security backports on that line** (`.cargo/audit.toml:29-34`); extism main is on
   wasmtime 48 via PR #912 but unreleased (`.scratch/wayfinder/registry-trust/research-0269-clearing.md`).
   Notably, extism's README and releases show no first-class component-model plugin
   story — its model remains the memory ABI + PDKs, with its own XTP schema IDL
   ([extism README](https://github.com/extism/extism)) — while wasmtime 43 already
   exposes the full `wasmtime::component` API to embedders directly. If SpecForge wants
   components before an extism release, the extism layer is what becomes optional.
4. **Watch mode (R-5) and warm pools (C7-08)** benefit from the same mechanics as
   everyone else: EnginePool-as-ledger should become either real pre-instantiated
   instances or, simpler for a CLI, AOT-cached modules where instantiation is already
   µs-class — the pool may be vaporware worth deleting rather than building.

## Bottom line

**KEEP_WASM**, confidence **4/5** — wasmtime 43's measured profile (µs-class
instantiation with AOT, KB-class instances, ms-class compile that caching eliminates,
3.5M downloads/month and Zed/dprint production precedents) fits SpecForge's
single-binary, sandboxed, hot-reload requirements better than any embedded interpreter,
provided the real AOT cache (C7-02) ships and the protocol migrates to WIT once the
extism/wasmtime 46+ release path unblocks.
