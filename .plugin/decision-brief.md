# Decision Brief — SpecForge Plugin Runtime

**Question:** Should SpecForge keep WASM (Extism/Wasmtime) as the plugin runtime, or adopt a dedicated embedded plugin language (Lua, Python, TypeScript) — or support multiple languages?

## Governing constraint (from the project owner — HARD)

> **R-1: There is no first-party plugin. All plugins are the same.**

The four extensions shipped with SpecForge (`@specforge/product`, `@specforge/software`,
`@specforge/governance`, `@specforge/formal`) are plugins like any third-party one.
There is **no first-party/native/trusted tier**. Whatever runtime is chosen:

- must run the four builtins' real workloads (validation rules, compiler passes with
  genuine logic, collectors, the describe protocol) under the **same** security model
  as any community plugin,
- must make the current parallel implementations (native Rust mirrors in
  `crates/specforge-emitter/src/builtins/`, embedded wasm blobs, guest crates)
  converge into **one** mechanism, or justify why each remains,
- must not assume "the host authors the plugins, so we trust them".

## The decision

Pick the plugin runtime for SpecForge extensions going forward:

| Label | Meaning |
| --- | --- |
| `KEEP_WASM` | Stay with Extism/Wasmtime. Plugins authored in Rust (or any language targeting `wasm32-unknown-unknown`) via the existing SDK macros; guest blobs distributed through the registry. Fix the audit gaps inside this model. |
| `LUA` | Replace the wasm runtime with an embedded Lua interpreter (e.g. `mlua`, Lua 5.4/LuaJIT). Plugins are `.lua` scripts. Sandbox via interpreter capability limits. |
| `PYTHON` | Embed CPython via PyO3 (or run a sidecar interpreter). Plugins are `.py` modules/packages. |
| `TYPESCRIPT` | Embed a JS/TS engine — `deno_core` (V8), QuickJS, or a Node sidecar — with TypeScript as the authoring surface. |
| `MULTI` | Support several of the above simultaneously behind one protocol (e.g. keep wasm + add a scripting tier). |

## What plugins do today (the workload any runtime must carry)

Each extension contributes, via the `describe_*` manifest and guest exports:

- **Entity kinds, edge types, fields, enhancements, surfaces, feature flags** — declarative metadata (manifest)
- **Validation rules** — declarative patterns (host-executed) **and** custom logic (`validate__*` functions receiving entities)
- **Compiler passes** — `__pass_<name>` exports receiving the entity graph snapshot and returning diagnostics; the `@specforge/formal` guest implements 4 passes with real logic (condition_check, layering_verify, event_graph_analyze, coverage_tracking) — several hundred lines of Rust each
- **Collectors** — `collect__*` exports gathering test results from external formats
- **Migration hooks**, prompts, grammar/body-parser contributions

Host context available to plugins: the compiled entity graph, spans, kind/field
registries. The product's primary consumer is **AI agents**; specs and plugins are
co-authored by AI coding agents and humans.

## Hard requirements

- **R-1** (above): one runtime, all plugins equal.
- **R-2**: untrusted third-party plugins must be sandboxable — no ambient filesystem,
  network, or process access by default; capability-scoped access only.
- **R-3**: single-binary cross-platform distribution of the host (macOS arm64, Linux x64
  at minimum) — a runtime that requires system packages per user machine fails.
- **R-4**: plugins are distributed through the signed registry (publish → verify →
  install) and must remain verifiable and reproducible.
- **R-5**: hot reload in `specforge watch` (edit plugin → re-run analyze without
  restarting the host).
- **R-6**: deterministic, snapshot-testable plugin output for the analyze pipeline.

## Evaluation criteria

1. Safety & sandboxing under R-2
2. Authoring ergonomics for AI agents and humans
3. Performance: cold start, per-call overhead, memory
4. Distribution under R-3 (binary size, system dependencies)
5. Rust embedding maturity & maintenance risk
6. Host API quality: graph access, large payloads, error handling
7. Hot reload under R-5
8. Debuggability for plugin authors
9. Migration cost from the current 4 wasm builtins + SDK
10. Binary size / dependency weight (wasmtime ≈ heavy)

## The known audit record (e87a665 → 30f82d2)

Full findings in `.reports/js/data.js` (clusters C7, C8, C14) — summarized in
[evidence.md](evidence.md). Notables: C7-00 fresh-clone build failure (fixed by
vendoring blobs), C7-02 AOT cache is a byte-copy, C7-03 no IDL (stringly JSON over
`call_export`), C7-04 sandbox fs allow-by-default, C7-08 engine pool is a ledger,
C7-09 query_scope ignored, C7-10 max_execution_ms unused, C7-11 three parallel
implementations, C8-06 publish atomicity (fixed), C8-09 integrity (fixed),
C14-03/C14-04 blocking I/O in async contexts.

## Output contract for analysts

Every analysis file MUST end with:

```
## Verdict

**Verdict:** KEEP_WASM | LUA | PYTHON | TYPESCRIPT | MULTI
**Confidence:** <1-5>
**One-line rationale:** <...>
```
