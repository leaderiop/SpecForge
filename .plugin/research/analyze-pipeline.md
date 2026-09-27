# The SpecForge Analyze Pipeline — Stages, Extension Hooks, Boundary Data, and What Each Runtime Would Have to Carry

Analyst: `research-specforge-analyze-pipeline` (batch 3 — pipeline anatomy).
Sources: direct code inspection at working tree 2026-09-27; `.plugin/decision-brief.md`; `.plugin/evidence.md`.
Line refs verified by reading the files listed below.

## 1. The pipeline is two halves

`specforge analyze` (`crates/specforge-cli/src/analyze.rs:212-384`) is a thin orchestration over two libraries:

1. **Compilation** — `pipeline::compile(path)` (`crates/specforge-cli/src/pipeline.rs:51-54`) = `build_runtime` + `specforge_emitter::compile::compile_with_runtime` (`crates/specforge-emitter/src/compile.rs:57-296`). This is the "parse → resolve → validate" half. One implementation serves CLI, MCP, LSP ("single source of truth", compile.rs:43-44).
2. **Analysis** — passes over the compiled context, in `specforge_emitter::analyze` (built-ins) plus CLI-only extension-pass dispatch. This is the "coverage / contracts / prove / extension passes" half.

## 2. Stage inventory

### Compilation stages (`compile_with_runtime`, compile.rs:57-296 — numbered 1-15 in source)

| # | Stage | What happens | Extension involvement |
| --- | --- | --- | --- |
| 1 | Load config | `load_project_config` (specforge.json: extension list, spec_root) | — |
| 2 | Load extensions | `load_extensions` → guest `__handshake`/`__describe` via `ProtocolHost` → `Vec<ManifestV2>`; failure = E031 | **Hook 1 (manifest)** |
| 3 | Populate registries | kind/field/edge registries from manifests | declarative only |
| 4 | Parse validation rules | manifest rules → `ValidationRulePattern`s + auto E006 required-field rules | declarative only |
| 5 | Keyword index | keyword→extension map; known dead code — I004 can never fire (compile.rs:87-94) | declarative only |
| 6 | GraphConfig | body-parser E001 suppression, single-reference fields | declarative only |
| 7 | Resolve project | `resolve_project(spec_root)` — file discovery + parse | — (parse stage) |
| 8 | Build graph | `build_graph_with_config(spec_files, graph_config)` | — (parse stage) |
| 9 | Core validation | `validate_with_config` — host-intrinsic rules | — |
| 10 | Strict registry validation | unknown kinds, unknown fields, mistyped refs (E022) vs extension registries | declarative only |
| 11 | Edge label map | manifest edge label → field name | declarative only |
| 12 | Extension validation | `run_extension_validation` (compile.rs:611-647): declarative patterns via `execute_pattern`; cycle detection host-side; `check: "custom"` rules via `WasmValidationRuntime` — **but the analyze path dispatches them to `NativeCustomRules` (compile.rs:486-609), native host Rust, not the guest `validate__*` exports** (the C6-11/C10-10 fix; evidence.md §1.2) | **Hook 2 (rules)** — declarative in wasm; custom rules natively re-implemented |
| 13 | (merged into 12) | conditional-field rules are `ConditionalFieldRequired` patterns | — |
| 14-15 | Extension info + surfaces | `extension_info`, `register_surface_contributions` | declarative only |

Output: `CompilationContext` (compile.rs:24-39) — `graph`, 3 registries, accumulated `diagnostics`, `resolved` (files + parse diags), `validation_patterns`, `surface_entries`, `manifest_surfaces`, `manifests`, `spec_root`.

### Analysis stages (`analyze.rs::run`, 212-384)

