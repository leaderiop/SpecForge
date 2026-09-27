# Consumer Surfaces — How CLI, LSP, and MCP Interact with Extensions

**Scope:** `crates/specforge-cli/src/analyze.rs`, `crates/specforge-lsp/src/backend.rs`,
`crates/specforge-mcp/src/tools/mod.rs` (+ their direct collaborators: `specforge-cli/src/pipeline.rs`,
`specforge-lsp` registry loading, `specforge-mcp` compile/lifecycle/registry/operations).
**Method:** direct code inspection at working tree rev ≈ `4c9e9f2`. All claims below cite files/lines.

## 1. The shared substrate (what all three sit on)

Every surface talks to extensions through one trait and one protocol, but over **two different
backends** — this is the single most important fact for the runtime decision:

- **Trait:** `specforge_wasm::WasmRuntime` (`call_export(name, export, payload) -> WasmCallResult::{Ok, Trap}`,
  `load_module`, `has_cached_module`). Stringly-typed JSON, no IDL (C7-03).
- **Protocol:** `specforge_wasm::protocol::ProtocolHost` (`crates/specforge-wasm/src/protocol/host.rs`) —
  `handshake()` → `__handshake` export (semver major check), `describe(name, category)` → `__describe`
  export per category, `load_protocol_extension()` = handshake + describe-all.
- **Backend A — `ExtismRuntime`** (`crates/specforge-extism`): real wasmtime 43 execution of the
  vendored builtin blobs plus any `name=path/to/x.wasm` entry in `specforge.json`
  (`crates/specforge-cli/src/pipeline.rs:16-49`).
- **Backend B — `BuiltinRuntime`** (`crates/specforge-wasm/src/builtin.rs:25-124`): a `WasmRuntime`
  **implemented in native Rust** over the mirror structs (`ProductExtension`, `SoftwareExtension`,
  `GovernanceExtension`, `FormalExtension`, `RustExtension`, `TypeScriptExtension` in
  `crates/specforge-emitter/src/builtins/`). It serializes typed native responses to the same wire
  format; unknown exports → `Trap{kind:"unknown_export"}`; it additionally serves a native
  `call_analyzer` hook (`builtin.rs:108-117`) that wasm guests cannot reach through this backend.
- **Backend selector:** `specforge_emitter::builtins::runtime_for_extensions(names)`
  (`crates/specforge-emitter/src/builtins/mod.rs:31-57`) maps only the six `@specforge/*` names to
  native mirrors and **silently skips everything else**.

**Correction to evidence.md §1.2:** the claim "the CLI/LSP/MCP production path loads the wasm blobs,
not these mirrors" is wrong as of the current tree. Only the CLI builds an `ExtismRuntime`. The LSP
(`backend.rs:185`) and the emitter's plain `compile()` — which is exactly what MCP calls
(`specforge-mcp/src/compile.rs:26-29` → `specforge-emitter/src/compile.rs:47-50`) — both use
`runtime_for_extensions`, i.e. the **native mirrors**. Two of the three production surfaces never
instantiate wasmtime.

## 2. Surface-by-surface: what each needs from extensions

### 2.1 CLI — `specforge analyze` (`crates/specforge-cli/src/analyze.rs`)

The only surface that executes guest *logic* in wasm. Needs, in order:

1. **A wasm runtime with real modules.** `pipeline::compile` → `build_runtime`
   (embedded builtin blobs by name + `.wasm` paths from config) → `compile_with_runtime`
   (pipeline.rs:51-54). `run_extension_passes` builds a **second** `ExtismRuntime` instance
   (analyze.rs:124) for pass dispatch.
2. **Describe category `"passes"`** per manifest: `host.describe(&manifest.name, "passes")` →
   `Vec<CompilerPassDescriptor>` (analyze.rs:167-173). A manifest that fails to describe is
   silently skipped (`continue`).
3. **`__pass_<name>` exports** executed against a `{entities, edges}` JSON snapshot built host-side
   from `build_validation_entities` + graph edges, with a host-computed `testable` flag from
   `kind_registry.get(kind).supports_verify` (analyze.rs:126-156). Response must deserialize as
   `Vec<Diagnostic>`; malformed JSON → warning, trap → warning (analyze.rs:180-206). Extensions can
   never fail the run — they can only add findings or warnings (R-6 posture).
4. **Ordering metadata:** `after`/`before` constraints resolved by a host-side stable Kahn
   topological sort (`order_passes`, analyze.rs:35-101). Constraints referencing host phases or other
   extensions' passes are ignored; a cycle falls back to declaration order with a stderr warning.
5. **Lifecycle beyond analyze** (rest of the CLI crate, not this file): `login/add/update/publish/
   search` via `HttpRegistryClient`, lock file, `install_extension`, sha256 integrity verification
   (crates/specforge-cli/src/{add,login,publish,search,update,migrate}.rs). The CLI is the only
   surface wired to the registry at all.

**Output coupling:** extension pass reports flow through the same `Report` struct as built-ins; in
`--strict` their warnings are upgraded to errors (analyze.rs:319-335); they change the JSON document
shape and the process exit code (analyze.rs:383). Extension findings are CI-visible contract.

