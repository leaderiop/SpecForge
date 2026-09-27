# Determinism Audit — `compile.rs` + `builtins/software.rs`

Analyst: `research-specforge-determinism-audit` · Rev audited: working tree @ 4c9e9f2 (matches evidence pack). All line references verified in this session.

**Question:** Are the validation rules deterministic? Are the compiler passes deterministic? What breaks if two runs produce different diagnostics? How would each candidate runtime affect this?

**Headline:** One genuine run-to-run nondeterminism bug lives in the host pipeline (`detect_cycles` iterates a per-process-randomized `HashSet`), proven empirically. Everything else in the validation/pass stack is deterministic by construction — including, surprisingly, the wasm guests. The nondeterminism is runtime-independent: swapping Wasm for Lua/Python/TS does not touch it.

---

## 1. Shape of the pipeline (what determinism has to hold across)

`compile_with_runtime` (`crates/specforge-emitter/src/compile.rs:57-296`) is the declared single source of truth for CLI, MCP, and LSP (doc comment at :41-46). `Vec<Diagnostic>` is built as a plain concatenation of step outputs in fixed pipeline order:

config → extension manifests (config order) → registry population → rule parsing → graph build → core validation → strict field validation → declarative rule execution → surface registration. **No downstream consumer re-sorts**: `specforge check` serializes/renders `ctx.diagnostics` as-is (`crates/specforge-cli/src/check.rs:12,33-48`); the LSP publishes subsets of it (`crates/specforge-lsp/src/backend.rs:850+`).

Two runtimes feed this same pipeline: the production **Extism/wasm path** (`specforge-cli/src/pipeline.rs:16-53`, embedded vendored blobs) and the **native `BuiltinRuntime`** (`crates/specforge-emitter/src/builtins/mod.rs:31`, `crates/specforge-wasm/src/builtin.rs:48`). Both hit identical host-side code, so every finding below applies to both — which is exactly the R-1 lens: there is no "trusted native" determinism escape hatch.

## 2. What is deterministic — verified, mechanism by mechanism

| Mechanism | Evidence | Why deterministic |
| --- | --- | --- |
| Spec file discovery order | `specforge-resolver/src/resolve.rs:220-231` | `files.sort()` after walkdir |
| Import resolution order | `resolve.rs:433-436` | topological queue explicitly sorted ("Sort queue for deterministic output") |
| Graph node iteration | `specforge-graph/src/graph.rs:140-144` | `nodes()` sorts by `id.raw`; edges are insertion-ordered `Vec` |
| Rule pattern execution order | `validation_engine.rs:218` | `patterns.sort_by(code)` — stable sort, ties fall back to manifest order (deterministic: manifests follow `specforge.json` array order) |
| Auto-generated E006 rules | `detection.rs:317` | sorted by `(target_kind, field)` |
| Per-rule diagnostics order | `validation_engine.rs:270-276,276-457` | entities iterated as a pre-sorted `Vec`; per-entity checks are pure lookups (`fields` HashMap used only via `get`/`contains_key`; `verify_kinds` is an ordered `Vec`) |
| Custom native rules (E004/E006/E010/W010) | `compile.rs:486-609` | pure functions of the graph; `PRIMITIVE_TYPES` is a `const`; first-fail short-circuit. `SPECFORGE_DEBUG_RULES` (`:506,623`) only gates `eprintln!` to stderr — never the diagnostic stream |
| Cycle diagnostics *order* | `compile.rs:733-736` | `sorted_members.sort()` before emit (but see F1: the *set* varies) |
| Manifest schema/consistency validation | `validate.rs:48,359,411` | diagnostics sorted by message; rules sorted by code; peer-dep cycles sorted (`:313`) |
| Registry contributions | `contributions.rs:125,196` | HashMap values collected then sorted by unique `entity_kind` key → total order |
| `populate_registries` diagnostics | `populate.rs:18,52,109,153,212` | pushed inside `for manifest in manifests` loops — manifest order |
| Unknown-kind/field/ref detection | `detection.rs:33-115,241+` | iterate caller-supplied `Vec`s (graph order); HashMaps are lookup-only |
| Extension pass dispatch order | `specforge-cli/src/analyze.rs:35-101,166-208` | stable Kahn (`order_passes`), declaration-order tie-break; manifests in config order; cycle in constraints → explicit fallback to declaration order |
| Pass wire payload bytes | `analyze.rs:126-157`; root `Cargo.toml:51` | entities pre-sorted; `fields` HashMap serialized through `serde_json::Map` = **BTreeMap** (no `preserve_order` anywhere in the workspace) → byte-identical payload per run |
| Guest pass execution | `extensions/formal/src/lib.rs:73-307` | pure functions of `PassInput`; no ambient sources; `extism` `Plugin::call` per export resets instance state (`specforge-extism/src/runtime.rs:140`) |
| Guest HashMap iteration order | rust std: `library/std/src/sys/unsupported/common.rs` @ 1.70 and `sys/pal/unsupported/common.rs` @ 1.82 — `hashmap_random_keys() -> (1, 2)` | wasm32-unknown-unknown has **no random seed**: guest map order is fixed per compiled blob (see F4 for the 1.85 caveat) |
| Timeout enforcement | audit C7-10: `max_execution_ms` never enforced | *inertia is determinism-preserving*: no wall-clock cutoff means no run-dependent truncation (see §6.5) |

