# WASM-Only Migration Plan — C7-11 / R-1 Consolidation

**Status: EXECUTED — complete.** Phases 0–8 implemented on `main` between `4616c4d` and the final verification commit. Verdict in `decision.md` carries the completion note.
**Created:** 2026-09-27 · tree at `0557cbb`
**Goal:** every extension executes exclusively through the Wasm runtime (Extism/Wasmtime); the native execution tier is deleted.

---

## 0. Scope interpretation

"Keep only wasm and remove the others" means: **delete the native execution tier** — the six native `BuiltinExtension` implementations, `BuiltinRuntime`, `NativeCustomRules`, `CompositeRuntime`, and `runtime_for_extensions` — so that CLI, LSP, and MCP all run extensions through `ExtismRuntime` (real wasmtime, vendored blobs). This closes audit item **C7-11** (mirror consolidation) and makes **R-1** (no first-party tier) literally true: the builtins ship as wasm blobs exactly like a third-party plugin would.

**Not in scope** (separate plans):
- MCP management-op stubs (`add`/`remove`/`extensions`/`doctor` — registry features, `operations/mod.rs:144-291`)
- Watch incremental reload (R-5) — benefits from this plan but is its own arc
- WIT / Component Model IDL (C7-03) — follow-on, per decision.md revisit conditions
- Cross-process AOT cache — blocked on extism exposing module serialization (see §6.3)
- Registry protocol work (C8, complete)

## 1. Success criteria (measurable)

| # | Criterion | Check |
|---|-----------|-------|
| S1 | Zero native-tier symbols | `grep -rn "BuiltinRuntime\|BuiltinExtension\|NativeCustomRules\|runtime_for_extensions\|CompositeRuntime" crates/ extensions/ xtask/` → 0 hits |
| S2 | All 6 builtins execute via wasm in all 3 surfaces | E2E matrix (§8) green |
| S3 | Custom rules fire through `validate__*` wasm exports | E004/E006/E010/W010 goldens byte-identical to pre-migration; debug trace shows wasm dispatch |
| S4 | Parity contract holds | Phase 0 harness byte-compares protocol outputs; green at every phase boundary |
| S5 | Suite + CI green | `cargo test --workspace`, clippy `-D warnings`, fmt, `gh run watch` |
| S6 | No release | standing directive — no tags/bumps/publishes |

## 2. Current state (verified at `0557cbb`)

### 2.1 Three `WasmRuntime` impls

| Impl | Crate | Backing | Used by |
|------|-------|---------|---------|
| `BuiltinRuntime` | `specforge-wasm/src/builtin.rs:25` | native Rust `BuiltinExtension` trait objects, serialized to the same JSON wire format | emitter default, LSP, MCP, infer_status, most emitter tests |
| `ExtismRuntime` | `specforge-extism/src/runtime.rs:18` | extism 1.30 `PluginBuilder` → wasmtime; `Mutex<HashMap<name, Plugin>>` | CLI (`pipeline.rs:16-49`), `analyze.rs` pass dispatch |
| `CompositeRuntime` | `specforge-extism/src/composite.rs:13` | builtin-name → native, else → wasm | tests only |

The wire protocol (`__handshake` / `__describe` / analyzer exports via `ProtocolHost`, `specforge-wasm/src/protocol/host.rs:12`) is runtime-agnostic and **survives unchanged**.

### 2.2 Native execution paths to migrate (call sites)

| # | Site | What runs natively today |
|---|------|--------------------------|
| N1 | `emitter/compile.rs:46-51` `compile()` | builds `BuiltinRuntime` via `runtime_for_extensions` for every caller that doesn't inject one |
| N2 | `lsp/backend.rs:185` (+ `state.rs:76`, tests `completion.rs:17`, `contracts.rs:168`) | initialize-time manifest/pattern loading |
| N3 | `lsp/backend.rs:445-447` | per-change declarative rules via `execute_pattern(pattern, &entities, None)` — `None` = custom rules never fire in LSP |
| N4 | `mcp/compile.rs:27` | `compile_project` → native pipeline |
| N5 | `mcp/tools/infer_gaps.rs:58-61` | native scanners (`scan_source_files`) |
| N6 | `mcp/tools/mod.rs:108-127` | extension tools answer "requires a Wasm runtime for execution" |
| N7 | `cli/infer_status.rs:128-132` | native scanners (builds a **second, native** runtime after `pipeline::compile` already built a wasm one) |
| N8 | `emitter/compile.rs:464-609` `NativeCustomRules` | E006/E010/W010/E004 validators executed as host-side graph walks |
| N9 | `xtask/src/extract_extension_json.rs:4-38` | extraction of describe JSON from native impls into guest crates |