### 2.2 LSP — `crates/specforge-lsp/src/backend.rs`

Consumes extensions as **pure metadata**; never executes guest code on any path:

1. **Manifest → registries, once per session.** `load_registries` (backend.rs:161-227) reads
   `specforge.json`, builds a `BuiltinRuntime` via `runtime_for_extensions` (native mirrors — wasm
   blobs never load), runs `load_protocol_extension` per name, `populate_registries` → kind/field/
   edge registries. Called only from `initialize` (backend.rs:807).
2. **Declarative `validation_rules`.** Parsed via `validation_engine::parse_all_rule_patterns`
   (backend.rs:210-215) plus auto-generated E006 required-field rules, then **host-executed** per
   reparse with `execute_pattern(pattern, &entities, None)` (backend.rs:439-447). The `None` is the
   wasm-validation slot — **custom `validate__*` guest functions cannot fire in the LSP**; only
   declarative patterns do. `CycleDetection` is deliberately skipped here (backend.rs:440-444).
3. **Kind/field registries** gate the workspace: E024 unknown-kind and W020 unknown-field detection
   only fire "when registries are populated" (backend.rs:359-433); reference-typed manifest fields
   seed the parser config (backend.rs:115-118). No extensions loaded ⇒ the LSP degrades to
   structural-only mode.
4. **No reload.** `did_change_watched_files` filters `.spec` files only (backend.rs:955-958); a
   `specforge.json` edit is invisible until the editor restarts. R-5 hot reload does not exist on
   this surface today.
5. **Third-party extensions are invisible:** `runtime_for_extensions` silently drops non-builtin
   names, and `load_registries` never loads `.wasm` paths. A community extension that works in the
   CLI simply no-ops in the editor — same project, different diagnostics.

Latency profile: registries load once, but the declarative rules re-run on every debounced
`did_change` (150 ms) and on every save, in the async server thread (C14-03 flagged the blocking
I/O family).

### 2.3 MCP — `crates/specforge-mcp/src/tools/mod.rs` (+ compile/lifecycle/registry/operations)

The broadest *manifest* consumer and the shallowest *execution* consumer:

1. **Full compile via native mirrors.** `initialize` (lifecycle.rs:50-65) and every
   `specforge.validate`/`specforge.analyze` re-compile (validate.rs:33-52, analyze.rs:30-50) call
   `compile_project` → `specforge_emitter::compile` → `BuiltinRuntime`. Custom rules execute through
   the host-native `NativeCustomRules` shim (`specforge-emitter/src/compile.rs:464-620`) — the
   post-C6-11 dispatch that bypasses guest `validate__*` exports entirely.
2. **`surfaces` category** is the MCP-exclusive consumption: `register_extension_surfaces`
   (registry.rs:18-40) converts `SurfaceContributions.mcp_tools` + resources into tool/resource
   descriptors, category `"extension"`. Recompiles tear them down (`retain(category != "extension")`,
   `uri !~ specforge://ext/`) and re-register.
3. **…but extension tools never execute.** `handle_tool_call` recognizes a registered extension
   tool and answers: "registered but requires a Wasm runtime for execution"
   (tools/mod.rs:116-131). Registration is theater today.
4. **`specforge.analyze` refuses extension passes by design:** "Extension-owned compiler passes are
   not dispatched here: they require a Wasm runtime for execution. Use `specforge analyze` from the
   CLI" (tools/analyze.rs:10-15). Built-in passes only.
5. **Manifest metadata feeds the AI-facing vocabulary:** `state.manifests` drives inference prompts
   (`kinds_info`, per-kind inference guides — prompts/infer.rs:46-83), `specforge.outline_extensions`
   (OutlineIntermediate_from_manifests), `specforge.model` extension grouping, schema derivation.
6. **`analyzer_contributions` are the one live logic call:** `specforge.infer_gaps` builds a
   `BuiltinRuntime` and dispatches `call_analyzer` through `scanner_dispatch::scan_source_files`
   (tools/infer_gaps.rs:58-64) — native Rust analyzers (`@specforge/rust`, `@specforge/typescript`),
   no wasm.
7. **Management tools are stubs:** `add_extension_op` returns `{"installed": true}` without touching
   the registry (operations/mod.rs:143-168); `remove_extension_op`'s orphan check is
   `.any(|_| false)` (operations/mod.rs:198); `extensions_op` returns `"extensions": []`
   (operations/mod.rs:254-256); `doctor_op` fabricates cache checks (operations/mod.rs:281-288).

## 3. Needs matrix