`builtins/software.rs` specifically: 668 lines of pure declarative manifest data — `Vec`s of string literals serialized in struct-declaration order (`software.rs:456-505`). It cannot be nondeterministic. (Side observation, not a determinism bug: the `behavior` and `invariant` field lists duplicate their entries — `:28-53` repeats `:41-53`, `:71-74` repeats `:75-78`. Deterministically duplicated, but it feeds duplicate registry entries and is worth a fix in its own right.)

## 3. Findings — what is NOT deterministic

### F1 (HIGH, live): `detect_cycles` DFS root order iterates a randomized `HashSet` → diagnostic **sets** differ across runs

`compile.rs:677` builds `node_ids: HashSet<&str>`; `compile.rs:727` seeds DFS with `for &id in &node_ids`. The algorithm's propagation rule (`:714-718`: a node joins `cycle_members` if any DFS child is a member) marks **ancestors of the discovered cycle in the DFS forest** — and which nodes end up as ancestors depends on the (randomized) iteration order.

**Proof** (verbatim port of `compile.rs:690-736`, standalone rustc binary, 200 runs, same binary, same input — repro kept at `.plugin/research/probe_detect_cycles.rs`):

```
input: A→B, B→C, C→B   (A feeds a cycle)
129x ["B", "C"]        ← feeder A not flagged
 71x ["A", "B", "C"]   ← feeder A flagged as cycle member
input: B→C, C→B        (pure cycle)
200x ["B", "C"]        ← stable
```

The cycle itself is always fully reported (both back-edge endpoints plus the tree path between them are always inserted); what varies is **whether nodes that merely point into the cycle get flagged**, and every flagged node becomes a full `Diagnostic` with code/severity/span.

This is live on every `compile_with_runtime` consumer via step 12 (`compile.rs:256,611-647`), against real rules: **E007/E015/E016/W045/W092** (module/milestone/deliverable/feature/release dependency cycles, `builtins/product.rs:554-582`) and **W065/W066** (refinement/process cycles, `builtins/formal.rs:336-349`). Dependency edges into a cycle are the *common* case in real specs, not an edge case. The `sorted_members.sort()` shows the intent was determinism — the set was fixed before the sort.

### F2 (MEDIUM, latent): `file_reference_fields` is a `Vec` built from a `HashSet`

