# One incremental rebuild, inside the project session

**Status:** accepted (2026-10-02)

The incremental rebuild was split three ways: `specforge-watch` picked the files to re-parse from
an import DAG, the project session seeded that DAG and resolved imports again on its own, and MCP
diffed graphs with a delta type of its own. The DAG only matched imports spelled `x` or `x.spec`
(no relative, `@alias` or `index.spec` targets), so the files it picked were wrong anyway.

The rebuild now lives in `specforge-project` (`incremental.rs`, `delta.rs`) behind
`ProjectSession`. `specforge-watch` is the file watcher and its debounce.

- **No import DAG.** A parse depends only on its own text, and references resolve across the
  project without `use` (ADR 0004 D1-a), so re-parsing an importer gains nothing. An update
  re-parses exactly the changed files, re-links references over the whole graph, recomputes the
  graph-build diagnostics over every cached parse (duplicates go to the first declaration by path),
  resolves every file's imports again and runs every check. A changed file discovery would not find
  (excluded, under `target/`, `build/`, ...) is ignored, as a cold compile ignores it.
- **One `GraphDelta`** for the session, watch and MCP, with MCP's notion of "modified": kind,
  title, fields (positions stripped), methods or outgoing edges differ, so moving an entity is not
  a modification. MCP's `specforge/graphChanged` payload is unchanged; watch's `modified_nodes`
  count follows the same rule. `delta_include_values` and `old_value`/`new_value` were never built
  and leave the spec.
- **`--verify-incremental`** also checks that the delta equals a full comparison of the two graphs
  and applies to the previous one.
- **Every import resolution step refuses a target above the spec root**, not only relative paths:
  go-to-definition on `use` goes through `specforge_resolver::resolve_import`, the compile's
  cascade, and a bare path such as `sub/../../x` (or an `@alias` target, which `specforge.json`
  cannot set yet) that leaves the spec root is E025 in a compile too. Such a target was never
  compiled, so its entities were missing anyway.
- **Known gap:** `dispatch_incremental_validators` was never wired; every check runs over the whole
  graph after an update. Its planner is deleted; the behavior stays until extensions receive deltas.