| Extension contribution | CLI | LSP | MCP |
| --- | --- | --- | --- |
| Manifest metadata (kinds/fields/edges/rules) | via compile (wasm) | **primary** (native) | **primary** (native) |
| `__handshake`/`__describe` protocol | wasm blobs | native mirrors | native mirrors |
| `__pass_<name>` guest execution | **yes (only consumer)** | no | no — documented refusal |
| `validate__*` custom rules | host-native (`NativeCustomRules`) | **no** (`execute_pattern(…, None)`) | host-native (`NativeCustomRules`) |
| Declarative rule patterns | via compile | **primary**, per-keystroke | via compile |
| `collect__*` collectors | dispatch machinery in `specforge-wasm` (`dispatch_collector` over `WasmRuntime`); not referenced by these three files | no | routed op exists; not studied here |
| `surfaces` (MCP tools/resources) | no | no | **register only; execution refused** |
| `analyzer_contributions` (`call_analyzer`) | no | no | **yes, native-only** |
| Prompts/inference guides | no | completion vocab via registries | **primary** |
| Registry lifecycle (add/publish/search/lock) | **full** | none | stubs |

## 4. Would a runtime change affect the surfaces differently? Yes — six asymmetries

1. **The "runtime" is load-bearing in different degrees.** Today: CLI = wasmtime execution; LSP and
   MCP = native mirrors. A *keep-wasm* internal fix (engine pool C7-08, AOT C7-02, sandbox C7-04,
   `max_execution_ms` C7-10) changes observable behavior **only on the CLI analyze path**, because
   that is the only path that runs wasm. Symmetrically, swapping to Lua/Python/TS leaves LSP and MCP
   byte-identical unless the migration explicitly includes `runtime_for_extensions` /
   `BuiltinRuntime` — which *are* the extension runtime for two of three surfaces. The decision
   brief's R-1 convergence is therefore mostly about the emitter/mirror layer, not the wasmtime
   layer.
2. **Different latency budgets.** LSP re-executes rule evaluation every 150 ms debounce; it got the
   native-mirror treatment precisely because per-call wasm instantiation is unattractive there. CLI
   analyze is batch — cold start amortizes, a 30 s budget is meaningful. MCP is request/response
   (recompile per mutating tool call). A replacement runtime must make manifest describe effectively
   free (or cacheable across the LSP session) or the cheap path will regrow native mirrors and
   C7-11 with it.
3. **Hot reload (R-5) exists on zero surfaces in equal form.** `specforge watch` is CLI-side. MCP
   *de facto* reloads: every validate/analyze recompiles and re-registers surfaces. The LSP has no
   reload at all (initialize-only, `.spec`-only watcher). A runtime swap alone fixes none of this;
   each surface needs its own reload story, and the LSP's is the only one that is real work.
4. **Determinism (R-6) can silently fork.** Same spec, same extensions: CLI derives diagnostics from
   wasm guests; LSP/MCP derive them from native mirrors. Mirror/blob drift produces different
   findings per surface; only the guard tests (`extension_json_sync`, `builtin_blob_sync`) hold the
   line, and they pin the *builtins*, not third-party plugins. One runtime with one describe
   mechanism collapses this fork; a MULTI decision institutionalizes it.
5. **Capability is already inconsistent per surface, independent of runtime choice.** Extension
   compiler passes: CLI-only (MCP punts explicitly). Extension MCP tools: registered, never
   executable. Third-party extensions: CLI-only (LSP silently skips). Under R-1, "all plugins equal"
   is currently false in *surface space*, not just runtime space — whichever runtime wins must also
   decide which capabilities each surface dispatches, or the divergence persists after migration.
6. **Sandboxing (R-2) is a CLI-only property today.** The native mirror tier has no sandbox because
   it is host-compiled code — which under R-1 is exactly the "trusted first-party tier" the project
   owner forbade. The builtins currently enjoy a no-sandbox native path on LSP/MCP that no community
   plugin can access. Any target state must either give every surface the same sandboxed execution
   or reduce the LSP/MCP contract to static manifest data only (which is, in fact, all they consume
   today — except MCP's `call_analyzer`).

## 5. Bottom line

The three surfaces do not share one extension runtime; they share one *protocol* over two runtimes.
The CLI needs a sandboxed, batch-grade logic executor (`__pass_*`, wasm today) plus the registry
lifecycle. The LSP needs fast, session-stable manifest metadata and declarative rules — and nothing
else. The MCP needs the richest metadata (kinds, surfaces, prompts, analyzers) but executes no
extension logic, refusing it with a "needs a Wasm runtime" error that is simultaneously true (its
runtime is native) and misleading (the wasm runtime is right there in the CLI). A runtime change is
therefore not one migration but two: the visible wasmtime swap on the CLI, and the quieter but
larger retirement (or sandboxing) of the native-mirror tier that LSP and MCP actually run on. The
consumer-surface evidence favors a single mechanism whose describe path is nearly free — metadata
consumption dominates two of three surfaces — with sandboxed execution reserved for the batch tier.

## Verdict

**Verdict:** LUA
**Confidence:** 3
**One-line rationale:** Two of the three consumer surfaces already run extensions as cheap native
metadata with no sandboxed logic, and the third only batch-executes guest functions — one embedded
interpreter with near-free describe and capability-scoped execution satisfies all three, while
KEEP_WASM would keep the native-mirror tier alive for LSP/MCP latency and perpetuate C7-11.