### 2.3 Missing wasm guests

`extensions/{product,software,governance,formal}` exist (SDK-authored, vendored blobs, embedded at `extism/src/builtins.rs:3-17`).
**`@specforge/rust` and `@specforge/typescript` have no guest crate** — their scan/classify/map logic lives only in `emitter/src/builtins/{rust,typescript}.rs` (pure line-parsers, no tree-sitter — mechanically portable).

### 2.4 Known wasm-side gaps (audit cross-refs)

- **C7-02**: `cache.rs` "AOT" is a byte-copy (`cache.rs:14-18` doc admits it); `instantiate()` ignores `_aot_cache_path` (`extism/runtime.rs:84`)
- **C7-08**: `EnginePool` is a metadata LRU ledger (`wasm/src/engine_pool.rs:24-26`) — names+MB, no engine handles. The *real* warm pool is `ExtismRuntime.plugins` (persistent per process). `analyze.rs` also builds a second runtime per run (`analyze.rs:124`)
- **C7-10**: `max_execution_ms` never enforced; extism 1.30 exposes `PluginBuilder::with_fuel_limit(u64)` + `Plugin::fuel_consumed()` (verified in extism 1.30.0 source)
- **C7-04**: `sandbox.rs` policy is advisory metadata; host functions in `HostContext` are the actual capability surface
- **C10**: custom rules "not yet wired in production" (`extension-sdk/src/lib.rs:62-63`)

### 2.5 extism 1.30.0 API facts (verified from vendored source)

- `PluginBuilder::compile() -> CompiledPlugin` (plugin_builder.rs:203); `Plugin::new_from_compiled(&CompiledPlugin)` (plugin.rs:448) — in-process compile-once/instantiate-many
- `PluginBuilder::with_fuel_limit(u64)` (plugin_builder.rs:168); `Plugin::fuel_consumed()` (plugin.rs:1233); `Plugin::reset()` (plugin.rs:675)
- **No cross-process module serialization** — `CompiledPlugin` does not expose wasmtime `Module::serialize`. Per-invocation compile cost is therefore irreducible without upstream changes: measured 77–110 ms per 415 KB blob (startup-latency research, M3 Pro), proportional to installed extensions.

## 3. Target architecture

```
                    ┌──────────────────────────────────────────┐
                    │  specforge_extism::project_runtime(path) │  ← ONE constructor
                    │  (hoisted from cli/pipeline.rs:16-49)    │
                    └───────┬──────────────────────────────────┘
                            │  Arc<ExtismRuntime>
        ┌───────────────────┼───────────────────────────┐
        ▼                   ▼                           ▼
   CLI pipeline         LSP backend                 MCP server
   (already wasm)   (initialize + per-change)   (compile/analyze/infer/tools)
        │                   │                           │
        └───────────────────┴───────────┬───────────────┘
                                        ▼
                     ProtocolHost over &dyn WasmRuntime   (unchanged)
                                        │
              6 wasm blobs (4 existing + rust + typescript) embedded in extism
              + third-party .wasm paths from specforge.json
```

- **One runtime per process/session**, constructed once, shared (`Arc`).
- **emitter keeps no runtime construction**: `compile_with_runtime(path, Option<&dyn WasmRuntime>)` remains the single compilation entry; the self-constructing `compile()` convenience is deleted (dependency direction preserved: emitter depends on `specforge-wasm` trait only).
- **Custom rules**: `WasmValidationRuntime` implemented as `WasmCustomRules { runtime, graph }` in the emitter — same trait, same `execute_pattern` loop, wasm dispatch instead of graph walks.
- **Fuel**: derived deterministically from manifest limits (§6.1).
- **Deleted**: everything in §2.2's native tier + `EnginePool` ledger + fake-AOT cache plumbing (§6.2).

## 4. Phases

Execution order: **0 → (1 ∥ 2 ∥ 6) → 3 → (4 ∥ 5) → 7 → 8**. Each phase is one PR to `main`, CI green, no release. Parity harness (Phase 0) re-runs at every boundary.

### Phase 0 — Parity harness (safety net, no behavior change)

**Deliverable:** `crates/specforge-extism/tests/parity.rs`

