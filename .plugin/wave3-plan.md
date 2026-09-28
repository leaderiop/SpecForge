# Wave 3 Plan — Product-Story Wave (make the flagship claims true and demonstrated)

Ground truth: verified 2026-09-28 by four parallel read-only scouts (economics/
traceability, formal semantics, freshness/renderers, registry/LSP). Findings the
audit got wrong are marked; everything else carries fresh file:line evidence.

## 0. Verification summary

### Closed before Wave 3 starts (no action)

| fid | finding | disposition |
|---|---|---|
| C13-08 | docs promise extension-contributed formats | CLOSED by docs sweep — `ModelFormat` is a closed 5-variant enum, no wasm renderer surface, and extension-model.md marks generators as design-direction (`:191-193`); no stale claim remains |
| C6-14 | LSP skips CycleDetection | INTENTIONAL — `backend.rs:488-493` defers to build_graph's W061, which flows through the pipeline; LSP and CLI agree byte for byte. Wave 3 adds only a rationale comment |
| C10-11 | cycle detector soundness untested | PARTIAL — real tests exist (cycle→E041, acyclic→empty, depth→W031, W029 consumer). Remaining slivers: self-cycle, parallel-edge dedup, cyclic-excluded-from-W031 — folded into W3-C |

### Wave 3 execution set (7 workstreams, 12 items)

| id | item | verdict | effort |
|---|---|---|---|
| A1 | C1-06 traceability loop demonstrated in examples/todo-app | VERIFIED (spec-only: 17 .spec files, zero tests/reports/CI) | M |
| B1 | C1-10 token budget for context/brief (the agent formats) | PARTIAL — budget exists but applies ONLY to schemaless JSON (`emit.rs:113-118`) | M |
| B2 | C9-04 compact serialization for machine formats | VERIFIED — every graph emitter is `to_string_pretty` (context.rs:61, brief.rs:41, budget.rs:133,153) | S |
| B3 | C1-07 mini-eval: measured numbers into RES-18 | VERIFIED (claims are projections; caveat added in docs sweep) | S |
| C1 | C10-00 process semantics in the formal guest | VERIFIED — no pass reads Process kinds or `EventParticipatesInProcess`/`ProcessComposesProcess` edges; docs promise E042 + `process_analyze` which don't exist | M |
| C2 | C10-11 soundness-test slivers | PARTIAL | S |
| D1 | C9-07 watch→MCP freshness bridge | VERIFIED — watch writes stdout only; MCP compiles once at `initialize` (`lifecycle.rs:46-52`) and caches forever; `pending_notifications` compares against `Graph::new()` (dead) | M |
| E1 | C14-04 residual: 12 inline blocking sites in registry-server | VERIFIED (list below) | M |
| E2 | C8-04 add/update resolve against `registries.first()` | VERIFIED (`add.rs:98-100`, same bug `update.rs:63`, `mcp operations/mod.rs:402`) | S |
| E3 | C8-05 lockfile peer check compares an entry against itself | VERIFIED (`lock_file.rs:134-147`) | M |
| F1 | C4-07 syntax-only fast path | VERIFIED — every edit runs validator + W022 + E024/W020 + Wasm rules even when E001 parse errors exist | M |
| F2 | C4-03 hardening + C6-14 comment | PARTIAL (supersede exists; in-flight reparse can't be superseded) | S |
| G1 | C12-100 276 template `verify contract` strings | VERIFIED (exact count) | L |
| G2 | C12-102 failure modes unlinked from features/behaviors | PARTIAL — edges exist in the governance schema (`threatens_features`, `affected_behaviors`); corpus uses none | S/M |
| G3 | C12-107 no-event declarations live in comments | VERIFIED (~14 comment sites; `produces []`/`consumes []` fields already exist) | S/M |

---

## Workstream A — demonstrate the traceability loop (C1-06)

Current state: `examples/todo-app/` has specforge.json + README + 17 .spec files.
No test suite, no specforge-test integration, no specforge-report.json, no CI.
The loop stops at `specforge trace` — nothing feeds results back.

### A1 — close the loop in the flagship example

Design (minimal, everything already ships):
1. Add a tiny Rust test crate inside the example (`examples/todo-app/tests/`)
   using the `specforge-test` integration: a handful of tests whose names
   mirror verify statements, plus `tests [...]` linkage fields on the
   entities they prove.
2. `specforge-test`'s atexit handler writes `target/specforge/<bin>.json`.
3. Document + script the loop in the example README:
   `cargo test -p todo-app-tests` → `specforge collect --path .` →
   `specforge analyze --path . coverage --test-results specforge-report.json`
   → `specforge trace <entity>` showing proof provenance.
4. A workspace-level integration test (`crates/specforge-cli/tests/` or an
   xtask check) runs the loop end-to-end against the example and asserts
   non-zero provenance — so the demo cannot silently rot (this is the
   "enforced by nothing" lesson from C1-09 applied to the flagship).
5. vision/principles.md Principle 5 gains a pointer: "demonstrated in
   examples/todo-app — see its README".

Acceptance: the loop runs green from a fresh clone; the integration test
fails if the example's report goes stale (asserts entities have recorded
proof results).

