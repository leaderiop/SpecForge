# Plugin Graph API Surface — What Plugins Need From `specforge-graph` and How to Expose It

**Analyst:** research-specforge-graph-api · **Rev:** tree @ 4c9e9f2 (evidence-pack baseline) · **Date:** 2026-09-27

## Scope

Assignment: read `crates/specforge-graph/src/graph.rs` (549 ln) and `crates/specforge-emitter/src/compile.rs` (763 ln), enumerate the operations plugins actually need, design a plugin graph API per candidate runtime (wasm / Lua / Python / JS), and derive the minimum viable surface. Supporting files read: `specforge-parser/src/ast.rs` (field model), `specforge-protocol-types/src/lib.rs` (wire types), `specforge-extension-sdk/src/lib.rs:659-769` (pass ABI), `specforge-cli/src/analyze.rs:100-210` (pass dispatch), `extensions/formal/src/lib.rs` (the only logic-bearing plugin), extension `describe_*.json` manifests.

## 1. What the graph is

`Graph` (graph.rs:24-37) is a `HashMap<Sym, Node>` plus a flat `Vec<Edge>` with two adjacency indices (`source_index`, `target_index`: sym → edge indices). `Sym` is an interned string (`Spur`, interner.rs:12) — plugin-visible IDs are plain strings; interning is host-internal. `Node` = `{id, kind, title, fields: FieldMap, source_span, methods: Vec<MethodDecl>}`; `Edge` = `{source, target, label}` (all `Sym`).

`FieldMap` is an **ordered Vec** of `FieldEntry {key, value: FieldValue, annotations}` — 12 `FieldValue` variants (ast.rs:133-153): `String`, `ReferenceList`, `VariantList`, `StringList`, `MixedList`, `Block(FieldMap)`, `VerifyList`, `Expression`, `TypeUnion`, `Integer(i64)`, `Boolean`, `Date`, `Identifier`.

### Full accessor inventory and who uses it