For each of the 4 existing builtin guests: run `__handshake` + all 9 `__describe` categories through `ProtocolHost` twice — once over the native `BuiltinRuntime` (via `runtime_for_extensions`, the oracle), once over `ExtismRuntime` + vendored blob — and assert **byte-equal** JSON.

- Fixtures: reuse the corpus from `extension_json_sync` (in-memory regeneration) + 2–3 real spec fixtures.
- Analyzer parity (scan/classify/map) lands in Phase 1 when the scanner guests exist.
- This test is the migration contract: it *proves* "wasm output == native output" before anything is deleted, and fails loudly if a blob drifts mid-migration.

**Acceptance:** green on current tree; runs in CI; documents the oracle relationship (deleted together with the native tier in Phase 7, its job done).

### Phase 1 — Scanner guests: `@specforge/rust`, `@specforge/typescript`

**Deliverable:** `extensions/rust/`, `extensions/typescript/` (SDK-authored, same shape as the existing four), vendored blobs, embedded + guarded.

1. Port `emitter/src/builtins/rust.rs` (348 lines: `scan_rust`, `parse_pub_item`, `classify_rust`, `map_rust`, `to_snake_case`, `is_test_or_build_file` + `PUB_PATTERNS`) and `typescript.rs` (same shape) into guest `lib.rs` exports `scan__rust`/`classify__rust`/`map__rust` (and `__typescript` twins). Pure line-parsing — no new deps, `wasm32-unknown-unknown` clean.
2. Author `handshake.json` (AnalyzerDescriptor must byte-match native — Phase 0 harness asserts) + describe JSONs.
3. `xtask build_builtins` → 6 blobs; `extism/builtins.rs` `BUILTIN_EXTENSIONS` +2 entries; `builtin_blob_sync` test extends (35 → ~53 payload checks); `extension_json_sync` covers the 2 new twins.
4. Parity harness extended: scan/classify/map over a fixture corpus (real `.rs`/`.ts` files covering pub fn/struct/enum/trait/impl/export forms, test/build file exclusions) — native vs wasm, byte-equal responses.

**Acceptance:** 6 blobs built and embedded; parity green including analyzers; suite green.

### Phase 2 — Guest-side custom-rule exports (software guest)

**Deliverable:** 4 `validate__*` exports in `extensions/software` + `ValidatorContext` wire format + native↔wasm parity.

1. **Wire format** (`specforge-protocol-types`, versioned `ValidatorContext v1`):
   ```json
   {
     "entity": { "id", "kind", "fields": [{ "key", "value", "annotations": [..] }],
                 "methods": [{ "name", "params": [{"name","type"}], "returns": "..." }] },
     "referenced": [{ "id", "kind" }],        // resolution of every ReferenceList target
     "declared_types": ["..."],                // ids of kind=="type" entities
     "primitives": ["string", "void", ...]     // host PRIMITIVE_TYPES list (compile.rs:469-475)
   }
   ```
   Response: `{"verdict":"pass"}` | `{"verdict":"fail","field":...,"value":...}` (mirrors `CustomVerdict`).
   Rationale per validator: E006/E010 need referenced-target kinds; W010 needs field annotations; E004 needs methods + declared types + primitives. Everything `NativeCustomRules` touches (`compile.rs:494-601`) is representable.
2. Port the 4 validators into the software guest as plain exports (`validate__event_triggers`, `validate__milestone_behavior_ranges`, `validate__type_field_annotations`, `validate__port_methods`) — naming-convention ABI, matching what manifests already declare.
3. **Parity as oracle**: while `NativeCustomRules` still exists, run both over fixture graphs (covering: valid triggers, dangling refs, non-behavior targets, unknown annotation, port method with undeclared type, generic types `Result<A,B>`, array types) — assert identical verdicts.

**Acceptance:** parity green; the SDK gains a typed `ValidatorContext` (guest-side struct) so third parties can implement custom rules without hand-rolling JSON.

### Phase 3 — Shared runtime constructor + emitter decoupling

**Deliverable:** one constructor, no native construction outside tests.