---

## Workstream B — token economics made real (C1-10, C9-04, C1-07)

### B1 — budget applies to context/brief (C1-10, M)

Problem: `emit.rs:113-118` gates budgeting to `format == Json && schema.is_none()`.
The agent formats — context (contract/status/verify) and brief (id/kind/title) —
ignore `token_budget` entirely, even though the MCP export tool advertises it.
`filter_graph_within_budget` (budget.rs:165-213) already does degree-centrality
truncation and is reused by the MCP context/brief resources (`?max_tokens=`).

Fix: in `emit`, when `token_budget` is set and format is Context or Brief,
run the subgraph through `filter_graph_within_budget` before serialization
(same helper the resources use). Keep schemaless-JSON path unchanged.
Update the MCP registry description: drop "ignored for other formats".

Acceptance: `crates/specforge-emitter/tests/` — context export with a small
budget returns a strict subset (assert node count / centrality ordering);
brief likewise; budget absent → unchanged output (golden).

### B2 — compact serialization for machine formats (C9-04, S)

Problem: json/context/brief/schema/budget all `to_string_pretty` — whitespace
burns agent tokens on every call (the exact cost RES-18 sells against).

Fix: machine formats serialize compact (`serde_json::to_string`); human
formats (`model` markdown/dot/mermaid, `schema --publish`) stay pretty.
`schema` emitted inside V2 exports: compact (it's for machines). Budget loop's
per-iteration pretty serialization (budget.rs:133,153) becomes compact —
also removes wasted work. Keep a `--pretty` escape hatch on CLI export if
cheap; otherwise document `jq` as the pretty-printer.

Acceptance: emitter tests — context/brief/json outputs contain no
pretty-print newlines between tokens; byte-size assertion (compact < pretty)
as a canary; human formats unchanged.

### B3 — measured numbers into RES-18 (C1-07 partial, S)

Fix: add a "Measured baseline (2026-09)" section to
`spec/research/RES-18-ai-agent-token-economics.md`: bytes + estimated tokens
for the self-host corpus (spec/) and examples/todo-app — full JSON vs context
vs brief, pretty vs compact, with and without budget, produced by a small
xtask bench (or a documented shell one-liner so it's reproducible). The
75-86% claims stay marked as projections; the measured table anchors them.

Acceptance: the section exists with real numbers; the producing command is
reproducible from the repo.

Commit series B: `feat(emitter): budget applies to context/brief; compact machine serialization` + `docs(research): measured token baseline`.

---

## Workstream C — process semantics in the formal guest (C10-00, C10-11)

Verified: Process kind + `EventParticipatesInProcess` (event→process) +
`ProcessComposesProcess` (process→process) are declared in describe payloads;
all four passes ignore them (`event_graph_analyze` matches only
`produces`/`consumes`; other labels hit `_ => {}`). docs/entity-model.md
previously promised E042 + `process_analyze`; glossary references the pass.

### C1 — interpret processes (M)

1. `pass_event_graph_analyze` gains process awareness:
   - `EventParticipatesInProcess` edges count as event usage (an event
     participating in a process is consumed by it — extends the W029
     unconsumed-producer logic rather than duplicating it);
   - `ProcessComposesProcess` cycles → **E042** ("process composition cycle")
     mirroring the refinement DFS (E041) — same exact-membership walker shape
     already used by `pass_layering_verify`.
2. Naming: fold into `event_graph_analyze` (do NOT create the phantom
   `process_analyze` pass). Update `spec/glossary.spec:590-594` to say the
   event-graph pass covers process composition; re-add the **E042** row to
   docs/entity-model.md formal table (it was removed as phantom in the docs
   sweep — now it becomes real) + an `explain.rs` entry.
3. Corpus: add a `process` entity + composition edges to the formal spec
   corpus exercising the new diagnostic (or a fixture in the guest tests).

### C2 — soundness slivers (S)

Extend `pass_tests`: self-cycle (a→a → E041), parallel duplicate `RefinesTo`
edges (single diagnostic, no dup), cyclic chain excluded from W031 depth
counting.

Acceptance: guest tests for E042 (cycle found, acyclic clean, participation
suppresses W029); corpus check stays 0 errors; explain entry present.

Commit: `feat(formal): process composition cycles and event participation (C10-00)`.

---

## Workstream D — watch→MCP freshness bridge (C9-07, M)

Verified freshness map: watch computes `result.delta` and prints stdout only;
MCP compiles once at initialize into `McpState.graph` and never recompiles;
`pending_notifications` diffs against `Graph::new()` (dead machinery).

### D1 — snapshot + freshness check (chosen design: cheapest)

1. Writer: `crates/specforge-cli/src/watch.rs` after each successful rebuild
   atomically writes the graph to `<project_root>/.specforge/graph.json`
   (serde of `pipeline.graph()`, tmp+rename — pattern from registry storage).
   Flag-gated (`--emit-graph`, default ON for watch).
2. Reader: `McpState` gains `snapshot_path: Option<PathBuf>` +
   `snapshot_mtime: Option<SystemTime>`. A `fresh_graph(&mut McpState)`
   helper stats the snapshot at each tool/resource entry: if newer than
   `loaded_at`, deserialize and swap `state.graph`, update `loaded_at`.
   Falls back to the initialize-time graph when absent/stale-broken.
   Single choke point: the tool dispatch entry calls `fresh_graph` before
   routing (and resources likewise).
3. Fix the dead `pending_notifications` baseline (compare against last-published
   graph, not `Graph::new()`) and emit `specforge/graphChanged` on actual
   deltas for subscribed sessions — the machinery exists, it just never had a
   producer.

Acceptance: e2e test — start MCP against a tempdir project, run watch (or
write the snapshot directly), assert a query tool returns the new entity;
`graphChanged` notification fires for a subscriber; no-snapshot projects
behave exactly as today.

Commit: `feat(mcp): fresh graph snapshots from watch (C9-07)`.

---

## Workstream E — registry server + client (C14-04 residual, C8-04, C8-05)

### E1 — remaining 12 inline blocking sites (M, mechanical)

Sites (verified): handlers.rs :117 get_package_versions, :146 get_package_version,
:219-229 download integrity re-check (DB + 64 MB inline SHA-256), :341/:770/:864/:913
validate_bearer (SHA-256 + rusqlite), :444/:463 scope claim/owner, :823 yank write,
:960/:990/:1017 admin token ops. Fix: mirror the Wave-2 pattern —
`spawn_blocking(move || ...)` with an Arc'd state, `.expect(...panicked)`
convention preserved; move the download SHA-256 into the storage blocking task.
Test surface: existing registry-server route tests.

### E2 — scope-routed resolution for add/update (S)

`registries.first().unwrap()` → `find_registry_for_specifier(name, &registries)`
in `add.rs:99`, `update.rs:63`, `mcp operations/mod.rs:402`, with the R-OPS-001
diagnostic when no registry's scope matches. Acceptance: integration test with
two registries where the package exists only on the scoped one (fails today).

### E3 — real lockfile peer check (M)

`lock_file.rs:134-147` compares an entry against its own recorded version.
Fix: plumb ManifestV2 peer_dependencies into `run_doctor_check` (via the
installed manifests the doctor already loads), verify each peer's presence and
semver-req satisfaction across lock entries, report
`PeerDependencyMismatch { name, peer: <actual peer>, required }`.
Acceptance: new test — A declares peer B@^2, lock has B@1 → mismatch naming B;
all-satisfied → clean. Risk: doctor gets stricter (that's the point).

Commit series: `fix(registry-server): move the last 12 blocking calls off the async runtime` + `fix(registry): scope-routed version resolution in add/update` + `fix(doctor): real peer-dependency verification`.

---

## Workstream F — LSP quality (C4-07, C4-03, C6-14)

### F1 — syntax-only fast path (M)

In `parse_and_update`, after the blocking `update_open_file` returns: if
`result.diagnostics` contains E001 parse errors for the edited file, publish
only those immediately and SKIP validator/W022/E024/W020/extension-rule passes
(they evaluate a broken graph; Wasm rule dispatch on every keystroke of a
broken file is pure waste). Full passes resume once the file parses cleanly.
The reparse itself still runs (it must — tree-sitter incremental is the cheap
part of correctness here).

Acceptance: integration test — did_change introducing a syntax error
publishes E001-only diagnostics (no W022/E024); fixing the syntax restores
the full set.

### F2 — debounce hardening + rationale comment (S)

Residual: a keystroke during an in-flight reparse queues another full
reparse. Hardening (S): replace spawn+sleep with a per-URI long-lived task
waiting on a generation counter (Notify), so a newer keystroke always
supersedes without spawn churn. C6-14: add the rationale comment at the
CycleDetection skip (`backend.rs:489`): intentional, W061 flows via build_graph.

Acceptance: concurrency tests stay green; rapid-fire did_change (10 edits in
150 ms) results in exactly one full reparse (assert via a counter hook or
timing).

Commit: `perf(lsp): syntax-only fast path + coalescing debounce`.

---

## Workstream G — corpus truths (C12-100, C12-102, C12-107)

### G1 — real contract verifies (L, mechanical but large: 276 sites)

All 27 behavior files close with `verify contract "requires/ensures
consistency for X"`. Rewrite pattern (DSL supports `requires {} / ensures {}`
and `contract """…"""`; W096 enforces requires⇒ensures): each behavior's
contract verify states its observable requires/ensures pair — three worked
examples verified: graph.spec:36 (nodes ⊆ parsed entities; every edge
resolves), lsp.spec:54 (initialize advertises exactly the trigger characters
of loaded extensions), mcp-server.spec:507 (unknown method → -32601 echo, no
partial mutation). Execute file-by-file (27 files), running the corpus check
(`specforge check spec` — 0 errors) after each batch; the `contracts.rs`
crates parse these strings, so the full test suite is the acceptance gate.
Batch order: graph, lsp, mcp-server, wasm-*, parsing, output, then the rest.

### G2 — link failure modes to features/behaviors (S/M)

`spec/governance/failure-modes.spec` (15 failure_mode blocks) uses only
`invariant <id>`. The governance schema already declares
`threatens_features [...]` (FailureModeThreatensFeature) and
`affected_behaviors [...]` (FailureModeAffectsBehavior). Populate them with
real targets (verified mappings exist: incremental_divergence →
affected_behaviors [rebuild_affected_subgraph, compute_graph_delta];
peer-dependency modes → extension init behaviors). Acceptance: corpus check
clean; `specforge trace` on a feature surfaces the threatening failure mode.

### G3 — declare no-event data instead of prose (S/M)

~14 comment sites (`graph.spec:41`, `lsp.spec:361/510`, `extensions.spec:286/
317/351/384`, `init.spec:67`, `output.spec:139`, `resolution.spec:45/147`,
`validation.spec:18-19/53-54/114-115/181-182`, `mcp-tools.spec:63`). Fix:
declare `produces []` / `consumes [...]` with the true data flow (e.g.
`emit_live_diagnostics` declares `consumes [incremental_rebuild_complete]`);
for genuinely terminal behaviors, `produces []` becomes the convention —
documented in entity-model.md (behavior fields) so silence is no longer
ambiguous. Acceptance: corpus check clean; a grep assertion in CI-style test
that behaviors with "No produces" comments carry an explicit `produces []`
(implemented as a corpus lint test in specforge-cli/tests or xtask).

Commit series: per-batch commits for G1 (`docs(corpus): real contract verifies — <file>`), one commit for G2+G3.

---

## Execution order & commit series

1. **B2** compact serialization (S, immediate token win, unblocks B3 numbers)
2. **B1** budget for context/brief
3. **B3** measured RES-18 baseline (depends on B1+B2 for final numbers)
4. **C1+C2** formal process semantics + soundness slivers
5. **E1** registry-server blocking sweep (mechanical)
6. **E2, E3** scope routing + peer check
7. **A1** todo-app loop demo
8. **D1** watch→MCP freshness
9. **F1+F2** LSP fast path + debounce
10. **G2+G3** corpus data truth
11. **G1** the 276-site contract rewrite (largest, mechanical, last)

Every series: fmt → clippy `-D warnings` → workspace tests → corpus check
(`specforge check spec` = 0 errors) → push → CI watch. G1 additionally keeps
the `contracts.rs` suites green per batch (they parse these strings).

## Explicit non-goals

- `process_analyze` as a separate pass (folded into event_graph_analyze; the
  glossary's phantom pass name is corrected instead)
- namespace-qualified reference visibility in the linker (C3-06 follow-up;
  qualified refs need their own parse story)
- selective-import narrowing of the visible set (v1 is deliberately
  permissive — aliases are visible, whole target exports visible; tightening
  waits for W099 warning data)
- generator/gen implementation (C13-08 stays closed-by-docs; it's a design
  direction needing its own arc)
- registry version diamond-unification/backtracking (C8-07; needs a resolver
  design doc first)
