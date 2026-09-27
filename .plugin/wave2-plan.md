# Wave 2 Plan — Audit Problem Resolution (engineering)

Ground truth: verified 2026-09-27 against current `main` (post `8bc568d`) by four
parallel read-only scouts (LSP, registry/protocol, CLI/coverage, graph/sandbox).
Verdicts differ from the audit in several places — the plan below executes only
what is still real.

## 0. Verification summary — what Wave 2 actually contains

### Removed from Wave 2 (verified stale — no action)

| fid | audit claim | verified reality |
|---|---|---|
| C5-01 | LSP never surfaces cycle diagnostics | STALE — `specforge-watch/src/pipeline.rs:120-140` builds W003 cycle diagnostics from the import DAG, merges them into `file_diagnostics` (`:174-185`, `:368-376`), and the LSP republishes them (`backend.rs:246-296`). Test: `crates/specforge-watch/tests/pipeline.rs` (~:783). |
| C4-00 | two parallel invalidation implementations | STALE — LSP owns a `specforge_watch::IncrementalPipeline` (`state.rs:44`); CLI watch drives the identical pipeline (`watch.rs:296,167`). Residual divergence is deliberate: LSP debounces 150 ms and layers extra inline validator passes (`backend.rs:301-455`). |
| C8-01 | publish non-atomic/racy | STALE — `handlers.rs:601-726` is a 3-phase atomic flow: temp file + fsync → `UNIQUE(name,version)`-arbitrated insert (loser's temp discarded) → atomic `fs::rename` with compensating `delete_package`. Tests: `storage.rs:120-163`, `server_contracts.rs`, `publish_signatures.rs`. |

### Wave 2 execution set (14 items, 5 workstreams)

| id | item | verdict | effort |
|---|---|---|---|
| A1 | C4-02a completion.rs UTF-16/byte mix | VERIFIED | S |
| A2 | C4-02b semantic tokens byte columns | VERIFIED | M |
| A3 | C4-02c rename byte columns | VERIFIED | S |
| A4 | C4-02d symbol + formatter-edit passthroughs | VERIFIED | S |
| A5 | C14-03 LSP blocking work on async runtime | PARTIAL (residual) | M |
| B1 | C9-13 MCP tool-schema drift | VERIFIED | M |
| C1 | C6-02 version negotiation degenerate | VERIFIED | S |
| C2 | C6-06 one schema, three node shapes | VERIFIED | M |
| C3 | C6-13 config schema vs ProjectConfig | VERIFIED | S |
| D1 | C11-00 collect stub | VERIFIED | M |
| D2 | C11-01 no coverage gate | VERIFIED | S |
| D3 | C11-02 no unmatched-test signal | VERIFIED | S/M |
| E1 | C13-00 DOT styles declared, never consumed | VERIFIED | S/M |
| E2 | C7-04 sandbox allow-by-default + unwired | VERIFIED + worse | M |
| E3 | C5-00 cycle-detector consolidation | PARTIAL (Phase D fixed the over-reporter; consolidation remains) | M |
| E4 | C3-06 scope-blind reference resolution | VERIFIED | M/L |

---

## Workstream A — LSP correctness (C4-02 residuals, C14-03)

Context: the core UTF-16↔byte conversion layer already exists and is correct —
`document.rs:60-85` (`utf16_col_to_byte_offset`, `byte_col_to_utf16_col`, both
clamp past-end and mid-surrogate), `lib.rs:52-68`
(`source_span_to_lsp_range_with_text`), `backend.rs:654-680` (`word_at_position`
— used by hover/completion/goto/references/rename-prepare). The remaining work
is the four sites that bypass it, plus moving blocking work off the async runtime.

### A1 — completion.rs cursor scan window (C4-02a, S)

- Problem: `completion.rs:42` `let scan_end = col.min(current_line.len())` — `col`
  is UTF-16 units (arrives raw from `pos.character` via `backend.rs:1169`), but
  `current_line.len()` is bytes. On any line with multi-byte characters before the
  cursor the scan window is wrong and `[..scan_end]` can panic
  (`byte index is not a char boundary`).
- Fix: convert first — `let byte_col = doc.utf16_col_to_byte_offset(line_idx, col)`
  (DocumentBuffer already carries the line table); slice with the byte col. The
  function needs the `DocumentBuffer` (or line text) passed in, not just the raw line.
- Acceptance: new test in `crates/specforge-lsp/tests/completion.rs`: file whose
  current line contains a multi-byte char (e.g. `ünits`) before the cursor;
  completion at end-of-line returns kind completions without panic and with the
  same results as the ASCII case. Also a panic-regression test with the cursor
  inside/past the multi-byte char.

### A2 — semantic tokens byte columns (C4-02b, M)

- Problem: `semantic_tokens.rs` `classify_tokens` computes columns via
  `line.find(...)`/`find_word_col` — byte offsets — and `backend.rs:1530-1538`
  emits them directly as `delta_start`, with `tok.text.len()` (bytes) as `length`.
  Non-ASCII prefixes shift every token on the line and lengths are wrong.
- Fix: after classification, walk each line's text and convert each token's
  byte start + byte length to UTF-16 start + UTF-16 length (char-count for
  ASCII, `chars().map(len_utf16).sum()` generally). Keep the delta encoding:
  convert absolute → UTF-16 absolute first, then take deltas; assert monotonic
  non-decreasing `start` in a debug assertion.
- Acceptance: `tests/semantic_tokens.rs` — file with multi-byte content; token
  on a line after a multi-byte char has correct LSP start; length of a token
  containing an accent equals its UTF-16 length. Compare against a
  hand-computed fixture.

### A3 — rename byte columns (C4-02c, S)

- Problem: `backend.rs:1360-1366` emits `edit.start_col.saturating_sub(1) as u32`
  — raw graph-span byte cols, no file text consulted.
- Fix: build the range via `source_span_to_lsp_range_with_text` against the
  target file's text (open buffer or `file_content`), the same helper the
  navigation handlers already use.
- Acceptance: `tests/rename.rs` — rename an entity referenced from a file whose
  earlier lines contain multi-byte characters; all WorkspaceEdits land on the
  identifier (assert exact ranges).

### A4 — symbol + formatter-edit passthroughs (C4-02d, S)

- Problem: workspace `symbol` handler (`backend.rs:1486-1504`) uses
  `source_span_to_location` (byte passthrough); `publish_format_diags`
  (`:704-724`) + `formatter_edits_to_lsp` pass formatter byte offsets through.
- Fix: `symbol` → text-aware conversion like document_symbol
  (`:1456-1477` already does it right — reuse its helper). Formatter edits:
  convert each edit's byte line/col via the file text.
- Acceptance: `tests/symbols.rs` multibyte case; a format-diagnostic test with
  a multi-byte file asserting the diagnostic lands on the right character.

### A5 — LSP blocking work off the async runtime (C14-03 residual, M)

- Problem (verified residual): `index_workspace` walkdir is fixed (spawn_blocking
  at `backend.rs:66`), but:
  1. `parse_and_update` (`:246-546`) runs `pipeline.update_open_file` (sync fs
     reads per invalidated importer at `:260-262`), a full graph rebuild, the
     validator, and Wasm rule dispatch **while holding `state.write()`**
     (`:257-263`, `:301-455`) — on a normal tokio worker.
  2. `did_change_watched_files` (`:994-1103`) does sync `fs::read_to_string`
     (`:1095`, `:1034-1036`) plus a full reindex inline.
  3. `load_registries` (`:161-243`) does sync specforge.json read + Wasm
     extension loading inline.
  4. `initialize` (`:745`) sync specforge.json read.
  5. `file_content` (`:547`) sync disk reads inside diagnostic loops under a
     read guard.
- Fix design (lock-safety first):
  - Restructure `parse_and_update` into three phases: (1) under `state.read()`
    snapshot the inputs (open-file contents map, changed path); (2) drop the
    lock, run `update_open_file` + rebuild + validator + rule dispatch inside
    `tokio::task::spawn_blocking` (pre-collect importer file contents inside
    the blocking section — that is where the sync reads belong); (3) take
    `state.write()` only to store the finished `IncrementalResult` and compute
    republish lists. This removes both the blocking-on-worker and the
    write-lock-across-blocking problems.
    - Send-ness risk: `IncrementalPipeline` must be owned inside the blocking
      task and returned; if it is not `Send`, box the non-Send parts or move
      the whole `LspState.pipeline` into the task (it is `Arc<RwLock<_>>`
      today — take the pipeline out, compute, put it back).
  - `did_change_watched_files`: wrap the read + reindex in `spawn_blocking`
    (it already batches; make the handler await a blocking join).
  - `load_registries` + `initialize` config read: `spawn_blocking`.
- Acceptance: existing `tests/concurrency.rs` + `tests/e2e.rs` stay green; add
  `tests/blocking.rs`: a project whose importer fan-out is large (50 files) with
  an artificial fs-sleep (tempdir on a slow path is not portable — instead
  assert via `tokio::test` + `pause()` time that no handler hogs a worker for
  the whole rebuild), plus a functional test that did_change_watched_files on a
  new file produces diagnostics. If the timing test proves flaky, fall back to
  a code-structure assertion (handler bodies delegate to a `spawn_blocking`
  wrapper — verified by a unit test on the wrapper).

Workstream A ships as one commit: `fix(lsp): remaining UTF-16 conversion sites + blocking work off the async runtime`.

---

## Workstream B — MCP honesty (C9-13)

### B1 — tool-schema drift (M)

Verified drift table (registry.rs `default_tools()`:130-589 vs handlers):

| Tool | unadvertised-but-read | advertised-but-unread |
|---|---|---|
| specforge.query | `format` (query.rs:20), `include_coverage` (query.rs:24) | — |
| specforge.validate | `severity_filter`, `use_cached` (validate.rs:27-30) | — |
| specforge.search | `field`, `value`, `references` (search.rs:26-28) | — |
| specforge.trace | — | `plan` (never read) |
| specforge.format | `write` (operations/mod.rs:133-135) | `paths` (verify — see below) |
| all operation tools | `path` via shared `project_root_of` (mod.rs:52-56) | — |

- Fix: update `registry.rs` advertised inputSchemas to match reality — advertise
  the eight hidden params, delete `trace.plan`, and advertise `path` on every
  operation tool that goes through `project_root_of` (format, rename, migrate,
  collect, add/remove already partial). Before editing, verify `format.paths`:
  if truly unread, remove it from the schema and make the handler log a
  no-op-params note; if read (check `format_op`), advertise it.
- Guard against future drift: add `crates/specforge-mcp/tests/schema_reflection.rs`
  — a conformance test that, for each advertised tool: (1) calls the handler
  with every advertised optional property set to a harmless value and asserts
  success; (2) asserts that no handler reads args outside the advertised set
  (implement via a wrapper that records accessed keys — the handlers take
  `serde_json::Value`; add a small `ArgSpy` in test-utils that tracks
  `.get()`/pointer reads).
- Acceptance: reflection test green; MCP suite (`tools_core.rs`,
  `tools_navigation.rs`, `operations_mutation.rs`) green; the Phase-M
  no-fake-success conformance gate still passes.
- Commit: `fix(mcp): advertise the parameters handlers actually read; drop trace.plan`.

---

## Workstream C — schema/config truth (C6-02, C6-06, C6-13)

### C1 — version negotiation (C6-02, S)

- Problem: `crates/specforge-cli/src/export.rs:62-70` negotiates with
  `min = max = schema.schema_version` (degenerate: only the exact current
  version passes, then the label is overwritten with the request anyway);
  `negotiate_version_or_latest` (schema.rs:852-869) is dead; three version
  strings coexist for the same graph (V1 `0.1.0` in `json.rs:6`, computed V2 in
  `schema.rs`, `0.1.0` hardcoded in `mcp/resources|tools/schema.rs:42`).
- Fix: (1) delete `negotiate_version_or_latest` + its re-export; (2) give the
  CLI a real range: `min = lowest supported same-major` (constant),
  `max = current` — negotiation then means something; on out-of-range request,
  E027 with the supported range in the suggestion; (3) replace the MCP
  hardcoded `"0.1.0"` with the emitter's V1 constant (single source).
- Acceptance: `emitter/tests/schema.rs` negotiation tests updated; new
  `crates/specforge-cli/tests/export_version.rs`: `--schema-version 99.0`
  fails with E027 + suggestion; current version succeeds; a same-major older
  version succeeds and labels the output.

### C2 — per-format published JSON schemas (C6-06, M)

- Problem: `publish_json_schema` (`schema.rs:899-961`) requires
  `["id","kind","file","line","fields"]` — the full-node shape. Context nodes
  carry `contract/status/verify` and omit file/line/fields (`schema.rs:437-446`);
  brief nodes carry only `id/kind/title` (`schema.rs:498-503`). One schema
  validates a shape two of three formats never produce. V1 and V2 envelopes
  also differ (`{schema_version,nodes,edges}` vs `{format_version:"2.0",...}`),
  and the MCP export tool always emits V1 while the CLI emits V2 by default.
- Fix: `publish_json_schema(format: EmitFormat)` emits the schema for that
  format — full (current), context, brief — sharing `$defs` for edge items;
  `run_schema` in the CLI takes the format (default `graph`). Keep the V2
  envelope's `format_version` in `required`. Decide V1: it stays a frozen
  legacy shape — document it in the schema description rather than
  schema-ifying it.
- Acceptance: `emitter/tests/schema.rs` — for each format, the published
  schema validates a real emitted document (wire `jsonschema` crate as a
  dev-dependency if acceptable; otherwise assert required-key sets per format).
  `crates/specforge-cli/tests/export.rs`: `specforge schema --format context`
  emits context-required keys, not full-node keys.

### C3 — config schema vs ProjectConfig (C6-13, S)

- Verified divergence (both directions):
  - In schema (`schema/specforge.schema.json`, `additionalProperties: false`),
    not in `ProjectConfig` typed fields: `$schema`, `strict`, `namespace`,
    `display_prefix`, `enhancement_policy`, `enhancement_overrides`,
    `entity_kind_policy`, `grammar_policy`, `entity_kinds`, `providers`,
    `personas`, `surfaces`, `test_dirs`, `coverage` (several are consumed via
    ad-hoc `raw` reads — providers at `registry/compilation/provider.rs:40`,
    mcp `operations/mod.rs:642`).
  - In `ProjectConfig`, not in schema (formally invalid under
    `additionalProperties:false`): the whole `inference` object
    (`project.rs:29-33`, `92-113`) and `registries` (`registry_config.rs:50`).
- Fix: add `inference` (global: bool, kinds: map<string,object>,
  density_threshold: number) and `registries` (array of registry configs) to
  BOTH schema copies (`schema/specforge.schema.json`,
  `integrations/vscode/schemas/specforge.schema.json` — keep them in sync; add
  a drift-guard unit test asserting the two files are identical). The
  schema-only keys stay (they describe the full spec-block surface consumed
  elsewhere); add a `$comment` in the schema noting which keys ProjectConfig
  consumes via raw passthrough.
- Acceptance: `crates/specforge-cli/tests/init.rs` gains a case validating a
  maximal specforge.json (inference + registries + providers) against the
  shipped schema (dev-dependency `jsonschema`); the two schema files are
  byte-identical (test).
- Commit (C1+C2+C3): `fix(schema): real version negotiation, per-format export schemas, config schema parity`.

---

## Workstream D — CLI data plane (C11-00, C11-01, C11-02)

The building blocks exist and are tested: `ingest_collector_report`
(`specforge-wasm/src/contributions.rs:546-593`, unit tests `:1611-1710`,
zero production callers), the analyze coverage pass
(`emitter/src/analyze.rs::pass_coverage` consuming `TestReport` via
`analyze --test-results`), and the rust integration's atexit reporter
(`integrations/rust/specforge-test`) writing `target/specforge/<binary>.json`.

### D1 — wire `specforge collect` (C11-00, M)

- Current: `collect.rs` (82 lines) detects a collector by listing directory
  entries, prints `status: ready`, reads nothing.
- Fix (v1 scope — the format `ingest_collector_report` already understands):
  1. `collect.rs` gains a report-input: one or more `--report <path>` args,
     else stdin.
  2. Parse JSON → run through `specforge_wasm::dispatch_collector`
     (`CallSite::Collector`) so collector-side dispatch semantics apply →
     `ingest_collector_report(&report, &known_entity_ids)` where
     `known_entity_ids` comes from the compiled graph
     (`pipeline::compile_with_runtime(path)` → graph node ids).
  3. Merge `coverage_updates` into the project's `specforge-report.json`
     (create/merge `TestReport` shape the analyze pass consumes — this makes
     `collect` → `analyze coverage` a closed loop).
  4. Print (text|JSON): mapped/unmapped counts, per-entity coverage deltas;
     emit `W0xx` warnings for `unmapped_entries` (see D3 for the code).
  5. junit/jest/pytest conversion is explicitly OUT of v1 scope — the
     auto-detect strings stay, but the help text says specforge-test JSON
     reports are what v1 ingests; junit is `planned`. (The audit's
     `--from-junit` is thereby scoped, not silently dropped.)
- Acceptance: `crates/specforge-cli/tests/collect.rs` — e2e: fixture project
  with two entities + a report JSON covering one and orphaning one id →
  collect exits 0, writes specforge-report.json with the mapped entry, warns
  for the orphan; empty dir still exits 1; JSON format stable.
- Note: the wasm runtime here is ComponentRuntime — dispatch goes through the
  component bridge; the collector CallSite unit tests already run against the
  component fixture path (verify; if they use MockRuntime, add one
  ComponentRuntime-based test).

### D2 — coverage gate (C11-01, S)

- Fix: `specforge analyze coverage --min <pct>` — after `pass_coverage`
  computes the percentage, `--min` compares and exits non-zero with an error
  diagnostic (`E0xx` — allocate from the free block; check W/E free codes at
  implementation) naming the threshold and actual. Insertion point:
  `crates/specforge-cli/src/analyze.rs` (flag plumbed into the coverage pass
  call; `--test-results` required when `--min` is used — error if missing).
- Acceptance: `crates/specforge-cli/tests/analyze.rs` — fixture with known
  coverage: `--min` below → exit 0; above → exit non-zero + E-code; missing
  `--test-results` with `--min` → usage error.

### D3 — unmatched-test signal (C11-02, S/M)

- Problem: exact-string matching at every layer; orphaned test records are
  dropped silently on the shipped path (orphan detection exists only inside
  `ingest_collector_report` → `unmapped_entries`, unused in production).
- Fix: (1) D1's collect already surfaces unmapped as warnings (v1); (2)
  `analyze coverage` also reports orphans: extend the `--test-results` loader
  to diff report entity_ids against graph ids and emit a warning per orphan
  (`W0xx — test record references unknown entity '<id>'`; allocate the code
  after checking the free W-range; W097+ candidates). Suggestion text: check
  for renames/typos via `find_close_match` (the existing Jaro-Winkler helper
  with the deterministic tie-break) — top-1 suggestion when score > 0.85.
  Fuzzy matching beyond suggestions stays out of scope (exact-match semantics
  preserved).
- Acceptance: `crates/specforge-cli/tests/analyze.rs` — report with one exact,
  one orphaned id → warning emitted with suggestion; exit code unaffected by
  warnings.
- Commit (D1+D2+D3): `feat(cli): collect ingests test reports; coverage gate + orphan signal`.

---

## Workstream E — emitter/graph/sandbox (C13-00, C7-04, C5-00, C3-06)

### E1 — DOT styles from the registry (C13-00, S/M)

- Problem: `populate.rs:100-104` copies `dot_shape/dot_color/dot_fillcolor`
  into `KindRegistryEntry` (`registries/kind.rs:16-19`); `dot.rs` `emit_dot`
  hardcodes `shape=box` and never reads them; `outline/dot.rs` uses its own
  static COLORS table (adjacent, separate surface).
- Fix: `emit_dot` looks up each node's kind in the KindRegistry and emits
  `shape=`, `color=`, `fillcolor=` when declared (fallback: current defaults);
  quote/escape values through the existing DOT escaping helper. Scope: the
  entity DOT emitter only; outline/dot.rs gets a follow-up note (its
  substring-color scheme is a different visual language — changing it is a
  design choice, not a bug fix).
- Acceptance: strengthen the vacuous `dot_node_shapes` test
  (`emitter/tests/contracts.rs:754-761`) — fixture kind with
  `dot_shape=component, dot_color=red` → emitted DOT contains
  `shape="component"` `color="red"`; kinds without declarations keep current
  output byte-identical (golden).

### E2 — sandbox default + honest wiring status (C7-04, M)

- Verified reality (worse than the audit): `default_sandbox_policy` sets
  `file_system_access = Some(true)` and `is_path_allowed` returns true for an
  empty `allowed_paths` — AND the whole check layer (`host_*_check`) has no
  production caller since the extism host-function surface was removed; it is
  the policy core for the future component host-import surface (R-2 requires
  untrusted plugins be sandboxable — the layer stays).
- Fix:
  1. Flip the defaults to deny-by-default: `file_system_access = Some(false)`;
     `is_path_allowed` with empty `allowed_paths` → false (explicit paths
     still allow). Update `crates/specforge-wasm/src/invariants.rs:22` (the
     test that pins allow-by-default — it asserted the OLD contract) and any
     policy-construction tests in `manifest_bridge.rs`/publish validation.
  2. Document at the top of `sandbox.rs`: this module is the sandbox policy
     core; it is enforced by the component host-import surface (planned) and
     exercised by publish-time manifest validation today; no current guest has
     host access (pure-compute components), so the flip cannot break
     production guests — verified: no host imports in any vendored component.
  3. `builtin_blob_sync`-style guard is unnecessary; instead add a test
     asserting `default_sandbox_policy()` denies a path read and that an
     explicit `allowed_paths: ["/tmp/x"]` allows exactly that prefix tree.
- Acceptance: invariants tests flipped + new default tests; manifest publish
  validation tests (sandbox policy checks in `handlers.rs` step 3) green;
  workspace green.

### E3 — cycle-detector consolidation (C5-00, M)

- Verified inventory (7 traversals): `graph.rs:470-540` (3-color DFS, sorted
  seeds, path-stack exact membership — Phase D ✓), `emitter/compile.rs:700-805`
  (same algorithm re-implemented — Phase D ✓), `resolver/resolve.rs:354-400`
  (import-cycle DFS — different graph: files, not entities), `registry/
  validate.rs:236-320` (peer-dep cycles, own DFS), `wasm/toposort.rs` (Kahn —
  ordering, not membership), `migrate` (target-version walk, not a cycle
  detector), LSP (delegates to pipeline).
- Fix (bounded consolidation): extract ONE exact-membership cycle extractor in
  `specforge-graph` — `pub fn exact_cycles(nodes: &[Sym], adj: &BTreeMap<Sym,
  BTreeSet<Sym>>) -> Vec<Vec<Sym>>` implementing the Phase-D algorithm (sorted
  seeds, 3-color, path-stack, feeder-exact, deterministic order) — and
  re-point `emitter/compile.rs` and `registry/validate.rs` at it (they own the
  semantic mapping to E/W codes). The resolver's file-level DFS and toposort's
  Kahn stay (different semantics, documented in the function docs). Net effect:
  ONE entity-level membership algorithm, three callers, two documented
  non-membership traversals.
- Acceptance: `crates/specforge-graph/tests/cycles.rs` — property-ish fixtures:
  simple cycle, self-loop, feeder-into-cycle (feeder NOT flagged), two
  disjoint cycles, determinism (two runs byte-identical); `determinism.rs`
  cross-process test still green; registry peer-dep tests (W063) green.

### E4 — scope-aware reference resolution (C3-06, M/L — LAST, biggest blast radius)

- Problem: `linker.rs:48-100` resolves against a project-global entity index;
  `compute_file_scopes` (`resolve.rs:467-531`) and the import DAG are computed
  but never consulted — a reference to an entity in a file you never imported
  resolves silently.
- Fix design (staged to control risk):
  1. Build the visible set per file: `declared(file) ∪ exports(import targets)
     transitively` — selective-import binding already exists in the parse
     (`use X as Y`, `use * as Y` at `grammar.js:30-36`); respect the alias map.
  2. Resolution order in `link_references`: (a) same-file declarations; (b)
     imported files' exported entities (alias-aware); (c) global fallback —
     HERE IS THE DECISION POINT: the audit wants (c) demoted. Flip it to a
     **warning first** (`W0xx — resolved outside the import graph; add an
     explicit use`), NOT an error: the corpus, fixtures, and every downstream
     test currently rely on global fallback in unknown places. After one
     release-cycle of warning data, the flip to E003-scoped can happen.
  3. `file_scopes` becomes the authority for (a)/(b); the flat global index
     remains only for (c).
- Validation gate (must all pass before commit): self-host corpus `specforge
  check spec` (expect 0 errors; count new W061-style warnings and TRIAGE them
  — if the corpus itself has out-of-import references, fix the corpus `use`
  statements in the same commit); all workspace tests; fixture projects;
  LSP diagnostics tests (resolution feeds LSP).
- Acceptance: new `crates/specforge-resolver/tests/scopes.rs` — file A declares,
  file B references without import → warning + resolution still succeeds;
  B imports A → no warning; aliased selective import honors the alias; the
  corpus check is clean or corpus-fixed in-commit.
- Commit: `feat(resolver): import-aware reference visibility (global fallback warns)`.

---

## Execution order & commit series

Independent workstreams; series ordered by blast radius (smallest first):

1. **B1** MCP schemas + reflection test
2. **C1+C3** negotiation cleanup + config-schema parity (C2 needs the
   jsonschema dev-dep decision — do together with C2 in the next commit if
   approved, else assert required-key sets)
3. **C2** per-format export schemas
4. **A1-A4** LSP UTF-16 residuals
5. **A5** LSP blocking restructure
6. **D1-D3** CLI data plane
7. **E1 + E2** DOT styles + sandbox flip (independent, batchable)
8. **E3** cycle consolidation
9. **E4** scope-aware resolution (last; includes corpus triage)

Each series: fmt → clippy -D warnings → full workspace tests → component
builtin parity → push → CI watch (established gates). E4 additionally runs the
self-host corpus check and triages any new warnings in-commit.

## Explicit non-goals (recorded, not dropped)

- junit/jest/pytest report conversion for collect (planned; v1 ingests
  specforge-test JSON) — C11-00
- outline/dot.rs registry styling (design choice, follow-up) — C13-00
- component host-import surface that ENFORCES sandbox at runtime (planned WIT
  host imports; C7-04 fixes the policy core + defaults now)
- global-fallback removal in resolution (E4 ships the warning first)
- streaming multipart on registry publish (64 MB bounded buffer is fine) — C14-04 residual inline rusqlite calls ARE in scope via a `db_spawn` helper if time remains after E4 (mechanical, M)