1. Hoist `cli/pipeline.rs:16-49` → `pub fn project_runtime(path: &Path) -> Arc<ExtismRuntime>` in `specforge-extism` (keeps: HostContext + `load_builtins_for` + `.wasm`-path loading + name normalization via `emitter::compile::normalize_extension_name` logic moved/duplicated behind extism? — no: move `normalize_extension_name` into `specforge-wasm` (shared, no deps) and have both use it).
2. `cli/pipeline.rs` delegates to it. `infer_status.rs` refactored: ONE `project_runtime` reused for `compile_with_runtime` + `scan_source_files` (kills the double runtime at `infer_status.rs:23+129`).
3. `scanner_dispatch::scan_source_files` signature: `&BuiltinRuntime` → `&dyn WasmRuntime` (`scanner_dispatch.rs:16`; internals already use `&dyn WasmRuntime` via `contributions::invoke`).
4. Delete `emitter::compile()` self-constructing variant; **all remaining callers pass a runtime**:
   - MCP `compile.rs:27` → `project_runtime` + `compile_with_runtime`
   - emitter tests (`builtins.rs`, `outline.rs`, `registry_fields.rs`, `e2e_pipeline.rs:47`, `extension_json_sync.rs:62`) → `project_runtime` (these become true wasm tests — the parity oracle keeps using `runtime_for_extensions`, which stays `#[doc(hidden)] pub` until Phase 7)
5. `mcp/Cargo.toml` + `specforge-lsp/Cargo.toml` gain `specforge-extism` dep.

**Acceptance:** `grep -rn "runtime_for_extensions" crates/ --include="*.rs" | grep -v tests | grep -v parity` → only the definition + oracle test remain; suite green.

### Phase 4 — LSP + MCP on wasm

**Deliverable:** both surfaces fully wasm; extension surfaces execute.

**LSP** (`backend.rs`):
1. `load_registries` (:162-203): `project_runtime` → `ProtocolHost` → manifests/patterns; delete the inline name-normalization dup (:191-197) in favor of the shared helper.
2. `index_workspace` (:~142): `known_extension_keywords` derived from the same manifests (`compile.rs:117-125` behavior) — fixes the dead I004 map (drift also flagged by hot-reload research: LSP vs CLI disagree).
3. `LspState` holds `Arc<ExtismRuntime>` (for Phase 5 custom rules).
4. Tests `state.rs:76`, `completion.rs:17`, `contracts.rs:168` → `project_runtime`.
5. Golden diagnostic files: capture pre-migration LSP output on a fixture workspace; assert identical post-migration.

