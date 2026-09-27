# Post-Migration Hardening Plan — P0 Determinism · P1 MCP Honesty · P2 IDL · P3 Hot Reload

**Status:** approved direction · execution plan
**Created:** 2026-09-27 · tree at `09be2e9` (WASM-only migration complete)
**Goal:** close the runtime-independent determinism bugs, make every MCP tool tell the truth, decide and implement the extension IDL (C7-03), and deliver R-5 hot reload on the single-runtime seam.

Companion to [`migration-plan.md`](migration-plan.md) (executed). Evidence below re-verified against today's tree by three scouting passes.

---

## 0. Scope

In scope, in priority order (user-directed):
- **P0** — determinism findings F1–F4 from `.plugin/research/specforge-determinism-audit.md` (all survive the WASM migration; all host/guest-side, no protocol change)
- **P1** — MCP operation stubs that return fake success (`operations/mod.rs` — 11 ops)
- **P2** — C7-03 extension IDL, decision-gated: Component Model/WIT vs versioned JSON-schema IDL; includes the extism-wasmpin/wasmtime-49 question
- **P3** — R-5 hot reload: config/plugin-artifact change → re-describe → swap → rebuild, no restart

Out of scope: registry protocol redesign (C8 done), cross-process AOT (stays a documented extism-upstream decision point unless W0 changes the runtime), release mechanics (P4, later).

## 1. Success criteria (measurable)

| # | Criterion | Check |
|---|-----------|-------|
| S1 | **Run-to-run determinism**: `specforge check` and `analyze` on cycle fixtures produce byte-identical diagnostics across 20 fresh processes (HashMap seeding differs per process — only subprocess runs prove it) | new test `determinism_cross_process` |
| S2 | Every MCP tool listed in `list_tools` either performs its real function or refuses with an explicit error — **no canned success JSON** | conformance test `no_fake_success_tools` |
| S3 | The extension contract is written down as a versioned IDL with conformance tests | WIT world (GO path) or JSON-schema package (FALLBACK path), referenced by SDK + host |
| S4 | Editing `specforge.json` or a project `.wasm` updates watch/LSP diagnostics **without restart**; a trapping artifact keeps last-good state + surfaces a diagnostic | E2E tests |
| S5 | Full gates green per phase; **no releases** | fmt · clippy `-D warnings` · `cargo test --workspace` · `gh run watch` |

## 2. Current state (verified at `09be2e9`)

### 2.1 P0 findings — exact sites

| ID | Site | Problem |
|----|------|---------|
| F1 | `emitter/compile.rs:723` (`node_ids: HashSet`), `:757-758` (Gray-hit marks `{next,node}`), `:762-763` (ancestor propagation), `:773-777` (`for &id in &node_ids` — randomized seed order) | Cycle detection: seed order varies per process → which back-edge is found first varies → **membership** of `cycle_members` varies. Emission is sorted (`:780-781`) but the set itself isn't stable. Live on E007/E015/E016/W045/W092/W065/W066. Under `--strict`, membership of a W-severity rule changes the exit code |
| F2 | `compile.rs:159-168` (`HashSet` → `.into_iter().collect()`), consumed in order at `specforge-validator/src/file_ref.rs:16-22` | `file_reference_fields` Vec is per-run random; latent until a plugin declares 2+ file-reference fields |
| F3 | `cli/prove.rs:154-159` (`z3_available`), solver block `:406-511`, `run_z3 :164-176` (`.ok()?` swallows spawn failures) | z3 absent → whole solver block skipped **silently**: `claims_proved=0`, no diagnostic; `run_z3` failure degrades individual entailments to no-ops. Machine-dependent analysis output |
| F4 | `extensions/formal/src/lib.rs:198,206` (`refines.keys()` seeds E041 DFS + W031 walk), `:272` (`&produced` HashMap iteration); `condition_check`/`coverage_tracking` already deterministic; host `emitter/analyze.rs:657-668` deserializes guest findings **verbatim** | Guest pass findings are HashMap-ordered → per-run random; rustc ≥1.85 changed HashMap seeding so guest rebuilds re-baseline snapshots |

