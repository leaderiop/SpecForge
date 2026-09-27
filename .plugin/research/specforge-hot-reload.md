# SpecForge watch pipeline & hot reload (R-5) — analyst: research-specforge-hot-reload

All local facts verified against the working tree by direct code inspection this session.
External runtime facts cite docs read this session (wasmtime, extism, mlua, CPython, deno_core).
`web_search` was unavailable (provider error); per parent guidance, facts come from direct
primary-source reads.

## 1. The incremental core is genuinely good — and shared

`IncrementalPipeline` (`crates/specforge-watch/src/pipeline.rs:84-104`) is the single
incremental engine both `specforge watch` and the LSP drive (LSP imports it:
`crates/specforge-lsp/src/backend.rs:19`). Per-file state:

| cache | purpose |
| --- | --- |
| `sources: path -> content` | whole-file diffing to synthesize tree edits |
| `trees: path -> tree_sitter::Tree` | retained parse trees fed back into `parse_incremental` |
| `parsed_files: path -> SpecFile` | cached ASTs (survive unchanged across rebuilds) |
| `file_diagnostics` | per-file diagnostic sets, diffed per cycle |
| `graph_config: GraphConfig` | **frozen at startup** — keywords, bidirectional pairs, body-parser suppressions, single-ref fields |
| `import_dag` | change → invalidation set (the edited file + transitive importers) |

### How a rebuild works (`apply_invalidated`, pipeline.rs:295-438)

1. `rebuild(changed)` / `update_open_file(path, content)` compute an **invalidation set**
   from the import DAG (`pipeline.rs:260,280`).
2. For each invalidated file: if a retained tree + old text exist, `edited_tree_for_replacement`
   (pipeline.rs:29-76) computes longest common byte prefix/suffix, builds a
   `tree_sitter::InputEdit`, and `parse_incremental` re-parses **only the changed subtrees**.
   Unchanged files are never touched; deleted files are evicted from all caches.
3. The graph is then rebuilt **cold but from cached ASTs**:
   `build_graph_with_config(&all_spec_files, &self.graph_config)` (pipeline.rs:361-363).
   Parse is incremental; the graph layer is a full rebuild over ~cached `SpecFile`s —
   no re-reading or re-parsing of source.
4. `compute_graph_delta_with_config` diffs old vs new graph into `GraphDelta`
   (`delta.rs:31-38`): added/removed/modified nodes (modified carries `changed_fields`
   with old/new values), added/removed edges, `affected_files`.
5. Per-file diagnostics are diffed → `changed_diagnostic_files`; consumers publish only
   those files (watch JSON output, LSP `publish_diagnostics`).
6. Optional `verify_incremental` runs a cold rebuild and compares counts + delta
   (pipeline.rs:403-429) — the deterministic-correctness guard (R-6 adjacent).

## 2. The two consumers

### `specforge watch` (`crates/specforge-cli/src/watch.rs:16-211`)

- Cold build via `pipeline::compile` → `build_runtime` loads the wasm runtime
  **once** (`watch.rs:18`), manifests → `GraphConfig` frozen into the pipeline
  (`watch.rs:65-72`).
- `SpecWatcher` (notify) watches the spec root recursively, **filters to `.spec`
  extension only** (`crates/specforge-watch/src/watcher.rs:85`), 50 ms debounce
  batching (watcher.rs:45).
- Loop: `pipeline.rebuild(&batch, read_file)` → prints delta/error summary
  (`watch.rs:153-208`).
- **What the loop computes**: graph-build diagnostics (E001/E003/duplicates/I004)
  + W003 import cycles only. The declarative validation patterns, custom-rule
  engine, and compiler passes are **not** in the incremental loop — `ctx`
  (the full `CompilationContext`) is used only for manifests/registries at
  startup; its diagnostics are discarded for output.

### LSP `did_change` (`crates/specforge-lsp/src/backend.rs:877-941`)

1. Content changes applied to the buffer **immediately** (incremental ranges or whole
   buffer swap, backend.rs:883-897) so completions/hover stay current.
2. Any pending debounced reparse for the URI is aborted; a new 150 ms debounced task is
   spawned (`DEBOUNCE_MS`, backend.rs:34, 900-940) — trailing-edge debounce per URI.
3. The task calls `parse_and_update` (backend.rs:232-483) →
   `pipeline.update_open_file(path, Some(buffer), disk_read)`: the open buffer is
   authoritative, transitive importers re-read from disk (pipeline.rs:271-290).
