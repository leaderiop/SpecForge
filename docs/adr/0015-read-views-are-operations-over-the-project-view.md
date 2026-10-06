# The read views are operations over the project view

**Status:** accepted (2026-10-05)

`specforge-ops` handed the read views out as building blocks and each surface chained them on its
own: the CLI's `stats` built a `CoverageRegistries` by hand and called `compute_project_stats`, MCP
did the same through its own `project_coverage`; each surface built `TraceExpectations` and picked
single or every trace; each ran `generate_schema` and versioned it, and each chained
`ModelIntermediate_from_schema → with_theme_colors → filter_entities → filter_fields → render`. Every
fix landed twice (stats 56f2bbce + 4d71702c, schema version 6feb6ab1 + 1d0a1db1, eight coverage
commits), and five drifts were live: the test report was read at three roots (`stats <sub>` read
`<sub>/specforge-report.json`, `analyze --path <sub>` walked up to the ancestor's, MCP read its
root); the schema cache at two (`specforge export <sub>` walked up to the project's cache, versioned a
schema compiled without the project's config against it, printed 20 W053 and overwrote it with
0 kinds); the MCP coverage tool listed the union types stats leaves out; MCP trace invented a
`gaps: ["no upstream links", ...]` list beside the real missing links; and MCP answered an unknown
schema kind with an empty schema where the CLI refused it. Model warnings reached the CLI's stderr
and were never read by MCP.

`analyze` was already the deep shape: one call over a `ProjectView`, one typed outcome, 40-line
adapters. Architecture plan 02 makes it the pattern for every read view.

## The project view

`specforge_ops::view::ProjectView` borrows what every read operation reads: the graph, the whole
`RegistryBuild` (kinds, fields, edges, rules, the extension declarations and their ordered passes,
ADR 0012), the root the project was compiled from, and its owner's coverage memo. Three constructors: `ProjectView::of(&CompiledProject)`
(the CLI), `ProjectView::of_session(&ProjectSession, root)` (the LSP), `ProjectView::new` (tests and
graphs built in memory); MCP builds every view through `ProjectRef::view()` of its call target (ADR
0014), or `Call::view()` for a call that may have no project. The view owns the recorded test
report (`test_report`), the coverage computed from it (`coverage`) and the versioned schema
(`versioned_schema`, `schema_cache`).

The operations, each one function over the view returning a typed outcome:
`stats::stats` (`Stats`), `coverage::{coverage, row}` (`CoverageOutcome`, `CoverageRow`),
`trace::trace` (`TraceOutcome`, which serializes as the document `specforge trace` writes),
`plan::check` (`PlanOutcome`), `schema::schema` (`SchemaOutcome`), `export::export`,
`model::{model, outline}` (`ModelOutcome`), `inspect::inspect` (`EntityFacts`, section "Inspect").
The CLI and MCP map arguments in and render the outcome, and the LSP hover renders inspect's; MCP
has one coverage-row presenter (`tools::coverage::row_json`) and one gap presenter
(`tools::trace::gap_json`).

## Decisions

- **D1. The view's root is the root the project was compiled from**, and the recorded test report
  (`<root>/specforge-report.json`) and the schema cache (`<root>/.specforge/schema-cache.json`) are
  read there, never in an ancestor. The compile never walks up (`Environment::load` reads
  `<root>/specforge.json`), so a walk-up read a report and a cache belonging to a project whose config
  was not loaded. `specforge analyze --path <sub>` now reads `<sub>`'s report, as `stats <sub>`
  already did, and `specforge export <sub>` versions against and records in `<sub>/.specforge/`.
  **Follow-up, not done here:** the CLI half of ADR 0014's call target, resolving `--path` to its
  project root once for every command, would change what `check`, `format`, `collect` and every
  other command compile for a sub-path. Until then `specforge collect --path <sub>` still writes the
  report at the project root, where views over `<sub>` do not read it.
- **D2. Coverage is memoized per compile and per report content.** `RecordedCoverage`
  (`specforge-project`) holds the parsed report and the coverage computed from it, keyed on the
  report's path and a hash of its bytes: one file read per call, no parse and no assessment of every
  entity when nothing changed. mtime is not the key (a rewrite within the same second would be
  missed). A `CompiledProject` and a `ProjectSession` each own one; the session starts a fresh one on
  every update, re-check and reload, and MCP replaces its session when it serves a graph built in
  memory, so no invalidation can be forgotten. Errors are never memoized. The versioned schema is
  not memoized: it is a function of the registries and one small file, and its formats are the
  emitter's (ADR 0007).