Also: `.plugin/research/specforge-determinism-audit.md:49` references `probe_detect_cycles.rs`, which was removed during migration cleanup — superseded by S1's permanent cross-process test (noted in D1).

### 2.2 P1 — the 11 stubs and their real backends

All stubs live in `crates/specforge-mcp/src/operations/mod.rs` (378 lines); routing via `tools/mod.rs:105-115`. **Every op has real machinery in the workspace** — the MCP layer just never calls it:

| Op | Stub lines | Canned behavior today | Real backend (exists, tested) |
|----|-----------|----------------------|------------------------------|
| `add_extension` | 144-175 | `installed: true` unconditionally | `cli/add.rs` full flow: `resolve_from_registry` (registry-ops.rs:25), `download_wasm` (http_client.rs:153), integrity, trust flow, `install_extension` (install.rs:19), lock write; local path via `install_from_local` (:125) |
| `remove_extension` | 177-214 | orphan check `.any(\|_\| false)` | `cli/remove.rs` + `check_dependents` (uninstall.rs:16), `uninstall_extension` (:25), lock file rewrite |
| `extensions` | 245-263 | `extensions: []` + graph kinds | `McpState.manifests` (:27) + `extension_info` (:25) + `read_lock_file` (lock_file.rs:62) |
| `providers` | 265-274 | `providers: []` | `cli/providers.rs` reads specforge.json providers |
| `doctor` | 276-293 | hardcoded `extensions_ok: true` | `cli/doctor.rs` + `run_doctor_check` (lock_file.rs:103) + `z3_available()` (prove.rs:154) + compile-cache dir check |
| `format` | 36-53 | `all_clean: true`, `changed_files: []` | `cli/format.rs` → `specforge-formatter` (`discover_targets` + `format_source`) |
| `rename` | 55-110 | `edits: []` (real edge count only) | `specforge-lsp::compute_rename_edits` (rename.rs:21, exported lib.rs:28) |
| `init` | 112-142 | canned project JSON | `cli/init.rs::run` |
| `migrate` | 216-243 | canned | `specforge-migrate` (`detect_format_version` :130, `migrate_project` :616, `run_rollback` :522) |
| `collect` | 295-337 | canned | `cli/collect.rs` (`auto_detect_collector`, contributions.rs) |
| `render` | 339-378 | canned | `cli/export.rs` (`run`, `run_schema` via pipeline compile + `build_schema`) |

### 2.3 P3 — hot-reload seams (all present, none connected)

| Seam | Site | State |
|------|------|-------|
| Watch filter | `specforge-watch/src/watcher.rs:80-88` | post-hoc filter keeps `.spec` only; whole root is watched recursively (notify, 50 ms debounce) — `specforge.json`/`.wasm` events are dropped here |
| CLI freeze | `cli/watch.rs:24` (one cold `compile()`), `:27-66` (GraphConfig derived once), `:100-106` (seeded via `from_cold_build`), loop `:141-147` never refreshes | GraphConfig frozen per run |
| Pipeline config | `specforge-watch/src/pipeline.rs:81` (private `graph_config`, used at `:392`/`:423`), only installed by `from_cold_build` (`:154-195`) | no setter — re-seed via `from_cold_build` is the seam |
| LSP reload | `lsp/backend.rs:161` `load_registries` is re-callable (reads specforge.json `:162-180`, `project_runtime` `:187`, describe `:193-214`, swaps all state `:233-237`); setters exist (`state.rs:149/154/169/173`); `did_change_watched_files` `:984`, `.spec`-only filter `:988-990`; early-return paths `:167-182` keep stale state | everything needed exists; not connected to config/plugin events |
| Runtime invalidation | `extism/runtime.rs` plugins = `Mutex<HashMap>`; `load_module_bytes_with_limits` swaps an entry; **no remove/unload/loaded-names API** | missing reload primitives |
| Re-describe cost | handshake (1 call) + describe per category (host.rs) per extension | cheap; fine to redo per change event |

### 2.4 P2 — IDL/runtime facts (decision inputs)