| Operation (graph.rs) | Complexity | Plugin-reachable today? | Real callers |
| --- | --- | --- | --- |
| `node(id)` lookup | O(1) | yes — via native mirror only (see §2b) | `NativeCustomRules` (compile.rs:509, 523, 551, 583) |
| `nodes()` — all, id-sorted | O(n log n) **per call** | via snapshot push | compile.rs:177, 191, 213, 218 (4× per compile, re-sorts each time); analyze.rs:126 |
| `nodes_by_kind(kind)` | O(n) scan | via snapshot push | MCP `entities_by_kind`, `tools/list.rs` (host-only) |
| `filter_nodes(pred)` | O(n) scan | no (generic; closures can't cross runtimes) | host tests only |
| `edges_from(id)` / `edges_to(id)` | O(deg) via index | counts only (`PassEntity.incoming/outgoing_edge_count`) | `build_validation_entities` (compile.rs:395-396) |
| `edges()` — full slice | O(1) | yes, in `PassInput.edges` | analyze.rs:145; formal passes |
| `nodes_in_file(file)` | O(n) scan | no | LSP host-side |
| `subgraph(root)` / `subgraph_depth(root, d)` | BFS | **no** — host-only | emitter `emit.rs`, `scope.rs`, `schema.rs`; MCP budget filtering |
| `invalidation_set(file, dag)` | BFS | no — host-only | watch pipeline |
| `has_cycles()` / `detect_cycles()` | DFS | no — plugins re-implement DFS over `PassInput.edges` | host `detect_cycles` in compile.rs:649; formal `layering_verify` does its own DFS |
| mutation (`add_node`, `add_edge[_checked]`, `remove_node`, `clear_edges`, `resolve_references*`) | — | **no — plugins are strictly read-only consumers** | host pipeline |

Key structural fact: **no plugin today needs traversal state or mutation.** Everything plugin-side is: iterate, filter, look up, read fields, walk edges, emit diagnostics. `subgraph`/`invalidation_set`/mutation are host-consumer operations (emitter/MCP/watch) and must not leak into the plugin API.

## 2. The plugin-facing graph surfaces that exist today

### 2a. Compiler passes — push-everything JSON snapshot

`specforge-cli/src/analyze.rs:126-181` serializes the **entire** entity list + edge list into one JSON payload (`{entities, edges}`) and calls each declared `__pass_<name>` export with the same bytes. SDK side (`specforge-extension-sdk/src/lib.rs:666-703`):

```rust
PassEntity { id, kind, fields: BTreeMap<String,String>, incoming_edge_count,
             outgoing_edge_count, span: Option<PassSpan>, testable: bool }
PassEdge   { source, target, label }        // labels are raw field names
PassInput  { entities, edges }
```

Good properties: one serialization per analyze run, reused across every pass of every extension; deterministic-by-construction shape; trivially snapshot-testable; `query_scope` (audit C7-09) could be enforced host-side by simply filtering what is pushed.

### 2b. Custom validators — an ID, and nothing else

`WasmValidationRuntime::call_custom_validator_detailed(fn_name, entity_id, entity_kind) -> CustomVerdict` (compile.rs:486-609). The plugin gets an entity id **and no graph**. The four real validators (`validate__event_triggers`, `validate__milestone_behavior_ranges`, `validate__type_field_annotations`, `validate__port_methods`) only work because `NativeCustomRules` executes them **natively in the host** with full `graph` access — including two-hop lookups (`event.triggers → node → kind == "behavior"`, compile.rs:521-537) and `graph.nodes()` kind scans for type resolution (compile.rs:579-586). This is the sharpest API finding under constraint R-1: the declared wasm contract (`validate__*` as guest exports) **cannot be honored by an actual wasm guest today** — there is no wire mechanism to resolve a reference or check a neighbor's kind. Any runtime migration must close this hole; keeping wasm requires inventing it.

### 2c. Declarative rules — host-executed patterns over flattened entities

`build_validation_entities` (compile.rs:390-459) flattens every node into `ValidationEntity {id, kind, fields: Map<String,String>, edge counts, span, verify_kinds}` and the host runs `execute_pattern` per manifest rule. No plugin code runs here; this is the *data shape* the plugin authors against when declaring rules.

## 3. Fidelity and determinism findings (these drive the API design)

**F1 — Stringification is lossy and loses the model's own semantics.** `build_validation_entities` collapses all 12 `FieldValue` variants into strings: `Integer(3)` → `"3"`, `VerifyList` → descriptions joined `"; "`, `Block` → clause names joined `", "`, `VariantList` → `"a | b"`, `MixedList` falls through the match arm and is **dropped entirely** (compile.rs:444 `_ => {}`). Annotations (`@readonly`, `@unique` — which W010 validates host-side) are invisible. `methods` (port method declarations) and `title` are not in `PassEntity` at all. A plugin cannot distinguish a reference list from a comma-containing string without the field registry.

**F2 — Two edge-label namespaces, neither exported.** Graph edges get `label = field name` ("produces", "consumes", "enforces") at creation (graph.rs:374-378); manifests declare CamelCase edge types ("BehaviorProducesEvent", "RefinementChainsToRefinement"). The host maps between them via `edge_label_to_field` (compile.rs:248-253, 658-661) but **does not pass the mapping or the manifest names to plugins**. Consequence observed in the only logic-bearing plugin: `layering_verify` bridges the gap with substring heuristics — `REFINEMENT_EDGE_MARKERS = ["RefinesTo", "RefinementChainLink", "refines"]` (formal lib.rs:144) — where "RefinesTo" and "RefinementChainLink" **cannot occur** in any real snapshot (no manifest field maps to them; real refinement labels are `abstract_entity`, `concrete_entity`, `chains_to`, and the behavior-enhancement field `refines`), and `chains_to` edges (the actual chain links the depth rule targets) match **none** of the markers. Its own test fixtures (lib.rs:347-348) assert against the impossible labels. The pass works today only because the loose `"refines"` substring catches the behavior-level field. This is audit C7-03 (no IDL, stringly drift) materializing inside the flagship plugin.

**F3 — Edge order is process-nondeterministic.** `resolve_references_with_singles` builds the edge Vec while iterating `self.nodes.values()` — a std `HashMap` with per-process random seed (graph.rs:352-363). `analyze.rs` does not sort pass findings, and `pass_event_graph_analyze` emits W029s while iterating its own `HashMap` (formal lib.rs:261-272). So `PassInput.edges` order and the diagnostic order in analyze reports vary across processes. R-6 (deterministic, snapshot-testable output) is currently protected only by luck or downstream sorting, and **this is a host bug, not a runtime-choice issue** — any API must pin node and edge iteration order.

**F4 — `nodes()` re-sorts per call** (graph.rs:140-144) and `compile_with_runtime` calls it four times per compile (steps 10/10a). A plugin-facing snapshot should materialize the sorted vec once.

**F5 — The validator hook has no graph context** (§2b). Minimum fix: hand the hook `{entity, fields, edges_in, edges_out}` per call, or expose node lookup as a host function. At ~1.7k entities (evidence-pack workload) a full-graph pull per entity would be ~1.7k boundary crossings — per-call context is the right granularity for a per-entity hook; the full push snapshot remains right for whole-graph passes.

## 4. The operation set plugins actually need (observed, exhaustive)

Derived from all real plugin code in the repo (4 formal passes, 4 native validators, declarative patterns):

| # | Operation | Evidence |
| --- | --- | --- |
| 1 | Iterate all nodes, deterministic order | every pass, `condition_check`, `coverage_tracking` |
| 2 | Filter nodes by kind | `condition_check` (`kind != "behavior"`), `coverage_tracking` (`invariant` / `testable`), `detect_cycles` host-side |
| 3 | Node lookup by id | `validate__event_triggers` two-hop, `layering_verify`/`event_graph_analyze` `by_id` maps (built manually — evidence the API should do it) |
| 4 | Read fields, typed, with annotations | `non_empty(entity, "requires")`, `tests` linkage check, W010 annotations (host-side today) |
| 5 | Iterate all edges; filter by label | `layering_verify`, `event_graph_analyze` |
| 6 | Adjacency by endpoint (in/out per node) | rebuilt manually in every pass; used as counts in `ValidationEntity` |
| 7 | Method declarations (ports) | `validate__port_methods` (native today) |
| 8 | Span for diagnostics | every `PassDiagnostic::with_span` |
| 9 | Kind metadata: `testable` / `supports_verify`, declared edge types + inverse (bidirectional) pairs | `PassEntity.testable`; bidirectional pairs needed so a plugin cycle pass doesn't false-positive on `enforces`/`enforced_by`-style complements (host `detect_cycles` filters them via `bidirectional_pairs`, graph.rs:431-466 — plugins get nothing) |

Never needed by any plugin: subgraph extraction, depth-limited traversal, file grouping, mutation, host-side cycle detection, `invalidation_set`.

## 5. Design: one logical API, four projections

### 5.1 GraphView — the runtime-neutral contract

```text
snapshot header:  { spec_root, node_count, edge_count, kinds: {kind → {supports_verify}},
                    edge_types: {manifest_name → {field_label, target_kind}},
                    inverse_pairs: [(label_a, label_b)] }          // fixes F2b: bidirectional pairs

graph:
  node(id) → Node?                    O(1)
  nodes() → [Node]                    id-sorted, materialized once (F3/F4)
  nodes_by_kind(k) → [Node]           id-sorted
  edges() → [Edge]                    canonical order: sort by (source, target, label)  (F3)
  edges_from(id) / edges_to(id) → [Edge]
  node_count() / edge_count()

Node:  { id, kind, title?, fields: [Field], annotations: [Annotation],
         methods: [Method], span, in_count, out_count, testable }
Field: { name, value: FieldValue, manifest_edge_type? }   // F2: both label namespaces
FieldValue: string | integer | boolean | date | ref_list[str] | list[str] |
            variants[str] | block[Field] | verify[{kind, description}] | expression | identifier
Edge:  { source, target, label /*field name*/, edge_type /*manifest name or null*/ }
```

Duality resolved host-side: every edge carries both its field-name label and the manifest edge type. Diagnostics flow back through the existing `PassDiagnostic` channel unchanged.

### 5.2 Projection per runtime

**WASM (Extism/Wasmtime — status quo).** Keep the **push** model; evolve `PassInput` → v2 in `specforge-protocol-types`: typed field union (tagged JSON, e.g. `{"t":"integer","v":3}`), `methods`, `annotations`, `edge_type` per edge, header with `inverse_pairs` + kind flags. No new host functions needed for passes. The validator hook gains a per-call context payload (`{entity, edges_in, edges_out}`) — closing §2b without any pull channel. Cost: wasm guests pay deserialize of the full snapshot per pass invocation (already true); tagged unions are the least ergonomic projection — this is wasm's structural API disadvantage. Ongoing wasmtime churn is real: workspace pins 43.0.2, latest stable is 49.0.1 (crates.io, fetched 2026-09-27).

**Lua (mlua).** Wrap an owned `Arc<GraphSnapshot>` (`Graph` is `Clone`) as mlua `UserData` with the GraphView methods; `node()`/iteration return **plain Lua tables** (copies owned by the GC) because Lua plugin code wants plain data, not proxy objects. Typed values map to native Lua types (integer/boolean/string/table); Lua 5.4 integers are i64 — no fidelity loss. Hook: `validate__*(ctx)` where `ctx = {entity, edges_in, edges_out}` tables. mlua 0.12.1, ~7.1M downloads (crates.io). Deterministic iteration trivial (host sorts). Hot reload: fresh `Lua` state per analyze run, matching the existing per-call engine pattern.

**Python (PyO3).** `#[pyclass] struct GraphView` over `Arc<GraphSnapshot>`; `#[pymethods]` for the GraphView ops returning `NodeView` pyclass objects (lazy, zero-copy until attribute access) or plain dicts for nodes. Typed values map to `str/int/bool/list/dict` natively. GIL is a non-issue at analyze scale (single-threaded, host-owned snapshots). Hook receives `ctx` dict. pyo3 0.29.2, ~261M downloads — the most battle-tested embedding here, but the embedding *distribution* cost (R-3), not the API, is its problem.

**JS/TS (deno_core / QuickJS).** The odd one out in a good way: the existing `PassInput` JSON shape **is** a JS object graph — the push projection is `Object.freeze`d plain objects handed to the script, i.e. the v2 wire format and the JS plugin surface are the *same document*. Lazy lookups (`graph.node(id)`) map to deno_core ops if needed later; not needed for MVP at this graph size. i64 fields would exceed `Number.MAX_SAFE_INTEGER` in principle — spec integers are small; document a string-overflow rule. deno_core 0.412.0; QuickJS is the small-footprint variant (no permissions model — R-2 burden moves to the host).

**Cross-runtime invariants (R-6):** node iteration = id-sorted; edge iteration = (source, target, label)-sorted; no ambient time/fs/net in the API; identical GraphView semantics verified by one host-side conformance suite executed against every runtime projection (this replaces today's three-way mirror sync tests — audit C7-11's fix shape).

### 5.3 Push vs pull

Pull-model (host functions per query) was rejected for passes: scanning 1.7k entities with per-node host calls is thousands of boundary crossings per pass versus one payload push [INFERENCE — no benchmark run in this session]; push also makes output byte-snapshot-able by construction (R-6) and lets the host enforce `query_scope` by filtering what it pushes (fixes C7-09's enforcement point). Pull remains unnecessary for the validator hook because per-call context covers the only observed need (neighbor kind checks). If future plugins need random-access over multi-MB graphs, add `graph.node/edges_from` as host imports then — the v2 header design doesn't preclude it.

## 6. Minimum viable API surface

**Nine read operations + two records + one hook context — everything in §4, nothing else:**

1. `nodes()`, `nodes_by_kind(k)`, `node(id)` — iteration/filter/lookup
2. `edges()`, `edges_from(id)`, `edges_to(id)` — traversal (adjacency indices precomputed host-side)
3. `node_count()`, `edge_count()` — summaries (analyze report already prints `entities_analyzed`)
4. snapshot header: kinds (`testable`), `edge_types` map, `inverse_pairs`
5. `Node` record with **typed** fields, annotations, methods, title, span, in/out counts
6. `Edge` record with both label namespaces
7. validator hook context `{entity, edges_in, edges_out}`

**Explicitly cut from MVP** (with the trigger to add them): `subgraph`/`subgraph_depth` (host emitter/MCP concern; add if a plugin ever asks for scoped analysis), `filter_nodes` with arbitrary predicates (impossible cross-runtime; kind + field access composes it), `invalidation_set` (watch-only), all mutation (host-only, forever under R-2), host `detect_cycles` exposure (plugins do their own DFS fine — with typed edges and `inverse_pairs` exported, F2's false-marker class disappears).

**Sequencing note:** F2 (label mapping) and F3 (canonical edge order) are host bugs in `compile.rs`/`graph.rs`/`analyze.rs` that must be fixed **regardless of runtime choice**; they are prerequisites for any GraphView, and fixing them first (plus typed fields in `PassInput` v2) delivers most of the API value even before any runtime migration.

## Bottom line

The graph API plugins need is small, read-only, and push-shaped: nine query operations over an id-sorted, canonically-ordered snapshot with typed fields and a resolved edge-label namespace. The current wasm ABI fails its own contract twice — stringified lossy fields (F1) and a validator hook with zero graph access that only works via the R-1-violating native mirror (§2b) — and the flagship formal plugin bridges the missing label namespace with substring heuristics that demonstrably miss the actual chain edges (F2). Every candidate runtime can project the same GraphView; scripting runtimes do it with native types and no tagged-union ceremony, while wasm needs an ABI extension (PassInput v2) that is worth building regardless. Two of the costliest findings (F2, F3) are host-side determinism/label bugs independent of the runtime decision and should land first.

## Verdict

**Verdict:** LUA
**Confidence:** 3
**One-line rationale:** The observed plugin workload is nine read-only graph operations over ~1.7k typed entities, which mlua carries with plain native-typed tables and zero ABI ceremony — while the wasm path needs a new tagged-union wire format and per-hook host-function invention just to honor its own declared contract — though this API-slice evidence alone favors scripting runtimes by ergonomics, not decisively on sandboxing or distribution.