| Order | Stage | Where | Notes |
| --- | --- | --- | --- |
| a | compile | `pipeline::compile` | builds ExtismRuntime #1 |
| b | parse `--test-results` | RES-15 `specforge-report.json` → `TestReport` | optional layer-3 input |
| c | **prove (first!)** | `crate::prove::run_prove` (host-native z3 SMT; prove.rs:341-548) | runs *before* built-ins so `proved_claim_ids` thread into coverage's discharge funnel (analyze.rs:247-258) |
| d | built-in passes | `specforge_emitter::analyze::PASS_NAMES = ["coverage", "contracts"]` (analyze.rs emitter:43), dispatched by `run_pass` (462-490) | host-native, shared with MCP |
| e | **extension passes (last)** | `run_extension_passes` (analyze.rs:110-210) | **Hook 3 (compiler passes)** — the only wasm calls in the analyze half |
| f | prove report appended | analyze.rs:308-315 | after extension passes, despite computing first |
| g | strictness + exit | Warning→Error under `--strict`, uniform across all three report sources (317-335); exit 1 on any Error | |

Pass-selection quirk: `--pass prove` computes the prove report then falls into the "unknown analysis pass" error (selected stays empty, analyze.rs:274-280) — prove is reachable only via `--prove` alongside a valid pass.

## 3. Where extension hooks plug in

Exactly three live hook classes touch this pipeline; only two are runtime calls:

1. **Manifest hook (compile step 2)** — every listed extension is loaded through `__handshake`/`__describe` and converted to a `ManifestV2` (`load_extensions`, compile.rs:350-386). Nine describe categories (entities, edges, fields, shared_fields, enhancements, validation_rules, surfaces, passes, feature_flags). Everything downstream of step 2 consumes the manifest declaratively — this is how "first-party" builtins register their kinds/rules with zero privilege.
2. **Validation-rule hook (compile step 12)** — declarative patterns execute in the host engine (`execute_pattern`); custom validators route through the `WasmValidationRuntime` trait, whose analyze-path implementation `NativeCustomRules` is a **native re-implementation** of the four builtin `validate__*` functions (E004/E006/E010/W010, compile.rs:514-606). The guest exports exist in the contract but are never called on this path (evidence.md §1.2). This is R-1's biggest existing violation to converge.
3. **Compiler-pass hook (analyze stage e)** — per manifest: `describe("passes")` → `CompilerPassDescriptor`s → `order_passes` (stable Kahn on `after`/`before`, analyze.rs:35-101) → `runtime.call_export(name, "__pass_<n>", payload)`. Traps and malformed returns degrade to stderr warnings, never run failures (analyze.rs:194-206).

Collector hooks (`collect__*`) are outside the analyze path: test results arrive as a pre-parsed file (`--test-results`), not through guest collection.