**MCP** (`specforge-mcp`):
1. `compile_project` (:27) → `project_runtime`; `McpState` holds `Arc<ExtismRuntime>`.
2. `analyze` tool: hoist `run_extension_passes` + `order_passes` from `cli/analyze.rs:35-210` into `specforge_emitter::analyze` (takes `&dyn WasmRuntime` — no dependency issue); MCP calls it with the state runtime; extension passes actually run (doc comment admitting they don't, deleted). `proved_claims` plumbed from prove results instead of hardcoded empty (`tools/analyze.rs:81`).
3. `infer_gaps.rs:58-66` → state runtime + wasm scanners (parity guaranteed by Phase 1).
4. **Extension tool execution** (new protocol surface, kept minimal): surface tool descriptors gain an `export` field (convention default: `__mcp_tool_<name>`); dispatch = `runtime.call_export(name, export, args_json)` expecting `{ "content": ... }`. Undeclared/no-export → clear MCP error (not silent). Resource reads: wire the already-existing `dispatch_surface_mcp_resource` (`wasm/src/surface.rs:210+`) into `resources/mod.rs:17-59` instead of falling through to "Unknown resource URI".
5. Parity: CLI `specforge analyze` vs MCP `analyze` on the same fixture → identical pass reports.

**Acceptance:** LSP goldens byte-identical; MCP analyze runs extension passes; infer_gaps uses wasm; extension tool + resource round-trips work (fixture extension with one tool + one resource); session-init latency measured and documented (expected one-time ~200–400 ms for 6 blobs, amortized per session).

### Phase 5 — Custom rules through wasm (R-1 cutover for validation)

**Deliverable:** `NativeCustomRules` deleted; wasm dispatch everywhere.

1. `emitter/compile.rs`: `WasmCustomRules<'a> { runtime: &'a dyn WasmRuntime, graph: &'a Graph }` implementing `WasmValidationRuntime`:
   - builds `ValidatorContext v1` per call from the graph (entity + `ReferenceList` target resolution + declared-type ids + primitives),
   - `runtime.call_export(ext_name, wasm_function, &context)` — note: needs the *extension name*, which the trait doesn't pass. Resolve at construction: `run_extension_validation` iterates rules per-manifest already (`rule_inputs` carry names) → construct one `WasmCustomRules` per manifest (name captured), thread the runtime from `compile_with_runtime` into step 12. No trait change, no new dependency.
   - trap/unknown-export → `Err` → existing warning path (same policy as today's unknown-validator `Err`, `compile.rs:606`).
2. LSP per-change (`backend.rs:439-447`): `execute_pattern(pattern, &entities, Some(&wasm_rules))` for `Custom`-kind patterns, runtime from state. Cost control: only Custom patterns trigger calls; context payload is per-entity (~KB).
3. Golden diagnostics for E004/E006/E010/W010 across the fixture corpus — byte-identical pre/post.
4. `SPECFORGE_DEBUG_RULES` trace gains `wasm` marker (verifiable dispatch).

**Acceptance:** `grep NativeCustomRules` → 0; goldens green; new LSP test fires a custom rule on edit; suite green.

### Phase 6 — Performance + limits (parallelizable with 3–5)

**6.1 Fuel (closes C7-10).** `project_runtime` reads each manifest's limits (`SandboxPolicy.total_memory_mb`, `max_execution_ms`) → `PluginBuilder::with_fuel_limit(max_execution_ms * FUEL_PER_MS)` with `const FUEL_PER_MS: u64 = 20_000_000` (documented heuristic: ~20M wasmtime instructions/ms on Apple Silicon; deterministic constant, not wall-clock — R-6 safe). Fuel exhaustion → trap → existing trap-as-warning paths (analyze) / Err-as-warning (validation). Test: fixture guest with `while(true)` export → returns warning, process terminates.

**6.2 Honest cache + pool (closes C7-02/C7-08).**
- Delete `cache.rs` wasm byte-copy machinery (`cache_wasm_binary`, `has_cached_artifact`, `invalidate_*` wasm paths, `cache_path_for_hash`) — it never fed compilation. **Keep** grammar cache (`grammar_cache_key` + friends — real consumers).
- Delete `ExtismRuntime::with_aot_cache_dir` + `has_cached_module` (callers: tests only) and `EnginePool` (`engine_pool.rs` + tests) — the persistent `plugins` map *is* the warm pool; the ledger misleads.
- Document in decision.md: C7-02 resolved as "fake cache removed; in-process compile-once via extism `PluginBuilder`; cross-process AOT deferred (§6.3)".
- Double-build elimination: `analyze.rs:124` reuses the compile runtime (thread `Arc<ExtismRuntime>` from `pipeline::compile` — refactor `pipeline::compile` into `compile_with(project_root) -> (CompilationContext, Arc<ExtismRuntime>)` or a callback form).

**6.3 Cross-process AOT — decision point (optional, deferred).** True per-invocation savings (~0.3–0.6 s CLI) require wasmtime module serialization, which extism 1.30 does not expose. Options: (a) accept status quo (CLI invocations compile installed blobs once), (b) upstream extism PR exposing `CompiledPlugin::serialize`, (c) bypass extism with direct wasmtime (large — re-implements host-function glue). **Decision: (a) now, revisit (b) if CLI latency complains.** Documented, not blocking.

**6.4 Sandbox enforcement at the real boundary (C7-04/C7-09).** Guests are pure-compute (no WASI imports); the capability surface is `HostContext` host functions. Wire per-plugin `SandboxPolicy` checks inside `build_host_functions` (`host_context.rs`): `emit_diagnostic` always allowed; `read_file`/`query_graph` gated on policy + `query_scope` (C7-09: honor the scope in `make_query_graph_fn`). Memory: verify extism manifest memory limits; if unsupported upstream, enforce via policy check + document gap (fuel already bounds CPU).

**Acceptance:** fuel test; scope test (out-of-scope query denied with diagnostic); single-build assertion (debug counter) for analyze; suite green.

### Phase 7 — Deletion (clean cutover)

**Delete:**
- `specforge-wasm/src/builtin.rs` (trait + runtime, whole file)
- `specforge-emitter/src/builtins/` — all 6 impls + `mod.rs` (`KNOWN_BUILTINS` moves to `extism::builtins`)
- `emitter/compile.rs`: `NativeCustomRules`, `PRIMITIVE_TYPES`, `base_type_names` (moved into guests in Phase 2)
- `specforge-extism/src/composite.rs` + tests
- `specforge-wasm/src/engine_pool.rs` + tests
- `cache.rs` wasm byte-copy remnants
- `StubWasmRuntime` (`validation_engine.rs:94-108`) if now unused
- `xtask/src/extract_extension_json.rs` + its invocation — guests become the **source of truth** for describe JSON (authoring moves fully guest-side; the mirror it fed is gone)
- `emitter/tests/extension_json_sync.rs` — with one copy of the JSON, the drift guard's rationale dies
- `runtime_for_extensions` + the parity harness itself (job complete) — replaced by a **CI grep gate** (workspace test that fails if any deleted symbol reappears, S1)

**Rewrite tests on the wasm path:** `emitter/tests/{builtins,dual_mode,outline,registry_fields,scanner_dispatch}.rs` → blob-backed (shared `OnceLock<ExtismRuntime>` per test binary to keep suite time sane; blobs are small, instantiation is µs — compile dominates and happens once).

**Keep:** `builtin_blob_sync` (blobs must track guest JSON — now the only guard needed), `WasmRuntime` trait, `MockRuntime`, `ProtocolHost` stack, all `&dyn WasmRuntime` machinery (lifecycle/contributions/surface/host_functions).

**Acceptance:** S1 grep gate green; full suite green; clippy/fmt clean; CI green.

### Phase 8 — Verification, docs, closure

1. **E2E matrix** (scripted, fixture project with all 6 builtins + 1 third-party `.wasm`):
   CLI (`check`/`analyze`/`outline`/`export`/`query`/`infer`/`watch`/`migrate`) × LSP (diags incl. custom rules, completion, contracts) × MCP (compile/validate/analyze passes/infer_gaps/tool/resource) — all outputs match pre-migration goldens where applicable.
2. **Perf table:** session-init (LSP), per-invocation (CLI), per-request (MCP) before/after — documented in decision.md.
3. **Docs:** decision.md execution-plan checkboxes → done; audit C7-02/04/08/09/10/11 + C10 closed with the honest resolution notes (§6.2, §6.4); architecture docs updated (one runtime); `.reports` data.js note if needed.
4. **No release** (standing directive). Commit series to `main`, CI watched per phase.

## 5. Risks & mitigations

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Behavioral drift native↔wasm (describe JSON, scanner edge cases, validator verdicts) | med | Phase 0/1/2 parity harnesses with byte-equality; fixtures before any cutover |
| `ValidatorContext` misses info a validator needs (E004 generics) | low | context carries declared types + primitives + per-reference kinds (superset of what native walks touch); parity on generic-heavy fixtures |
| LSP init latency regresses (~100 ms/blob compile) | certain, bounded | lazy load of configured extensions only (existing behavior); amortized per session; measured + documented; revisit via §6.3 if painful |
| New MCP tool-execution protocol surface grows scope | med | minimal: declared-export convention only; undeclared → clear error; SDK helper, no new manifest schema required |
| extism memory limits unsupported | low | verify in Phase 6; fallback = policy check at host-fn boundary + documented gap (fuel bounds CPU regardless) |
| Test suite slows (native tests → wasm) | certain, small | shared runtime per test binary (`OnceLock`); measure in Phase 7 |
| Fuel heuristic (instr/ms) miscalibrated → truncation of legit runs | low | generous default (20M/ms), per-manifest override via limits, trap surfaces as warning not error; deterministic |
| xtask extraction retirement breaks contributor flow | low | describe JSONs hand-authored in guest crates from Phase 1 onward; README note in extensions/ |

## 6. Definition of done (per phase)

Every phase PR: [ ] `cargo build --workspace` · [ ] `cargo test --workspace` 0 failed · [ ] clippy `-D warnings` · [ ] fmt clean · [ ] parity/golden gates for that phase green · [ ] CI watched via `gh run watch` · [ ] no version bumps/tags/publishes.

## 7. Sequencing summary

| Phase | Depends on | Parallelizable with | PR-sized |
|-------|-----------|--------------------|----------|
| 0 parity harness | — | — | small |
| 1 scanner guests | 0 | 2, 6 | medium |
| 2 validator exports | 0 | 1, 6 | medium |
| 3 shared constructor | 0 | 1, 2, 6 | medium |
| 4 LSP+MCP wasm | 1, 3 | 5 | large (split LSP/MCP if needed) |
| 5 custom rules wasm | 2, 3 | 4 | medium |
| 6 perf+limits | 0 | 1, 2 | medium |
| 7 deletion | 1-6 all | — | medium |
| 8 verification/docs | 7 | — | small |