4. Diagnostic layers published per file: pipeline (parse/graph, byte-identical to CLI),
   `specforge_validator::validate(graph)`, **declarative extension validation patterns
   re-executed every edit** (`execute_pattern` over `build_validation_entities`,
   backend.rs:435-447 — E006 required fields, W001-W011), E022 mistyped references,
   plus completion-supporting layers.

Registry loading (`load_registries`, backend.rs:161-227) happens **once at
`initialized`** (backend.rs:806-807): reads `specforge.json`, builds a runtime via
`runtime_for_extensions`, calls `__describe` per extension through `ProtocolHost`,
populates kind/field/edge registries + validation patterns. Never re-run.

**Known drift, relevant to reload:** the LSP's `GraphConfig` passes
`known_extension_keywords: HashMap::new()` (backend.rs:140) while the CLI fills it from
manifests (`watch.rs:25-33`) — so I004 keyword hints already disagree between surfaces.
Any reload design must regenerate `GraphConfig` **everywhere** or the drift widens.

## 3. What happens when extension code changes (not the spec)

**Answer today: nothing, at every layer.**

| layer | why it's blind |
| --- | --- |
| CLI watcher | `extract_spec_paths` keeps only `.spec` files (watcher.rs:85) |
| LSP client watcher | registered glob `**/*.spec` only (backend.rs:795); `did_change_watched_files` early-continues on non-`.spec` (backend.rs:956) |
| runtime | `ExtismRuntime.plugins: Mutex<HashMap<name, LoadedPlugin>>` is populated at startup and never invalidated; `call_export` reuses the retained `Plugin` instance forever (`crates/specforge-extism/src/runtime.rs:94-97,117-148`) |
| manifests | `__describe` results → `ManifestV2` → registries → `GraphConfig` captured by value into the pipeline at startup; no re-describe path exists |
| custom rules | `NativeCustomRules` (host-executed since C6-11 fix, `crates/specforge-emitter/src/compile.rs:464-486`) run during cold `compile()` only — never in watch's incremental loop |
| compiler passes | only `specforge analyze` (one-shot process) runs `__pass_*`; watch/LSP never call them |
| body parsers | extension code participates **during parsing** (`call_export(ext, parse_export, body_text)` — `crates/specforge-wasm/src/lifecycle.rs:284`); the parse cache (`parsed_files`/`trees`) has no notion of "the body parser for kind X changed" |

So R-5 ("edit plugin → re-run analyze without restarting the host") is **unimplemented
end-to-end**: detection, reload, and re-analysis are all missing — not just one link.

## 4. What a reload must touch (the refresh dependency chain)

Editing plugin code can change, in causal order:

1. **Parsing itself** — only via `has_body_parser` kinds (extension-owned syntax; also
   `suppressed_parse_error_ranges` in `GraphConfig`). Worst case: re-parse affected
   entities, not whole files.
2. **`GraphConfig`** — keywords (I004), bidirectional pairs (cycle suppression),
   single-reference fields, body-parser suppressions. Cheap: rebuilt from manifests.
3. **Registries** — kinds/fields/edges/validation patterns (+ auto-E006). Cheap, host-side.
4. **Graph** — already rebuildable from cached `SpecFile`s (no re-parse) once
   `graph_config` is swapped. `IncrementalPipeline.graph_config` is a private field; a
   `set_graph_config` (or full `refresh_extensions` method that re-describes and swaps
   config in place) is the minimal seam.
5. **Analysis layers** — declarative patterns are already re-run per edit in the LSP;
   passes are full-snapshot, so "re-run" = re-serialize `PassInput { entities, edges }`
   and call each `__pass_<name>` again (no incremental pass ABI exists;
   `crates/specforge-cli/src/analyze.rs:110-210`). For the ~1.7k-entity reference graph
   that is fine; entity-scoped re-validation could be keyed off `GraphDelta`
   (`modified_nodes.changed_fields`) later.
6. **Determinism (R-6)** — unchanged: passes are pure functions of the snapshot;
   `verify_incremental` is the existing guard pattern to copy for "reload == cold build".

Note the analogous trap in `analyze` itself: it calls `pipeline::build_runtime` fresh
per invocation (analyze.rs:124), so a long-lived host that wants hot reload cannot
reuse that path today — the runtime handle and the manifests are process-startup
artifacts everywhere.

## 5. How each candidate runtime handles the reload loop (detect → reload → re-run)

### KEEP_WASM (Extism/Wasmtime)

