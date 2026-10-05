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
`RegistryBuild` (kinds, fields, edges, rules, manifests, extension info), the root the project was
compiled from, and its owner's coverage memo. Three constructors: `ProjectView::of(&CompiledProject)`
(the CLI), `ProjectView::of_session(&ProjectSession, root)` (the LSP), `ProjectView::new` (tests and
graphs built in memory); MCP builds every view through `ProjectRef::view()` of its call target (ADR
0014), or `Call::view()` for a call that may have no project. The view owns the recorded test
report (`test_report`), the coverage computed from it (`coverage`) and the versioned schema
(`versioned_schema`, `schema_cache`).

The operations, each one function over the view returning a typed outcome:
`stats::stats` (`Stats`), `coverage::{coverage, row}` (`CoverageOutcome`, `CoverageRow`),
`trace::trace` (`TraceOutcome`, which serializes as the document `specforge trace` writes),
`plan::check` (`PlanOutcome`), `schema::schema` (`SchemaOutcome`), `export::export`,
`model::{model, outline}` (`ModelOutcome`). The CLI and MCP map arguments in and render the
outcome; MCP has one coverage-row presenter (`tools::coverage::row_json`) and one gap presenter
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
- **D7. Stats' diagnostics are the surface's**: the CLI passes what `check` reports, MCP its
  project's diagnostics plus its surface-registration conflicts (`StatsRequest.diagnostics`).
- **D8. An unknown schema kind is refused on both surfaces**: `unknown_kind` naming the closest
  kind; the CLI keeps its message (exit 1) and adds a help line, MCP answers `invalid_input` on
  `kind`. The CLI's `--kind` still prints the kind's entry alone; the operation returns the filtered
  schema MCP serves.
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
  untouched.

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