- **D3. The unfiltered coverage view lists the entities that count toward coverage**, extending
  ADR 0004 D2-b to MCP's listing: `rows = stats' testable count`, covered rows = the proven count. An
  `entity_id` still reaches any entity; every row says whether it is `exempt` (a testable-kind
  entity that owes no obligations and declares none). An unknown `status_filter`, outside the tool's
  declared enum, is `invalid_input` with a did-you-mean.
- **D4. "Unverified" has one definition**: counts toward coverage and is not proven
  (`ProjectCoverage::is_unverified`, `CoverageRow::unverified`). Rejected: "declares no `verify`"
  (ignores proof, flags non-testable kinds) and "testable kind and not covered" (flags exempt unions).
  The trace prompt adopts it with plan 07.
- **D5. The gap vocabulary.** A **missing link** is an expected edge (from the registries) a traced
  entity lacks: `missing` in every trace document, the only gap a trace reports. A **plan gap** is
  how an agent plan falls short (`unresolved_entity`, `missing_plan_entry`, `ordering`). A dangling
  edge is E003, `check`'s business, never a gap. `ops::trace::Gap` is the presentation union of the
  first two (`McpTraceGap`). MCP's invented "no upstream/downstream links" is gone: isolation is an
  empty `upstream` or `downstream`, and `specforge.trace` returns exactly the CLI's document (its
  output schema is a `oneOf` of that document and `McpTracePlanResult`).
- **D6. Stats keeps two JSON shapes**, `ProjectStatistics` and `McpStatsResult` (both published), as
  ADR 0004 D1-c accepted for diagnostics. The numbers live once, in the operation; a parity test
  holds the two shapes to the same numbers.
- **D7. Stats' diagnostics are the surface's**: the CLI's view reports what `check` reports, MCP's
  its project's diagnostics plus its surface-registration notices (`ProjectView::reported`, see
  "Management operations"; `StatsRequest` is gone).
- **D8. An unknown schema kind is refused on both surfaces**: `unknown_kind` naming the closest
  kind; the CLI keeps its message (exit 1) and adds a help line, MCP answers `invalid_input` on
  `kind`. ~~The CLI's `--kind` still prints the kind's entry alone; the operation returns the filtered
  schema MCP serves.~~ Amended by [ADR 0027](0027-an-enumerated-argument-is-one-option-table.md)'s round
  (architecture plan 2026-10-06 12, D4): `specforge schema --kind` prints the operation's outcome, the
  document `specforge.schema` returns (the kind and the edge types that touch it), and `--publish`
  goes through `ops::schema::json_schema`; `--kind` cannot be combined with `--publish`.
- **D9. Model warnings are W146 on both surfaces**: CLI stderr `warning[W146]: model: …`, MCP the
  tool result's diagnostics. A registry-built schema only carries known field types, but the model
  accepts any Graph Protocol schema, and a catalogued code reaches `explain` and the docs.
- **D10. Only `specforge export` records the schema cache**, at the view's root (D1). MCP only
  reads it.
- **D11. Trace errors are typed**: `TraceError::EntityNotFound { entity_id, near }` replaces the
  emitter error that carried `E003: …` in its message for MCP to re-parse. It converts to an
  `OpError` E003 with a did-you-mean; the CLI prints `error[E003]: …` with a help line.
- **D12. The CLI's read commands hold a `CompiledProject`** (`pipeline::compile_project`), since
  `ProjectView::of` needs the registry build and the memo. `check` and the other commands are
  untouched; the management commands hold one too since "Management operations".

## Consequences

- MCP clients see: `specforge.coverage {}` and `status_filter: uncovered` without union types,
  abstract entities and governance entities that declare nothing; an `exempt` field on every row;
  no `gaps` on an entity trace; an error for an unknown schema kind or coverage status; W146 in
  model results.
- Sub-path invocations change (D1); the R1/R2 reproductions of plan 02 are tests.
- No ADR conflicts: D3 extends ADR 0004 D2-b, D4 is consistent with D2-a, D6 mirrors D1-c, and ADR
  0004 is silent on roots (27c48e54 had already chosen the view's own root for MCP).
- Found on the way: `specforge outline` printed a different document on most runs (a HashMap
  transitive closure); it is deterministic now.

## What would reopen it

A read view that needs more than the project view (the build cache, another project), or a third
surface that needs a shape of its own.

## Management operations (amendment, architecture round 3, plan 05)

The read views took the project view; the management operations still took the project in
pieces. `extension::list(root, enabled, loaded, kinds, graph)` read `specforge.json` again, `remove`
took an eight-field request (five of them project pieces) and read it a third time, `doctor`,
`providers`, `collect` (six parameters; both callers built `KnownEntities` themselves) and
`infer::{progress, gaps}` each took their own slice. Every change to the Environment's shape touched
seven CLI adapters and eight MCP handlers (714144a7, 6e040512, 94bbdeb3, aad9160d), and the slices
drifted: 94bbdeb3 gave `list` and `remove` the `enabled` entries and not `doctor`, which called a
`.wasm` file entry "builtin"; `remove` uninstalled the binary and emptied the lock before it found
`specforge.json` unreadable.

### The view, completed

`ProjectView` also borrows the **Environment** it was compiled in (`env()`: the config, what each
`extensions` entry enabled, the spec root, the registry build; `registries()` is its one accessor,
the view keeps no copy) and says **what its
surface reports** for the project (`reported()`): a `CompiledProject`'s diagnostics, a session's
plus MCP's I017 notices (`also_reporting`), or a listed slice (`reporting`, for graphs built in
memory). The view's fields are private: `graph()`, `env()`, `registries()` and `root()` are the
way in. `ProjectView::new` takes `&Environment` (`Environment::with_registries` for a bare build).
`project_root()` is the root for an operation that reads or writes the project on disk, `no_project`
without one.

### The operations

Each is one function over the view and a request: `extension::list(&view) -> ExtensionListing`,
`extension::providers(&view) -> ProviderListing`, `extension::remove(&view, &RemoveRequest { name,
force, dry_run })`, `doctor::diagnose(&view)`, `collect::collect(&view, runtime, Request { runner,
mode, consent, announce })`, `infer::{progress, progress_or_fresh}(&view)`, `infer::gaps(&view,
runtime)`. `stats::stats(&view)` reads `reported()` (D7 amended). The CLI compiles once
(`pipeline::compile_project`) for every command; `CompilationContext` is deleted.

### Decisions

- **M1. The view borrows the whole Environment**, not chosen fields: every owner of a view owns
  one, and the next Environment field reaches the operations without touching a constructor. It
  adds no dependency: the Environment is `specforge-project`, which the LSP links; the `Registry`
  port (ADR 0010) is not in the view.
- **M2. Reported diagnostics are the view's, asked of its owner on demand.** The surface still
  decides what it reports, by how it builds its view. `check` keeps its `reported` parameter (ADR
  0018).
- **M3. The extension runtime is a parameter** of the operations that run extension code
  (`collect`, `gaps`, as `analyze`), never part of the view.
- **M4. Disk is the view root's.** `specforge.lock`, installed binaries, source files and the
  recorded report are read and written at `root`; `remove`, `collect`, `progress` and `gaps` refuse
  a rootless view (`no_project`); the listings and doctor answer from what the view enabled and
  loaded, doctor skipping the installation checks.
- **M5. `remove` decides every refusal from the view before it writes, then writes
  `specforge.json` first**, then the lock and the binary. A `specforge.json` the compile could not
  parse refuses every removal with `config_invalid` and changes nothing.
- **M6. One source rule** (`Origin::of`) names where an extension comes from in the listing and in
  doctor: `file:<path>` for a `.wasm` file entry, the lock entry's source, `builtin`, else `unknown`.
- **M7. One presenter per listing** (`ExtensionEntry::to_json`, `ProviderListing::to_json`); doctor
  keeps two published shapes (ADR 0004 D1-c).
- **M8. `add`, `update`, `init` and `migrate` are not view operations**: they run before or instead
  of a compile, and `add`/`update` reach the `Registry` port. `add` and `update` read the config
  through `config::usable`/`config::required` (`read_project_config`, the function the compile reads
  it with) and the lock through `LockState::at` (M10), never a reader of their own (see M11).
- **M9. A `specforge.json` not used as written is E069, an error.** The Environment keeps every way
  the file is not used as written (`config_problems`: unreadable, not JSON, not an object, a key of
  the wrong type, a non-string `extensions` or `exclude` item) and
  reports each as E069, first; when nothing could load, I002 names the file instead of "no
  extensions configured". `remove` refuses from those reasons without reading the file again (M5),
  when one blocks the edit. It is an error under ADR 0018's unchanged verdict rule ("no error among
  what was reported"), as E028 is for one extension that does not load: a project whose config
  loads nothing must not pass `check`. Doctor reports it as an error, and reports a missing
  `specforge.json` as the warning finding `config_missing`.
- **M10. `specforge.lock` is read once, by the Environment.** `Environment::lock` is a typed
  `specforge_wasm::LockState` (`Absent`, `Read`, or `Unreadable` with its E033 problem), read at
  `Environment::load` and reloaded when the file changes (it is an environment input); the view's
  `lock()` hands it to `list`, `doctor` and `remove` (none without a root), so they read what the
  compile read, not the disk again. It is a typed result, not a diagnostic: a corrupt lock does not
  fail `check` (it did not before), and `doctor` lists it as the error finding `lock_unreadable`
  naming E033. `specforge_wasm::lock_path` is the one definition of where the lock lives, used by the
  Environment, the extension loader and the root-based `add` and `update` (M8), which read it with
  the same `LockState::at`.

- **M11. One refusal for an unusable `specforge.json`.** `add`, `update` and `remove` refuse a
  config that `ConfigProblem::blocks_edits` names with `config::refusal`: code `config_invalid`
  (kind `schema_mismatch`), the problem's text, which is E069's reason, as the message; before they
  install, write or delete anything, dry runs included. `remove` builds it from the compile's
  problems (M5); `add` and `update`, which run without a compile (M8), from `read_project_config`.
  `config::edit_extensions`, the one writer, refuses through the same function, so a config that
  changed after the compile is refused too. A missing `specforge.json` is `config_not_found` for
  `add` (hint: `specforge init`) and not refused by `update`, which only reads the lock. `add` used
  to refuse with E032 ("extension install or uninstall failed"), a code about something else.

### Consequences

- `specforge doctor` and `specforge.doctor` report a `.wasm` file entry's source as
  `file:<path>`, and an extension of unknown origin as `unknown`, not `builtin`.
- `specforge remove` and `specforge.remove_extension` with an unreadable `specforge.json` change
  nothing (they used to uninstall first); `specforge add`, `specforge.add_extension` and `specforge
  update` refuse it the same way (`config_invalid`, not E032).
- A `specforge.json` that is there and not used as written is the error E069 on every surface
  (`check`, watch, the LSP, MCP validate and doctor), and I002 says the file could not be read when
  nothing loaded; `check` fails on it where it passed, and so does `specforge doctor`.
- `specforge doctor` where there is no `specforge.json` reports the warning finding `config_missing`.
- No other output changes; the parity harness holds the CLI and MCP listings, doctor, inference and
  collect equal as before.

### What would reopen it

An operation that needs a project the view cannot describe (two projects, or the build cache), or
a management operation that must run without a compile.

## Inspect

*(Added 2026-10, architecture plan 07.)*

MCP `specforge.inspect` and the LSP hover answered "what is this entity" from two sources: inspect
from navigate's references, the coverage row, the headline statement and the diagnostics' data;
hover from raw graph edges and the kind registry entry. Of 27 commits to inspect, the one that
touched hover renamed a heading by hand (61f38250). Hover showed no coverage and none of the entity's
diagnostics unless they were under the cursor, and nothing held its `testable` badge to inspect's
field (ADR 0004 D2-d, which the spec states "as hover shows it"). A third copy, the MCP context
prompt, read the same headline, edges and obligations.

- **I1. Inspect is a read view**: `specforge_ops::inspect::inspect(view, entity_id) ->
  Result<EntityFacts, OpError>`. `EntityFacts` borrows the node and its kind's registry entry and
  carries the headline statement, the standing (the entity snapshot's `snapshot::Standing`,
  borrowed: `testable`, `obligated()`, `exempt()`; inspect keeps no standing type of its own), the obligations, the
  references (`navigate::References`, one per edge, in edge order, with the peer's kind and the
  field), the coverage (`Result<EntityCoverage, ReportError>`) and the diagnostics the view reports
  about it (`navigate::is_about`). MCP inspect, the context prompt and the hover only render it.
- **I2. A reference list without spans** is navigate's (`References::of`), selected by the same
  `reference_edges` as `Navigator::references`. Inspect reads no spec file.
- **I3. The standing does not depend on the recorded report**, and the coverage carries the report's
  error instead of failing the view: MCP still fails `inspect` on E045 (ADR 0004 D2-e), and the hover
  shows the error on its Coverage line.
- **I4. The hover is editor-neutral markdown**: no raw `lsp_icon` word (a SymbolKind name, which
  document symbols use); the VS Code client adds its codicons (for a kind it does not know, the
  codicon of the SymbolKind the server reports for the entity's workspace symbol) and finds the
  entity header on any line.
- **I5. Additive MCP fields**: `exempt` (the coverage row's meaning), `obligated` and `source_extension`;
  `references` and `reference_count` stay deprecated aliases, now derived from the same reference
  list.

Consequences: hover gains the headline as a summary, a Coverage line and the entity's diagnostics not
already shown, and a long non-ASCII field value no longer crashes the server. A parity test holds
hover and inspect to the same facts on every fixture entity. The reported diagnostics are the
view's (section "Management operations"): MCP's view reports its call target's, the LSP's reports
what it published (`ProjectView::reporting`), so the hover's dedupe against the cursor's
diagnostics compares the same copies. While the LSP's session is out for an update, the hover says
coverage is unavailable rather than reading a stand-in view with no root.
