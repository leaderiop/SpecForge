# One incremental rebuild, inside the project session

**Status:** accepted (2026-10-02)

The incremental rebuild was split three ways: `specforge-watch` held a pipeline that picked files to
re-parse from an import DAG, the project session seeded that DAG and separately resolved every
file's imports after each update, and MCP diffed graphs with a delta type of its own. The DAG
matched imports only as `x` or `x.spec`, missing relative, `@alias` and `index.spec` targets, so
the files it picked were wrong anyway. The rebuild now lives in `specforge-project`
(`incremental.rs`, `delta.rs`) behind `ProjectSession`; `specforge-watch` is the file watcher and
its debounce.

- **No import DAG.** A parse depends only on its own text, and references resolve across the project
  without `use` (ADR 0004 D1-a), so re-parsing an importer gains nothing. An update re-parses exactly
  the changed files, re-links references over the whole graph and resolves every file's imports
  again (E025, I004, W113, W027), as before. Nothing order-dependent needed the DAG: duplicates go to
  the first declaration by path over all cached parses, and W113 comes from the resolver's own
  import graph. `invalidate_changed_files` now says the invalidation set is the changed files;
  `track_import_dag_incrementally` became `resolve_imports_on_update`; `compute_subgraph_for_invalidation`
  and `compute_invalidation_set` are gone.
- **One `GraphDelta`**, used by the session, watch and MCP. Its notion of "modified" is the one MCP
  already sent: kind, title, fields (positions stripped), methods or outgoing edges differ, so moving
  an entity is not a modification. MCP's `specforge/graphChanged` payload is unchanged (it lists node
  IDs); watch's `modified_nodes` count follows the same rule. A rebuild computes the delta from the
  nodes the changed files held and the edge set, without copying the graph; MCP takes the delta of
  the reload it runs instead of cloning the previous graph.
- **`--verify-incremental`** (always on in a debug build of watch) compares each rebuild with a cold
  one, and checks that its delta equals the full comparison and applies to the previous graph
  (`validate_delta_correctness`, which was a function nothing called).
- **Trimmed:** `delta_include_values` and the delta's `old_value`/`new_value` were never configurable;
  they leave the spec. The unused `DeltaSubscriber` registry is gone; `notify_delta_subscribers` now
  says what happens: the session reports each update's delta and its holder delivers it.
- **Known gap, left unproven:** `dispatch_incremental_validators`. Its planner was never wired;
  every check runs over the whole graph after an update. The planner and its tests are deleted, the
  behavior stays until extensions receive deltas.
- **Go-to-definition on `use`** resolves through `specforge_resolver::resolve_import`, the compile's
  cascade. Every step of that cascade now refuses a target above the spec root, not only relative
  paths.
