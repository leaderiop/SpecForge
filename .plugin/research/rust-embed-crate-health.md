# C5 Research — Rust Embedding Crate Health (maintenance & risk)

**Analyst:** research-rust-embed-crate-health · **Data pulled:** 2026-09-27 (live from crates.io API v1 and GitHub API; release notes read from GitHub releases feeds)
**Scope:** maintenance status of the seven Rust crates on the runtime shortlist — mlua, PyO3, rquickjs, deno_core, boa_engine, extism, wasmtime — feeding criterion **C5 (Rust embedding maturity & maintenance risk)**.

## 1. The numbers (all live, 2026-09-27)

Downloads are crates.io counters; "recent" = trailing 90 days. "Open issues" is issues-only (PRs excluded, via GitHub search API); "bug-labeled" counts open issues carrying the repo's own bug label (each repo's taxonomy: `bug` / `A-Bug` / `kind:bug` / `bugfix`).

| Crate | Newest | Last release | Cadence (last 5 releases) | Downloads 90d | Downloads total | Open issues | Bug-labeled | Repo last push | Stars |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **wasmtime** | 49.0.1 | 2026-09-24 | monthly majors + patch stream (49.0.0 Sep 21, 48.x Sep 10/24, 48.0.0-rc1 Sep 5) | 10,176,664 | 37,280,656 | 754 | 115 (`bug`) | 2026-09-26 | 18.7k |
| **pyo3** | 0.29.2 | 2026-08-05 (0.29.0 Jun 11) | minor every ~2 months, patches between | 61,515,183 | 260,631,552 | 294 | 0 (`bugfix` label; backlog is design-labeled — see §2) | 2026-09-25 | 16.2k |
| **deno_core** | 0.412.0 | 2026-09-16 | every 2–5 weeks, synced to Deno (0.408 Jul 15 → 0.412 Sep 16; Deno 2.9.x current) | 1,721,223 | 8,090,751 | 1,252 ⚠️ whole-Deno monorepo | 126 ⚠️ monorepo-wide `bug` | 2026-09-25 | 108.5k (Deno) |
| **rquickjs** | 0.14.0 | 2026-09-18 | accelerated: 5 releases in 4 months (0.12.0 May 27 → 0.14.0 Sep 18) | 2,070,533 | 4,564,865 | 55 | 7 (`kind:bug`) | 2026-09-25 | 1.0k |
| **mlua** | 0.12.1 | 2026-08-29 | ~2 majors/yr + patches (0.11.6 Jan → 0.12.0 Jul → 0.12.1 Aug) | 1,785,292 | 7,093,846 | 49 | 0 (`bug` label exists, nothing currently carrying it) | 2026-09-20 | 2.9k |
| **boa_engine** | 0.22.0 | 2026-08-28 | slow: ~1 major/yr + rare patches (0.20.0 Dec 2024 → 0.21.0 Oct 2025 → 0.22.0 Aug 2026) | 1,597,247 | 5,087,682 | 160 | 23 (`A-Bug`) | 2026-09-26 | 7.6k |
| **extism** | 1.30.0 | 2026-06-04 | sparse: 1.12 Jul 2025 → 1.13 Nov 2025 → 1.20/1.21 Mar 2026 → 1.30 Jun 2026 | 196,722 | 714,289 | 41 | 0 (`bug` label exists, unused) | 2026-09-02 | 5.8k |