- extism 1.30 pins **wasmtime 43.0.2** (6 majors behind 49.x); extism has **no Component Model story** (registry-protocol + wasmtime-perf research)
- wasmtime 49: component model production-grade, WASI 0.3 shipped, `wasmtime::component::bindgen!` for hosts, wit-bindgen for guests
- Component sync-call overhead caveat: ~3.5× async-task overhead per call until wasmtime#12311 — mitigated by **batch-shaped** world functions (our passes already take whole-graph snapshots — good shape)
- Blobs: component `.wasm` still `\0asm`-prefixed; registry integrity/sha256/semver gates unchanged
- Our SDK (`specforge-extension-sdk`) owns guest authoring; guests depend on `specforge-protocol-types` (serde-only) — migration surface = bindings layer, not business logic

## 3. Phases

Execution order: **D → M → W0 → (W1–W5 ∥ H)**. Each phase = PR(s) to `main`, gates green, no release.

### Phase D — Determinism (P0)

**D1 — F1 `detect_cycles`: deterministic seeds + true cycle membership.**
1. `node_ids`: `HashSet` → sorted `Vec<&str>` (or `BTreeSet`); seeds iterate in sorted order (line 723, 773).
2. Adjacency: sort each neighbor `Vec` after building (724-734) — defense against upstream edge order.
3. **Exact membership**: replace Gray-hit `{next,node}` + ancestor propagation (`757-763`) with an explicit DFS path stack: on Gray hit of `next`, mark only the current-path segment `next..=node` as cycle members. Feeders *into* a cycle are no longer flagged — they are not in the cycle.
   - This is an intentional semantic fix (today's output over-reports and is unstable precisely because of the propagation). Golden updates for E007/E015/E016/W045/W092/W065/W066 fixtures; changelog-style note in the commit.
4. Tests: golden per rule code incl. a feeder-node case (assert feeder **not** flagged); new `determinism_cross_process` test — run the built binary 20× on a cycle fixture, assert byte-identical stderr+diagnostics (in-process loops cannot prove this; HashMap `RandomState` differs per process).
   - Supersedes the deleted probe file; fix the stale reference in the audit doc (`:49`).

**D2 — F2 `file_reference_fields`:** build as `BTreeSet` → ordered Vec (compile.rs:159-168). Unit test with 2+ file-reference fields asserting diagnostic order.

**D3 — F3 `--prove` without z3:** when `!z3_available()`: emit an explicit diagnostic into the prove report (`code: W098 "prove skipped: SMT solver 'z3' not found on PATH"` — exact code assigned at implementation via `specforge explain` registry), report `solver_available: false`, `proved_claim_ids` stays empty (deterministic). `run_z3` spawn failure (`None`) → per-claim diagnostic, not silent fall-through. Test: run prove with `PATH` stripped, assert the diagnostic exists and output is identical across runs.

**D4 — F4 formal pass findings:** guest-side: sort `findings` by `(entity id, code, message)` before return in `layering_verify` and `event_graph_analyze` (formal lib.rs, before `:250`/`:306`). Host-side (defense in depth, covers ALL third-party passes): canonicalize in `run_extension_passes` after deserialization (`analyze.rs:659-668`) with the same key. Golden test: pass findings order stable across two loads.

**Acceptance:** S1 test green (20×); goldens updated; F2/D3/D4 unit tests green; suite green.

### Phase M — MCP honesty (P1)

Principle: **wire real, or refuse — never fake.** All backends exist (§2.2); the wiring pattern is the CLI command layer, with `state.project_root` supplying the path (missing root → `INVALID_PARAMS` with an honest message, per the extension-tool precedent).

- **M1 read-only ops first** (zero risk): `extensions` (manifests + extension_info + lock-file install state), `providers` (specforge.json), `format` (formatter; `write: false` default → report changes; `write: true` applies), `render` (export schema path), `collect` (auto-detect + report), `init` (real `run`), `migrate` (real `migrate_project` + `run_rollback` on request), `rename` (`compute_rename_edits` returning real edits).
- **M2 `doctor`:** `run_doctor_check` + wasm compile-cache dir check + `z3_available()` + runtime/wasmtime version. Every check reports measured state; `extensions_ok` derived from the lock file, not hardcoded.
- **M3 `remove_extension`:** `read_lock_file` → `check_dependents` (refuse with dependents list unless `force: true`) → `uninstall_extension` → lock rewrite → truthful result incl. what was removed. Routed through the existing mutation-event gating (`tools/mod.rs:70-75`).
- **M4 `add_extension`:** registry specifier → `resolve_from_registry` + `download_wasm` + integrity + trust flow + `install_extension` + lock write (mirroring `cli/add.rs`); local `.wasm` path → `install_from_local`. Network/auth failures returned as real errors. Note in response text when an install requires a session reload to take effect.
- **M5 conformance gate:** test `no_fake_success_tools` — for every tool advertised in `list_tools`: invoke with minimal args against a fixture project; assert the response is either a truthful result (verified against on-disk effect, e.g. lock-file entry exists after `add`) or an explicit typed error. Forbid the canned payloads (exact-shape assertions).

**Acceptance:** S2 conformance test green; manual smoke of add→doctor→remove against the local registry server (already proven in the C8 audit).

### Phase W — IDL / C7-03 (P2), decision-gated

**W0 — Decision spike (time-boxed).** Build on a spike branch:
1. A minimal **componentized** guest (wit-bindgen, one export) + host call through `wasmtime 49::component::bindgen!` alongside the current extism 1.30 path.
2. Microbench: per-call overhead on **real pass payloads** (1.7 k-entity snapshot JSON) — component vs extism, batch-shaped function.
3. Registry compatibility: component blob through publish/download/integrity gates (`\0asm` check, sha256, semver).
4. SDK delta: what `ContributionsBuilder`/macros need for wit-bindgen exports.

**Decision gate:** GO component model iff per-call overhead ≤ 2× on batch-shaped calls, registry gates green, SDK delta confined to the bindings layer, binary-size delta acceptable. Otherwise **FALLBACK**: formalize `specforge-protocol-types` as a versioned JSON-schema IDL package (schema version in handshake, validation against schemas, conformance tests) — C7-03 closes either way; only the mechanism differs.

> **W0 GATE RESULT (2026-09-27): GO.** Spike at `spike/w0-component/` — typed WIT component (wasm32-wasip2, wit-bindgen 0.30) hosted by wasmtime 49 `component::bindgen!`: tiny-call floor **0.31 µs**, 1.5 MB batch call **20.4 µs**, vs **1,056 µs** for the extism byte-array handshake (guest JSON through the PDK dominates). Component blob keeps the `\0asm` magic (layer=1) and sha256 addressing, so registry publish/download/integrity gates are unaffected. SDK delta confined to the bindings layer. All gate criteria met with large margin.

**W1 — WIT world (GO path): ✅ DONE** — `wit/specforge-extension.wit`, package `specforge:extension@0.1.0`: typed `types` interface (diagnostics, spans, validator context/verdict, pass-input, analyzer scan/classify/map), capability worlds (`contributor-world`, `validator-world`, `pass-world`, `analyzer-world`, plus combination worlds for software/formal/rust/typescript). Boundary policy v0.1: per-call surfaces typed; handshake/describe descriptors cross as JSON strings (v0.2 typing follow-up). Validated host-side with wasmtime 49 `component::bindgen!`.

Original scope: `handshake`, `describe(category)`, `pass(snapshot) → list<diagnostic>`, `validate(context) → verdict`, `scan/classify/map`, `mcp-tool`, `mcp-resource`; host imports: `emit-diagnostic`, `read-file` (policy-gated), `query-graph` (scope-gated). Types mapped 1:1 from protocol-types (`ValidatorContext`, `PassInput`, diagnostics). The WIT file becomes the reviewed, versioned contract artifact (checked into `spec/` or `wit/`).

**W2 — Host runtime:** wasmtime 49 direct; component engine + `bindgen!` host side behind the existing `WasmRuntime` trait (ProtocolHost and all machinery keep working); fuel + compile-cache + pooling parity (component `InstancePre`).

**W3 — SDK:** wit-bindgen guest bindings wrapped by the existing `ContributionsBuilder` (guest business logic unchanged); macros regenerate exports; `MockHost` parity.

**W4 — Guest migration:** migrate the 6 builtins; dual-load transition (legacy JSON third-party guests keep working via the extism path for one deprecation window); registry: `manifest_version` bump only if a blob-flavor field is needed (magic bytes unchanged).

**W5 — Flip + close:** component path becomes default for new installs; docs; close C7-3; drop extism dependency after the deprecation window.

### Phase H — Hot reload (P3), parallelizable with W1+

- **H1 — Runtime invalidation API** (`extism/runtime.rs`): `reload_module(name, bytes, fuel) -> Result` (swap under the existing Mutex), `unload(name)`, `loaded_names() -> Vec<String>`. Unit tests: reload swaps behavior; unload + call → `extension_not_found` trap.
- **H2 — Watcher event classes** (`watcher.rs:80-88`): classify events — `SpecChanged(.spec)`, `ConfigChanged(specforge.json)`, `PluginChanged(project *.wasm)`; coalesce bursts in the existing debounce. Embedded builtin blobs change only with binary upgrades — documented, not watched.
- **H3 — Watch CLI loop** (`watch.rs:141-147`): on `ConfigChanged`/`PluginChanged` → rebuild runtime via `project_runtime` → re-describe → rebuild GraphConfig → re-seed pipeline via `from_cold_build` with cached parses → full re-diagnose. Failure policy: a trapping artifact keeps **last-good** runtime/manifests and surfaces an `E04x` diagnostic; never a dead watcher.
- **H4 — LSP reload:** client registration adds `**/specforge.json` + project `.wasm` glob; `did_change_watched_files` (`backend.rs:984-990`) gains branches: `ConfigChanged` → `load_registries()` (re-callable, `:161`) → GraphConfig rebuild → full reindex + republish; `PluginChanged` → same via the H1 API. Fix the stale-state early-return paths (`:167-182`) while here.
- **H5 — E2E:** fixture with a third-party `.wasm`: (a) edit the wasm → new diagnostics appear without restart; (b) ship a trapping wasm → last-good state + E04x; (c) remove `@specforge/software` from specforge.json → its diagnostics disappear. Tests for debounce coalescing.

## 4. Risks & mitigations

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| D1 changes which nodes get flagged (feeders no longer flagged) | certain, intentional | documented semantic fix; goldens updated; alternative "sort-only" variant recorded and rejected (deterministic but wrong) |
| Cross-process determinism test flakes on CI runners | low | fixed iteration count (20), byte-compare of stable serialization; not timing-dependent |
| MCP mutation ops now destructive for real | med | dependents check + `force` flag on remove; mutation-event gating already in place; conformance test asserts on-disk truth |
| W0 spike says "no" to components | possible | FALLBACK path fully specified (versioned JSON-schema IDL) — C7-03 closes either way; no sunk-cost migration |
| Component per-call overhead on tiny calls (validators) | med | batch-shaped world functions; validators already receive full context per call; W0 gate measures before committing |
| Hot reload swaps runtime mid-request | med | plugins under Mutex (per-entry swap atomic); LSP state under existing RwLock; last-good policy on describe failure |
| scope creep in M (11 ops) | med | read-only ops first (M1), then doctor, then mutations (M3/M4); conformance gate can land after M1 to lock in honesty incrementally |

## 5. Definition of done (per phase)

`cargo build --workspace` · `cargo test --workspace` 0 failed · clippy `-D warnings` · fmt clean · phase-specific acceptance green · `gh run watch` · **no version bumps/tags/publishes**.

## 6. Sequencing summary

| Phase | Depends on | Parallelizable with | Size |
|-------|-----------|--------------------|------|
| D (determinism) | — | M | small |
| M (MCP honesty) | — | D | medium (11 ops, gate) |
| W0 spike | D, M | H1-H2 | medium |
| W1–W5 (IDL) | W0 gate | H | large |
| H (hot reload) | — | W | medium |
