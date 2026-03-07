// Incremental compilation behaviors — watch mode and file change handling

use invariants/core
use invariants/validation
use invariants/zero-entity-core
use types/core
use types/config
use types/graph
use types/diagnostics
use types/zero-entity-core
use ports/inbound
use ports/outbound
use events/compilation

behavior watch_file_system_for_changes "Watch File System for Changes" {
  invariants [incremental_correctness]
  types      [FileEntry]
  ports      [FileSystem]
  produces   [file_changed]

  contract """
    When specforge watch is active, the system MUST monitor all .spec files
    under the spec root for changes using the OS file watching API.
    File creation, modification, and deletion MUST each trigger
    recompilation of affected files.
  """

  verify unit        "file modification triggers recompilation"
  verify unit        "file creation triggers recompilation"
  verify unit        "file deletion triggers recompilation"
  verify integration "watch detects changes within 100ms"

}

behavior invalidate_changed_files "Invalidate Changed Files" {
  invariants [incremental_correctness]
  types      [Graph, Subgraph, FileEntry]
  consumes   [file_changes_coalesced]
  produces   [subgraph_invalidated]

  contract """
    When a coalesced batch of file changes is received from the debounce
    stage, the system MUST compute the
    invalidation set: the changed file plus all files that transitively
    import it. Only invalidated files MUST be re-parsed. The system
    MUST NOT re-parse files outside the invalidation set.
    Delegates subgraph computation to compute_subgraph_for_invalidation (behaviors/graph). For file
    deletions, all entities declared in the deleted file MUST be removed
    from the graph along with their edges; no re-parse is attempted.
    For file creations, the new file MUST be parsed and its entities
    added to the graph, then dependents of any files that import it
    MUST be invalidated.
  """

  verify unit "changed file is in invalidation set"
  verify unit "direct importers are in invalidation set"
  verify unit "transitive importers are in invalidation set"
  verify unit "unrelated files are not re-parsed"
  verify unit "deleted file entities removed from graph"
  verify unit "new file entities added to graph"

}

behavior rebuild_affected_subgraph "Rebuild Affected Subgraph" {
  invariants [incremental_correctness, graph_traversal_integrity, zero_domain_knowledge_core]
  types      [Graph, Subgraph, FileEntry]
  ports      [SourceParser]
  consumes   [subgraph_invalidated, import_dag_updated]
  produces   [incremental_rebuild_complete]

  contract """
    After re-parsing invalidated files using SourceParser.parseIncremental,
    the system MUST remove stale nodes and edges from the graph using
    the mutable graph interface (per [maintain_mutable_graph]), then add
    new nodes and edges from the re-parsed ASTs. The result MUST be
    identical to a full cold rebuild — identical means same node set,
    same edge set, same field values, same diagnostic set
    (order-independent comparison). The rebuild MUST operate on generic
    entity nodes — it MUST NOT contain logic specific to any entity kind.
    All kind-specific validation is deferred to the extension validation
    phase after the subgraph is rebuilt.
  """

  verify unit        "stale nodes are removed"
  verify unit        "new nodes are added"
  verify property    "incremental rebuild equals cold rebuild"

}

behavior emit_incremental_diagnostics "Emit Incremental Diagnostics" {
  invariants [multi_error_collection, incremental_correctness, diagnostic_determinism, zero_domain_knowledge_core, watch_mode_response_latency]
  types      [DiagnosticBag, DiagnosticsDelta]
  consumes   [incremental_rebuild_complete, graph_delta_computed, incremental_validators_dispatched]
  produces   [incremental_diagnostics_complete]

  // Join barrier: waits for all three consumed events before emitting diagnostics
  contract """
    After incremental rebuild, the system MUST re-validate the affected
    subgraph and emit updated diagnostics. This behavior MUST wait for
    both compute_graph_delta and dispatch_incremental_validators to
    complete (via graph_delta_computed and
    incremental_validators_dispatched respectively) before emitting.
    Extension diagnostics MUST be collected into the final bag only
    after both prerequisites are satisfied. Re-validation MUST include all
    registered core validation passes and all extension-contributed
    validation passes scoped to entities within the affected subgraph.
    Diagnostics from invalidated files MUST be replaced with fresh
    results. Diagnostics from non-invalidated files MUST be preserved
    unchanged — if file B was not invalidated, its validation
    diagnostics MUST remain unchanged regardless of changes in file A. Extension validation is dispatched via
    dispatch_incremental_validators, which handles the WasmRuntime
    interaction. This behavior orchestrates the diagnostic collection,
    not the extension invocation directly.
  """

  verify unit "diagnostics from changed files are refreshed"
  verify unit "diagnostics from unchanged files are preserved"
  verify unit "total diagnostic set matches full rebuild"
  verify performance "file change to diagnostics emitted within 100ms"

}