Sources per row: [crates.io/crates/<name>](https://crates.io) API (`/api/v1/crates/<name>` and `/versions`); GitHub repo + search APIs for issues; [boajs.dev](https://boajs.dev) for conformance; [extism releases](https://github.com/extism/extism/releases) for release-note content.

## 2. Per-crate reading

**wasmtime** — the industrial benchmark. Monthly version trains with rc's and patch trains on two lines at once (48.x and 49.x shipping the same week), Bytecode Alliance staff, continuous fuzzing (dedicated `fuzz-bug` label). 754 open issues / 115 bug-labeled is the *largest raw backlog on the list*, but it is a throughput artifact, not neglect: the tracker covers Cranelift, Winch, and the component model, and issues are triaged in public with LTS branches back-patched. Cost of this velocity for embedders: a new major roughly every 4–5 weeks, so pinning is mandatory and downstreams age quickly. ([crates.io/crates/wasmtime](https://crates.io/crates/wasmtime), [github.com/bytecodealliance/wasmtime](https://github.com/bytecodealliance/wasmtime))

**pyo3** — the highest-traffic crate here by an order of magnitude (61.5M downloads/90d, 260M lifetime). Very active (pushed daily-ish, 0.29.x train current), but the 294-issue backlog is *not bug rot*: top labels are `needs-design` (33), `documentation` (22), `needs-implementer` (10), `1.0-candidate` (9), `Unsound` (5). The 5 open `Unsound` issues and the still-absent 1.0 are the honest maintenance caveats; the embedding-for-distribution story (shipping CPython) remains the fragile part, which is a design fact, not a maintenance one. ([github.com/pyo3/pyo3](https://github.com/pyo3/pyo3))

**deno_core** — maintained by the Deno core team at Deno's cadence: 0.408→0.412 between Jul 15 and Sep 16 2026, riding Deno 2.9.x. Caveats: (a) its issue numbers are un-isolatable — the 1,252 open / 126 bug-labeled figures are for the whole [denoland/deno](https://github.com/denoland/deno) monorepo (CLI, std, node compat), not the crate; (b) 412 minor versions in ~5 years means the embedding API churns constantly — third-party docs rot fast; (c) it drags all of V8. Maintenance risk is low; API-stability and weight risk is not. ([crates.io/crates/deno_core](https://crates.io/crates/deno_core))

**rquickjs** — the surprise: after years of sparse releases, 5 ships in 4 months (May–Sep 2026), repo pushed Sep 25, downloads accelerating (2.07M/90d vs 4.56M lifetime). Backlog is feature-heavy (31 `kind:feature` vs 7 `kind:bug`), with telling side labels: 7 `topic:upstream` (engine defects filed against QuickJS/quickjs-ng, outside this crate's control) and 6 `topic:safety`. Single primary maintainer (DelSkayn) — bus factor 1 is the main risk. ([github.com/DelSkayn/rquickjs](https://github.com/DelSkayn/rquickjs))

**mlua** — steady, mature, low-drama. 0.12.x current (Aug 2026), pushes within the last week, and a 49-issue tracker with zero open bug-labeled items and 47 of 49 issues unlabeled — bugs get fixed or closed rather than piling up. This is what a stable FFI wrapper around a frozen C core (Lua 5.4/LuaJIT) looks like: low release pressure because the substrate barely moves. ([github.com/mlua-rs/mlua](https://github.com/mlua-rs/mlua))

**boa_engine** — alive (pushed Sep 26, 0.22.0 Aug 2026, 7.6k stars) but the slowest cadence on the list: ~1 major/yr, with 0.20.0→0.21.0 taking 10 months. The tracker is the most conformance-shaped of the scripting options: 23 open `A-Bug`, plus 40 enhancements, 16 API, 13 `C-Intl` (ECMA-402 i18n gaps), 9 performance. The project itself states it passes "more than 90% of ECMAScripts test262" ([boajs.dev](https://boajs.dev)) — meaning ~10% of the spec test suite still fails, and its own benchmarks page exists because perf is the known gap. Healthy for what it is (a pure-Rust engine), but it is the least battle-tested JS engine of the three TS-path options.

**extism** — the caution flag for KEEP_WASM. Alive (repo pushed Sep 2, 5.8k stars) but releases are sparse — 4 in 19 months — and thin: v1.21.0 and v1.30.0 are each a single "upgrade wasmtime" PR; v1.20.0 was one pool thread-safety fix. As of 2026-09-27 the crate on crates.io (1.30.0, wasmtime 43) is **six majors behind** wasmtime (49), and the fix — "Upgrade to Wasmtime 48 (LTS)" ([PR #912](https://github.com/extism/extism/pull/912)) — sits in the Sep 2 dev-build tag, unreleased. This is a thin wrapper layer with a small team (nilslice/zshipko + community) perpetually chasing a monthly-major dependency; exactly the "extism lags wasmtime" risk SpecForge already lives with (locked at extism 1.30.0 → wasmtime 43.0.2). ([crates.io/crates/extism](https://crates.io/crates/extism), [releases](https://github.com/extism/extism/releases))

## 3. Which are most actively maintained?

1. **wasmtime** — unmatched: monthly trains, dual-line patching, fuzzing, BA-funded staff. The gold standard, at the price of version churn.
2. **pyo3** — extremely active with the largest user base; backlog is roadmap (`1.0-candidate`, `needs-design`), not rot.
3. **deno_core** — maintained at Deno-core-team tempo (release every 2–5 weeks); caveat = monorepo opacity + API churn.
4. **rquickjs** — currently in its most active period ever, but bus factor 1.
5. **mlua** — quiet-steady; low cadence reflects a stable substrate, not neglect. Tracker cleanliness is the best on the list.
6. **boa_engine** — active but slowest-cadence scripting option; conformance/perf work remains.
7. **extism** — alive but the thinnest maintenance margin: dependency-chase releases, months-long gaps, lagging wasmtime by 6 majors as of today.

## 4. Which have the most bugs/limitations filed?

Raw open counts: **deno_core (1,252/126 bug)** and **wasmtime (754/115 bug)** dominate — but both numbers are monorepo/platform-wide and reflect throughput; neither indicates poor health. The honest per-crate bug picture:

- **boa_engine** files the most *visible crate-level defects* among the scripting engines (23 open `A-Bug` + 13 i18n conformance gaps + a ~10% test262 miss the project itself publishes).
- **rquickjs** shows 7 open `kind:bug` plus 6 `topic:safety` and 7 `topic:upstream` — the upstream split matters: engine-level defects live in quickjs-ng, outside the Rust crate's control.
- **pyo3** reports almost no bug-labeled rot, but carries 5 open `Unsound` items — low count, high severity class.
- **mlua** and **extism** show essentially zero labeled-open bugs (small, fast-closening trackers); extism's "limitations" are structural (thin layer over a fast-moving dependency) rather than tracker-visible.

**Method caveat:** repos use different label taxonomies and labeling discipline (mlua: 47/49 open issues unlabeled; extism: 33/41; pyo3: 202/292) — so bug-label counts *understate* true defect volume for the loosely-labeled repos and are only comparable as orders of magnitude. GitHub unauthenticated APIs; counts are point-in-time 2026-09-27.

## 5. Bearing on C5 for SpecForge

- The current stack's *engine* (wasmtime) has the strongest maintenance story in the sample; the *layer SpecForge actually depends on* (extism) has the weakest — that asymmetry, not wasm-the-model, is the real C5 risk, and it is confirmed live (wasmtime 43→49 gap, wasmtime-48-LTS port still unreleased).
- Every scripting alternative is at minimum "actively maintained": mlua and PyO3 are the most boring-in-a-good-way (stable APIs, clean trackers); rquickjs is rising; deno_core is well-fed but churns its API with V8 in tow; boa is the riskiest engine bet on cadence + conformance grounds.
- Crate health alone does not flip the decision; it removes "maintainer abandonment" as an argument against LUA or TYPESCRIPT, while *strengthening* the case for either dropping the extism layer (call wasmtime directly) or reading extism's thin-maintenance reality as an argument to stay on a substrate this well-resourced.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 3
**One-line rationale:** crate-health data shows wasmtime is the best-maintained engine on the board while the extism layer SpecForge actually pins is the weakest-maintained item on the list — a fix-inside-KEEP_WASM problem (bypass or vendor the thin layer), not a reason to switch runtimes; all scripting alternatives are healthy, so maintenance risk does not block a future LUA/TYPESCRIPT move on these numbers alone.