`compile.rs:161-167`: `file_ref_fields` is collected through `HashSet::into_iter()`; `validate_file_references` iterates that `Vec` per node (`specforge-validator/src/file_ref.rs:16-17`), so with ≥2 file-reference fields the per-node diagnostic interleaving is process-random. **Inert today**: all four builtin manifests hard-code `file_reference: false` (verified in each mirror's `fd_defaults`). It ignites the first time a third-party plugin sets `file_reference: true` on two fields — i.e., precisely the registry-distribution future R-1/R-4 bet on.

### F3 (MEDIUM, adjacent): `analyze --prove` shells out to system z3

`specforge-cli/src/prove.rs:154-164`: runs whatever `z3` is on `$PATH`; availability degrades silently (returns `Option`). Verdicts on hard instances and any core/model output vary by solver version and machine — the analyze funnel's output is machine-dependent, breaking "same tree → same report" across contributors/CI.

### F4 (LOW, forward-looking): guest findings order is HashMap-ordered — stable per blob, unstable across guest rebuilds

The formal guest emits E041 order from `refines.keys()` (`lib.rs:198`), W031 from `&depth` (`lib.rs:235`), W029 from `&produced` (`lib.rs:272`). On rustc ≤ 1.84 wasm32 std seeds HashMaps with constants `(1, 2)` → frozen per vendored blob. **Rust ≥ 1.85 changed this**: `hashmap_random_keys` now derives the seed from *allocation addresses* (`library/std/src/sys/random/unsupported.rs` @ 1.85) — de facto still stable under wasmtime's deterministic linear memory, but no longer guaranteed by construction. The E041 message even names whichever node the DFS re-entry happened to hit (`lib.rs:180-187`), so a guest rebuild can silently re-baseline snapshot tests. Rebuilding guests with a newer toolchain is exactly what R-4 reproducibility work would do.

## 4. What breaks if two runs produce different diagnostics

1. **Exit-code instability under `--strict`.** `check.rs:33-39` promotes `Warning → Error`; `has_errors` then drives exit status. A feeder node flagged into a **W-severity** cycle rule (W045/W065/W066/W092) exists in run 1 and not run 2 → `specforge check --strict` returns 1 then 0 on an unchanged tree. CI goes green/red on identical input.
2. **Snapshot tests flake.** Any snapshot of `check --json`, analyze reports, or LSP-published diagnostics re-rolls the dice per process (Rust `RandomState` is seeded per process from the OS). The 3,070-test suite stays green until a fixture has a feeder-into-cycle shape, then becomes a permanent source of phantom failures.
3. **AI-agent loops (the primary consumer) chase phantom regressions.** The product's stated consumer is agents (evidence pack §1.5). An MCP `analyze` that returns a different E007 set on re-run teaches agents that the tool is unreliable; agents re-plan, re-run, or "fix" nonexistent regressions. Deterministic diagnostics are load-bearing for that consumer specifically.
4. **LSP churn.** `publish_diagnostics` diffs on every keystroke-recompile; F1 makes editor squiggles appear/vanish non-reproducibly, which users read as flakiness of *their spec*.
5. **Unverifiable bug reports / irreproducible check runs.** A user cannot minimize a failing case: the failure disappears on retry. R-6 ("deterministic, snapshot-testable analyze output") is currently violated in the strict sense, for any spec whose graph has a cycle with incoming edges — and R-6 is a hard requirement the runtime decision must preserve.
6. **R-4 verifiability is split.** The registry path (blobs, sha256, signing) is reproducible; the *output* plugins produce is not — "verifiable" at install time but not at analysis time.

## 5. How each candidate runtime affects this

Determinism decomposes into two independent layers:

- **Layer A — host pipeline ordering.** F1/F2 live here. *No candidate changes this.* Replacing Wasm keeps the bug; adding any scripting tier alongside it keeps the bug. This must be fixed in `compile.rs` regardless of the decision.
- **Layer B — plugin execution semantics.** This is where the candidates genuinely differ:

| Runtime | Guest-side iteration order | Deterministic timeout mechanism | Net determinism effect vs today |
| --- | --- | --- | --- |
| **KEEP_WASM** | Rust HashMap: fixed-seed ≤1.84; allocator-address ≥1.85. Stable per blob; drift only on guest rebuild | wasmtime `consume_fuel`: "can be used to **deterministically** prevent infinitely-executing WebAssembly code" (docs.wasmtime.dev, `Config`) — the correct implementation path if C7-10 is ever fixed; wall-clock `max_execution_ms` would *add* run-to-run variance | Baseline. Integer-only workloads sidestep the classic wasm NaN-nondeterminism caveat. Mitigate F4 by sorting guest findings host-side |
| **LUA** (`mlua`) | `pairs` order "is not specified" (Lua 5.4 manual, §`next`/`pairs`) — in practice stable per interpreter build/table, but authors get no guarantee; same hazard class as HashMap, enforced by convention | Stock Lua/LuaJIT: no instruction-count fuel (LuaJIT lacks hooks; Luau has them) → enforced timeouts would be wall-clock → variance risk | Neutral-to-slightly-worse; requires authoring rules ("sort keys before emitting findings") |
| **PYTHON** (PyO3) | **Worse by default**: str/bytes hash randomization is process-random (PYTHONHASHSEED; docs.python.org `using/cmdline.html#envvar-PYTHONHASHSEED`) → dict/set iteration over string keys re-rolls *every interpreter instance*, i.e. every plugin call unless the embedder pins the seed | Signal-based interruption only; coarse | Strictly requires `PYTHONHASHSEED=0` (or fixed seed) at embed time + a documented "sort your findings" contract, or third-party plugins become F1-class nondeterminism factories |
| **TYPESCRIPT** (`deno_core`/QuickJS) | **Best in class**: ECMAScript `Map`/`Set`/object property iteration is insertion-ordered *by spec* — defined, not merely stable | No fuel model; host-initiated interrupt is wall-clock | Engine-version drift can't reorder spec-mandated iteration; guest determinism survives recompilation entirely — eliminates the F4 class |
| **MULTI** | Union of the above | Union | Worst surface area: host-side canonicalization becomes mandatory rather than advisory, and per-tier determinism docs are needed |

The decisive observation: **today's plugin outputs are already only "arbitrarily but stably" ordered** (guest HashMap order), while the *only* per-run randomness in the product is in the host (F1). A host-side canonicalization — sort `ctx.diagnostics` by `(span.file, start_line, code, message)` at pipeline exit, and sort plugin-pass findings on receipt (`analyze.rs:182-193`) — would make R-6 hold under *every* candidate runtime and simultaneously neutralize F4. Conversely, no runtime choice fixes F1.

**Recommended fixes** (all small, runtime-independent, and they decide nothing about the runtime — by design):
1. F1: iterate `graph.nodes()` (already sorted by id) instead of the `HashSet` when seeding DFS in `detect_cycles` (`compile.rs:727`); or better, compute SCCs so membership is order-independent by construction.
2. F2: `file_reference_fields` → sorted `Vec`/`BTreeSet` (`compile.rs:161-167`).
3. Canonicalize diagnostics once at pipeline exit and on pass-result deserialization (covers every current and future plugin tier).
4. Sort findings in the formal guest before returning (cheap insurance against the rustc ≥ 1.85 seed change when blobs get rebuilt).
5. If C7-10 (`max_execution_ms`) is ever implemented: use fuel/epoch, never wall clock — the current *unenforced* state is paradoxically the determinism-safe one.

## 6. Bottom line

The validation rules and declarative pass machinery are deterministic by construction — sorted files, sorted nodes, sorted patterns, sorted cycle members, byte-stable payloads. The compiler *passes* (host 15-step pipeline and wasm guests alike) are deterministic except for one real bug: `detect_cycles`' HashSet-seeded DFS makes the *set* of cycle diagnostics (E007/E015/E016/W045/W092/W065/W066) vary run-to-run whenever a node feeds a cycle — empirically 129×/71× over 200 identical runs — which under `--strict` changes the process exit code on an unchanged tree. That bug, plus the latent HashSet-ordered `file_reference_fields` and the system-z3 dependency of `analyze --prove`, live in the **host**, so no candidate runtime (KEEP_WASM/LUA/PYTHON/TYPESCRIPT/MULTI) fixes or causes them. Among candidates, wasm has the strongest guest-side story (per-blob-frozen iteration, fuel-based deterministic timeouts), TypeScript the best-*defined* iteration semantics, Python the worst default (hash randomization) unless the seed is pinned. Determinism therefore argues mildly for KEEP_WASM — but the load-bearing work is a one-line host fix plus host-side diagnostic canonicalization, which preserves R-6 under any choice.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The only real determinism violation is host-side (`detect_cycles` HashSet iteration, empirically proven) and survives every runtime swap; wasm additionally offers the strongest guest-stability + deterministic-fuel story, so keep it — but fix the host ordering and canonicalize diagnostics at the boundary regardless.