- **Detect**: extend watcher globs to `**/*.wasm` + `specforge.json`; identity =
  content hash (sha256 infra already exists in install/integrity paths).
- **Reload**: read bytes → `read_and_validate` → `instantiate` again — the
  `plugins` map insert atomically replaces the `LoadedPlugin`
  (`runtime.rs:80-97`), and `call_export` already serializes through the mutex, so a
  swap between calls is race-free. Wasmtime compiles once per module and
  `Module::serialize`/`Module::deserialize` reload a compiled module with
  **no recompilation** ("AOT-style use case"), with no runtime tiering or
  re-optimization (wasmtime docs, `Module`). Extism 1.30.0 exposes exactly this:
  `Plugin::new_from_compiled(&CompiledPlugin)`, plus `Plugin::reset` and
  `CancelHandle` for in-flight calls (`extism 1.30.0` docs, `Plugin`). C7-02's
  "AOT cache is a byte-copy" is the exact gap: wire `has_cached_module`/`_aot_cache_path`
  to real `precompile_module` output and plugin reload becomes
  hash-miss → deserialize → instantiate (sub-ms to low-ms per 324-415 KB blob).
- **Crash containment**: already built — a trapping plugin returns `WasmTrapInfo`
  (`call_failed`), host survives and can keep serving the old or empty module.
  Memory isolation is the sandbox; capability scope is per-module host functions (C7-04
  must be fixed so reloaded modules don't inherit fs access).
- **Re-run analysis**: re-`__describe` → registries → new `GraphConfig` →
  `build_graph_with_config` over cached ASTs → re-run patterns/passes. All plumbing
  exists except the orchestration and the pipeline's frozen `graph_config` seam.
- **Fit**: the current architecture (extension output = JSON data over `call_export`,
  per-call stateless passes) is reload-shaped by construction.

### LUA (mlua)

- **Detect**: watch `.lua` text files — no build step, fastest authoring loop.
- **Reload**: trivial mechanically — drop the per-plugin `Lua` state and create a
  fresh one, or `lua.load(new_src).exec()` into a clean environment (README shows the
  load/exec pattern). Fresh-state-per-reload avoids stale-globals problems entirely.
  State creation is cheap (no compilation tier like wasm).
- **Sandbox**: the weak link. mlua's own `Lua::sandbox` is **Luau-only**
  (README, "Sandboxing"); for Lua 5.4 the host hand-rolls capability control by
  stripping `os`/`io`/`load` from globals, and CPU limiting needs interpreter hooks
  (Luau instruction budgets; vanilla Lua has none). A plugin can still spin forever —
  today's wasm story has the same hole (C7-10: `max_execution_ms` never enforced;
  extism's `fuel_consumed` exists unused), so both need a timeout story, but wasm
  also gives memory isolation per module that a shared Lua state does not.
- **Crash containment**: Lua errors are catchable values; mlua wraps Rust-callback
  panics into Lua errors (README) — host survives; memory unsafety remains the
  documented mlua caveat ("huge amount of unsafe code").
- **Re-run analysis**: identical host plumbing as wasm (describe → registries → graph).
  Determinism: pure Lua is deterministic; fine for R-6.

### PYTHON (PyO3)

- **Detect**: watch `.py` — text, no build step.
- **Reload**: `importlib.reload` semantics are documented minefields: module dict is
  **retained** (old definitions survive deletion), `from … import` names elsewhere are
  **not rebound**, existing instances keep old classes, and it is **not thread-safe**
  (CPython docs, `importlib.reload`). In practice a sandboxed-feeling reload means
  fresh subinterpreter or fresh process, not `reload()`.
- **Sandbox/runtime**: none by default — stdlib is ambient fs/net/process capability
  (fails R-2 as in-process), and the GIL means plugin CPU blocks the tokio LSP/watch
  loops unless a sidecar process is used. A sidecar makes reload clean (kill +
  respawn) but then R-3 (bundled CPython per platform) and startup cost bite.
- **Re-run analysis**: same host plumbing, but serialized through the GIL;
  determinism OK for pure functions.

### TYPESCRIPT (deno_core)

- **Detect**: watch `.ts` — text, but each change needs in-process transpile
  (deno_core embeds the deno TS pipeline).
- **Reload**: each `JsRuntime` "corresponds roughly to the Web Worker concept" with
  its own V8 isolate (deno_core docs, `JsRuntime`) — reload = create a fresh runtime
  (cleanest, snapshot-backed to cut startup) or
  `load_side_es_module_from_code` to re-evaluate the module graph in an existing
  runtime (risks stale module registry state). Heap ceilings via
  `add_near_heap_limit_callback`; the permission system of the deno CLI does **not**
  come with deno_core — capability control is the host's op whitelist.
- **Cost**: V8 platform init + isolate startup is heavier than a Lua state or a
  wasmtime instantiate-from-compiled; dep weight lands in the single binary (R-3).
- **Re-run analysis**: same host plumbing; async op model overlaps awkwardly with
  today's synchronous `call_export` contract (would need block-on wrappers).

### MULTI

Every reload property above must be built and tested per runtime (N detection paths,
N reload primitives, N timeout/memory models), and cross-runtime determinism of
`analyze` output (R-6) becomes an integration matrix. Highest cost on exactly this
requirement.

## 6. Ranking strictly on R-5 (hot reload)

1. **KEEP_WASM** — reload primitive exists (`new_from_compiled` + atomic map swap),
   crash/mem isolation already in place, one artifact format to watch; work is
   orchestration + making C7-02/C7-08 real (compiled-module cache, warm instances).
2. **LUA** — simplest reload (text → fresh state), but sandbox/CPU limits must be
   hand-built, weakening the security story that R-2/R-5 share.
3. **TYPESCRIPT** — clean isolate-per-reload, but heavier startup, DIY permissions,
   biggest dep weight.
4. **PYTHON** — worst: reload semantics, GIL contention with async hosts, no in-process
   sandbox, distribution pain.
5. **MULTI** — N× everything.

Cross-cutting facts that hold for every runtime: detection is missing (spec-only
globs in both surfaces), `GraphConfig`/registries are startup-frozen with no swap
seam, and the LSP already disagrees with the CLI on `known_extension_keywords`
(backend.rs:140 vs watch.rs:25-33) — fix the seam, not just the runtime.

## Sources

- Local (inspected this session): `crates/specforge-watch/src/pipeline.rs`,
  `crates/specforge-watch/src/delta.rs`, `crates/specforge-watch/src/watcher.rs`,
  `crates/specforge-cli/src/watch.rs`, `crates/specforge-cli/src/pipeline.rs`,
  `crates/specforge-cli/src/analyze.rs`, `crates/specforge-lsp/src/backend.rs`,
  `crates/specforge-extism/src/runtime.rs`, `crates/specforge-wasm/src/lifecycle.rs`,
  `crates/specforge-emitter/src/compile.rs`, `crates/specforge-graph/src/build.rs`.
- https://docs.wasmtime.dev/api/wasmtime/struct.Module.html — compile-once,
  `serialize`/`deserialize` AOT reload, no runtime tiering.
- https://docs.rs/extism/latest/extism/struct.Plugin.html (1.30.0) —
  `new_from_compiled`, `reset`, `cancel_handle`, `fuel_consumed`.
- https://github.com/mlua-rs/mlua — load/exec model, `Lua::sandbox` Luau-only,
  panic-wrapping safety caveats.
- https://docs.python.org/3/library/importlib.html#importlib.reload — retained module
  dict, stale references, non-thread-safety.
- https://docs.rs/deno_core/latest/deno_core/struct.JsRuntime.html — worker-like
  isolate per runtime, module load from code, heap-limit callback.

## Bottom line

The watch/LSP pipeline is well-engineered for **spec** hot reload — tree-sitter
incremental parsing, import-DAG invalidation, graph delta, per-file diagnostic
diffing — but it is structurally blind to **plugin** changes: watchers filter to
`.spec`, extension state (wasm instances, manifests, registries, `GraphConfig`) is
startup-frozen with no invalidation seam, and analysis layers beyond graph-build
(rules, passes) never run in the incremental loop. R-5 therefore needs a new
orchestration path regardless of runtime: watch plugin artifacts (hash-keyed),
re-describe → swap registries + `GraphConfig` → rebuild graph from cached ASTs →
re-run patterns/passes. Wasm is the runtime where that path is shortest, because the
reload primitive (compile-once/instantiate-many with `Plugin::new_from_compiled`),
crash containment (`WasmTrapInfo`), and memory isolation already exist; its gaps
(C7-02 fake AOT cache, C7-08 ledger engine pool, C7-10 no timeout) are exactly the
hot-reload gaps, so fixing R-5 and fixing the audit cluster are the same work.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The incremental pipeline is spec-only and startup-frozen, but wasm's compile-once/instantiate-many reload primitive plus existing trap isolation and memory sandbox make "detect → swap → re-describe → rebuild" the shortest path of any candidate to real R-5.