**Ordering facts that matter for any runtime:**
- `order_passes` runs **per manifest only**; constraints naming host phases (`after: "resolve"` — used by formal's condition_check, extensions/formal/src/lib.rs:72) or other extensions' passes are silently ignored (analyze.rs:58-60). Cross-extension ordering does not exist; the whole extension block runs after built-ins unconditionally (RES-25, analyze.rs:296-297).
- Constraint cycles fall back to declaration order with a warning (analyze.rs:88-99).

**Hot-reload-relevant fact (R-5):** `run_extension_passes` calls `build_runtime(project_root)` a **second** time per `analyze` invocation (analyze.rs:124) — pipeline::compile already built runtime #1. So one `specforge analyze` run constructs the ExtismRuntime twice and re-loads embedded blobs twice; there is no engine reuse across the two halves (consistent with C7-08: no warm instances). Any runtime comparison must note the current cost model is "fresh engine per half," not "pool."

## 4. What crosses the boundary

### Inbound payload (host → guest), built once, shared by all passes

`run_extension_passes` serializes `{ entities, edges }` **once** (analyze.rs:126-163) and passes the same bytes to every pass of every extension:

- `entities[]` = `ValidationEntity` snapshot (`build_validation_entities`, compile.rs:390-459) plus a host-computed `testable` flag (`kind_registry.supports_verify`, analyze.rs:130-133). Shape (SDK mirror `PassEntity`, specforge-extension-sdk/src/lib.rs:669-684): `{ id, kind, fields: Map<String,String>, incoming_edge_count, outgoing_edge_count, span?, testable }`.
- **The fields map is lossy.** All field values are stringified: StringList/ReferenceList → `", "`-joined; `verify` VerifyList → descriptions `"; "`-joined (kinds extracted to a separate `verify_kinds` vec that does **not** reach the pass payload — the SDK's PassEntity has no verify_kinds field); contract Block → clause keys `", "`-joined. Typed structure degrades to prose at the boundary.
- `edges[]` = `{ source, target, label }` only (analyze.rs:145-155). No edge spans, no edge fields.
- **Not in the payload:** project_root, test_results, proved_claims, diagnostics accumulated so far, registries, source map. Passes are pure functions over the snapshot — which is exactly why R-6 (determinism) and R-2 (sandboxing) are tractable.

### Outbound payload (guest → host)

`Vec<Diagnostic>` JSON — `{ code, severity: "Error"|"Warning"|"Info", message, span?, suggestion? }` (`PassDiagnostic`, SDK lib.rs:727-736). Deserialized with `serde_json::from_slice` on the host (analyze.rs:183). Malformed → warning. Trap → warning. No timeout wraps the call (C7-10: `max_execution_ms` never enforced) — a pass can hang the whole analyze run.

### Transport

`runtime.call_export(module, export_name, payload_bytes) -> WasmCallResult::{Ok(bytes), Trap}` — stringly-typed JSON, no IDL (C7-03). The ABI is documented only in SDK comments ("Wire ABI (v1)", SDK lib.rs:659-703) and the guest's comment (formal lib.rs:66-68).

### Surface asymmetry

The MCP tool `specforge.analyze` runs **built-ins only** (`crates/specforge-mcp/src/tools/analyze.rs:88-108` — `run_pass` over `PASS_NAMES`, `proved_claims` hardcoded empty); extension-pass dispatch exists solely in the CLI. "CLI, MCP, and LSP run identical code" holds for compilation and built-in passes, not for extension passes. LSP likewise reuses `build_validation_entities` for diagnostics, not passes.

## 5. How each candidate runtime would carry this data flow

The flow's actual shape constrains the comparison: **one bulk JSON snapshot per run, N pure-function calls (N=4 today, one extension), tiny diagnostic arrays back, no ambient context, no incremental state, ordering resolved host-side.**

**WASM (Extism/Wasmtime) — incumbent.**
- Transfer: bytes into guest linear memory; guest `serde_json` parse per pass. At evidence-pack scale (~1.7k entities) this is milliseconds. Isolation: linear memory + capability imports is the strongest R-2 story and matches the flow's zero-ambient-context shape perfectly.
- Cost model today is pathological, not intrinsic: two fresh runtime builds per analyze (§3), no module pooling/AOT (C7-02/C7-08), no call timeout (C7-10). All three are fixable *inside* the wasm model — epoch interruption covers the timeout; pooling covers cold cost.
- Determinism: wasm execution is deterministic; R-6 holds today (snapshot-testable suites exist).
- Boundary quality: the missing IDL (C7-03) is the real wasm-side gap — `PassInput`/`PassDiagnostic` types exist in the SDK but the wire is unvalidated JSON both directions. A schema (or wit/schemars generation from the existing types) fixes it without changing runtimes.
- Authoring: Rust + `wasm32-unknown-unknown` + vendored blobs — the heaviest authoring loop of the candidates for AI-agent co-authoring (evidence.md §1.5: guests are ~97% generated manifest, ~3% logic — i.e., the toolchain weight is paid mostly for manifest boilerplate that could be declarative anyway).

**Lua (mlua).**
- Transfer: no cheap native bulk-table bridge; realistic pattern is passing the payload JSON string and parsing guest-side (vendored cjson) — i.e., the same one-parse-per-pass cost as wasm. Near-zero per-call overhead thereafter (function call, no instantiation), and it eliminates the double-runtime-build problem: one interpreter instance serves compile-step-2 and analyze.
- Sandbox: strip `io`/`os`/`package` from the environment → pure-function capability model; mlua memory limits + instruction-count debug hooks can enforce what C7-10 wants (CPU ceiling per pass) more easily than the current unused wasmtime knobs.
- **Determinism tax (R-6):** Lua's `pairs` iteration order is undefined. A pass that builds diagnostics from a hash-table walk (exactly what layering_verify/event_graph_analyze do over HashMaps) returns findings in arbitrary order unless the author sorts or the host sorts. Today's host preserves pass output order verbatim — under Lua it would need a canonicalization step (sort by code+span) to keep snapshots stable. Mitigable, but it is a real, permanent tax the wasm model doesn't pay.
- Authoring/hot-reload: best in class — `.lua` file reload, no toolchain; AI agents write Lua fluently; the 478-line formal guest ports nearly 1:1.

**Python (PyO3).**
- The data flow itself is fine semantically: `json.loads` of the snapshot, pure functions, diagnostic dicts back. GIL is not a blocker (passes already run sequentially by topological order).
- The failures are the hard requirements, not the flow: no capability sandbox survives `import os` (R-2), and single-binary distribution means bundling CPython or requiring system Python (R-3), which also wrecks R-4 reproducibility. Determinism is ambient-ecosystem-dependent.

**TypeScript (deno_core / QuickJS).**
- The boundary was born JSON; JS consumes it natively. `JSON.parse` per pass, plain-object diagnostics back — zero impedance mismatch, and the ABI becomes a `.d.ts` typed contract, which is the cleanest available fix for C7-03. Key iteration is insertion-ordered and `Array.sort` is stable (ES2019), so R-6 holds without Lua's pairs tax; strip `Date`/`Math.random` and determinism is complete.
- deno_core: permissions model (allow-none default) gives R-2; hot reload = ES module re-import (excellent); cost = V8 in the binary — the evidence pack already flags it as heavy-static, comparable in weight order to wasmtime. QuickJS: ~1 MB and deterministic, but permissions must be hand-rolled (frozen realm, no import machinery).
- Per-isolate memory (V8, several MB × extensions) exceeds wasm instances for large plugin counts.

**MULTI.**
- The pipeline is already runtime-agnostic at the boundary (`PassInput` JSON in, diagnostics out) — a `PassEngine` trait could dispatch to several engines mechanically. But R-1 demands one mechanism, and the flow gains nothing from two: passes need no ecosystem (all four builtin pass bodies are HashMap/Vec bookkeeping), so the "platform pull" argument for a scripting tier is empirically absent. MULTI doubles the sandbox, determinism, and distribution audit surface for zero pipeline benefit.

## 6. Data-flow-derived weights for the decision

1. The flow is a **pure bulk-snapshot function** — every candidate can carry it; the pipeline does not inherently discriminate. Discriminators are per-call overhead as N grows, determinism mechanics, sandbox defaults, and authoring loop.
2. Wasm's pipeline problems are all **config/mechanism gaps** (double build, no pool, no timeout, no IDL), not model gaps — fixable without changing runtimes.
3. Lua's one structural liability is **iteration-order determinism**; TypeScript's is **binary weight**; Python's are **R-2/R-3/R-4 outright**.
4. The lossy fields flattening and the absent `verify_kinds`/test_results/proved_claims in `PassInput` are ABI design decisions independent of runtime — whichever engine wins, the IDL work (C7-03) is mandatory, and adding `proved_claims` to the snapshot is the natural next ABI v2 step so extension passes can join the discharge funnel.
5. Surface parity (MCP/LSP never run extension passes) must be fixed regardless of runtime by moving `run_extension_passes` into `specforge_emitter::analyze` next to `run_pass` — the same "one implementation serves every surface" rule the built-ins already follow.

## Bottom line

The analyze pipeline's extension boundary is a clean pure-function contract — one serialized `{entities, edges}` snapshot in, `Vec<Diagnostic>` out, host-side ordering, no ambient context — and that shape is the easiest possible workload for *any* embedded runtime. The incumbent wasm path's defects on this pipeline (double runtime construction per run, no warm engines, unenforced timeouts, no IDL) are implementation gaps fixable inside the KEEP_WASM model, while each alternative carries a pipeline-visible liability: Lua's undefined `pairs` order (R-6), TypeScript's V8 binary weight (R-3/criterion 10), Python's unsandboxability (R-2), MULTI's duplicated audit surface (R-1).

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 3
**One-line rationale:** The pipeline's pure snapshot-in/diagnostics-out flow is carried well by any runtime; wasm's pipeline costs are fixable mechanism gaps (pooling, timeout, IDL, single runtime build), while every alternative introduces a pipeline-level liability (Lua determinism, TS binary weight, Python R-2/R-3 failure) that the flow itself never pays for.