behavior debounce_file_changes "Debounce File Changes" {
  invariants [incremental_correctness, diagnostic_determinism, watch_mode_response_latency]
  types      [FileEntry, CompilerConfig]
  consumes   [file_changed]
  produces   [file_changes_coalesced]

  contract """
    When multiple file_changed events arrive in rapid succession (e.g.,
    save-all or editor reformatting), the system MUST coalesce them into a
    single invalidation batch. A configurable debounce window (default 50ms)
    MUST be applied: the system MUST wait until no new changes arrive within
    the window before emitting a file_changes_coalesced event. The coalesced
    batch MUST include the union of all changed files within the debounce
    window.
  """

  verify unit "rapid successive changes coalesced into single batch"
  verify unit "debounce window prevents redundant recompilation"
  verify unit "coalesced batch includes union of all changed files"
  verify unit "single isolated change triggers after debounce window"

}

behavior track_import_dag_incrementally "Track Import DAG Incrementally" {
  // Runs synchronously before rebuild_affected_subgraph — the import DAG
  // must be up-to-date before any subgraph rebuild begins.
  invariants [import_dag, incremental_correctness]
  types      [Graph, FileEntry]
  consumes   [subgraph_invalidated]
  produces   [import_dag_updated]

  contract """
    When a file is re-parsed during incremental compilation, the system
    MUST update the file-level import DAG to reflect any added or removed
    use import statements. Added imports MUST create new edges in the file
    dependency graph. Removed imports MUST delete the corresponding edges.
    After updating, the system MUST re-check for import cycles (E003)
    within the affected subgraph. The import DAG MUST remain consistent
    with the result of a full rebuild.
  """

  verify unit "added use import creates file dependency edge"
  verify unit "removed use import deletes file dependency edge"
  verify unit "cycle detection re-runs after import DAG update"
  verify property "incremental import DAG matches full rebuild import DAG"

}

// ── Incremental Graph Delta ───────────────────────────────────

behavior compute_graph_delta "Compute Graph Delta" {
  invariants [incremental_correctness, graph_traversal_integrity, diagnostic_determinism]
  types      [Graph, GraphDelta, NodeChange, ModifiedNodeChange]
  consumes   [incremental_rebuild_complete]
  produces   [graph_delta_computed]

  contract """
    After an incremental rebuild completes, the system MUST diff the
    previous graph state against the new graph state to produce a
    GraphDelta. The delta MUST enumerate all added nodes, removed nodes,
    modified nodes (with changed field names), added edges, removed edges,
    and the list of affected files. added_nodes, removed_nodes, and
    modified_nodes MUST be sorted by EntityId.raw to guarantee
    deterministic output. The delta MUST be computed before any
    subscribers are notified.
  """

  verify unit "added nodes appear in delta"
  verify unit "removed nodes appear in delta"
  verify unit "modified nodes list changed fields"
  verify unit "added and removed edges appear in delta"
  verify unit "affected files listed in delta"

}

behavior dispatch_incremental_validators "Dispatch Incremental Validators" {
  invariants [incremental_correctness, diagnostic_determinism, zero_domain_knowledge_core]
  types      [GraphDelta, Graph, ManifestEntityKind]
  ports      [WasmRuntime]
  consumes   [graph_delta_computed]
  produces   [incremental_validators_dispatched]

  contract """
    After a graph delta is computed, the system MUST dispatch validation
    to extensions. Extensions that declare incremental=true in their
    manifest MUST receive only the GraphDelta. Extensions without
    incremental support MUST receive the full graph for re-validation.
    Entity kinds with incremental: false in ManifestEntityKind MUST
    skip incremental rebuild and always trigger full revalidation for
    entities of that kind. Dispatch MUST follow the topological
    extension order.
  """

  verify unit "incremental extension receives delta only"
  verify unit "non-incremental extension receives full graph"
  verify unit "dispatch follows topological order"

}

behavior notify_delta_subscribers "Notify Delta Subscribers" {
  invariants [incremental_correctness, diagnostic_determinism]
  types      [GraphDelta, DiagnosticsDelta]
  // MCP notification is handled by notify_graph_delta_via_mcp in behaviors/mcp-server.spec
  ports      [LspProtocol]
  consumes   [graph_delta_computed]
  produces   [delta_subscribers_notified]

  contract """
    After a graph delta is computed, the system MUST notify LSP
    subscribers: the LSP MUST receive semantic token updates for affected
    files and diagnostics MUST be updated as a DiagnosticsDelta (added
    and removed diagnostics). Notification delivery MUST be
    non-blocking — a slow subscriber MUST NOT delay the compilation
    pipeline. Note: LSP clients currently re-request full semantic
    tokens after delta notification; SemanticTokenDelta support is
    deferred to a future iteration.
  """

  verify unit "LSP receives semantic token updates for affected files"
  verify unit "diagnostics delta includes added and removed"
  verify unit "slow subscriber does not block pipeline"

}

behavior validate_delta_correctness "Validate Delta Correctness" {
  invariants [incremental_correctness]
  types      [Graph, GraphDelta]
  consumes   [graph_delta_computed]
  produces   [delta_validation_failed]

  contract """
    In debug mode, after computing a graph delta, the system MUST verify
    correctness by applying the delta to the previous graph state and
    comparing the result with the new graph state. Any discrepancy MUST
    trigger a debug assertion failure with a descriptive message identifying the inconsistent
    nodes or edges. This check MUST be disabled in release builds to
    avoid performance overhead.
  """

  verify unit "delta applied to old graph equals new graph"
  verify unit "discrepancy triggers debug assertion with descriptive message"
  verify unit "check disabled in release builds"

}
