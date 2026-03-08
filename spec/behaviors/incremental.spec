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
  invariants [incremental_correctness, watch_mode_response_latency]
  types      [FileEntry]
  ports      [FileSystem]
  produces   [file_changed]

  requires {
    watch_mode_active "specforge watch command is active and the file watcher is initialized on the spec root"
  }

  ensures {
    file_changed_emitted "file_changed event is produced for every detected file creation, modification, or deletion"
  }

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
  verify contract    "requires/ensures consistency for file system watching"

}

behavior invalidate_changed_files "Invalidate Changed Files" {
  invariants [incremental_correctness, graph_traversal_integrity]
  types      [Graph, Subgraph, FileEntry]
  consumes   [file_changes_coalesced]
  produces   [subgraph_invalidated]

  requires {
    file_changes_coalesced_fired "file_changes_coalesced event has fired, providing a batch of changed files from the debounce stage"
  }

  ensures {
    invalidation_set_computed "Invalidation set contains the changed files plus all transitive importers"
    subgraph_invalidated_emitted "subgraph_invalidated event is produced with the computed invalidation set"
    unrelated_files_untouched "Files outside the invalidation set are not re-parsed"
  }

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
  verify contract "requires/ensures consistency for file invalidation"

}

behavior rebuild_affected_subgraph "Rebuild Affected Subgraph" {
  invariants [incremental_correctness, graph_traversal_integrity, zero_domain_knowledge_core]
  types      [Graph, Subgraph, FileEntry]
  ports      [SourceParser]
  // Sequential dependency: track_import_dag_incrementally consumes
  // subgraph_invalidated and produces import_dag_updated. Therefore
  // import_dag_updated always arrives AFTER subgraph_invalidated.
  // This is a correctness gate, not a parallel join barrier.
  consumes   [subgraph_invalidated, import_dag_updated]
  produces   [incremental_rebuild_complete]

  requires {
    subgraph_invalidated "subgraph_invalidated event has fired, providing the set of invalidated files"
    import_dag_updated "import_dag_updated event has fired, confirming the import DAG reflects latest file dependencies"
  }

  ensures {
    graph_reflects_reparse "In-memory graph reflects the re-parsed state of all invalidated files"
    stale_removed "Stale nodes and edges from invalidated files are removed"
    new_added "New nodes and edges from re-parsed files are added"
    rebuild_event_fired "incremental_rebuild_complete event fires with accurate rebuilt file and node counts"
  }

  maintains {
    unaffected_subgraph_intact "Nodes and edges from non-invalidated files remain unchanged throughout rebuild"
  }

  contract """
    After re-parsing invalidated files using SourceParser.parseIncremental,
    the system MUST remove stale nodes and edges from the graph using
    the mutable graph interface (per [maintain_mutable_graph]), then add
    new nodes and edges from the re-parsed ASTs. The result MUST be
    identical to a full cold rebuild — identical means same node set,
    same edge set, same field values, same diagnostic set
    (order-independent comparison). In debug builds, the concrete
    comparison mechanism is validate_delta_correctness, which performs a
    full cold rebuild and diffs against the incremental result. The
    rebuild MUST operate on generic entity nodes — it MUST NOT contain
    logic specific to any entity kind. All kind-specific validation is
    deferred to the extension validation phase after the subgraph is
    rebuilt.
  """

  verify unit        "stale nodes are removed"
  verify unit        "new nodes are added"
  verify property    "incremental rebuild equals cold rebuild"
  verify unit        "debug --verify-incremental performs cold rebuild comparison"
  verify contract "requires/ensures consistency for affected subgraph rebuild"

}

behavior emit_incremental_diagnostics "Emit Incremental Diagnostics" {
  invariants [multi_error_collection, incremental_correctness, diagnostic_determinism, zero_domain_knowledge_core, watch_mode_response_latency]
  types      [DiagnosticBag, DiagnosticsDelta]
  consumes   [incremental_rebuild_complete, graph_delta_computed, incremental_validators_dispatched]
  produces   [incremental_diagnostics_complete]

  requires {
    incremental_rebuild_complete_fired "incremental_rebuild_complete event has fired, confirming subgraph rebuild is done"
    graph_delta_computed_fired "graph_delta_computed event has fired, providing the diff between old and new graph"
    incremental_validators_dispatched_fired "incremental_validators_dispatched event has fired, confirming extension validators have run"
  }

  ensures {
    diagnostics_refreshed "Diagnostics from invalidated files are replaced with fresh validation results"
    unchanged_diagnostics_preserved "Diagnostics from non-invalidated files remain unchanged"
    incremental_diagnostics_emitted "incremental_diagnostics_complete event fires with the merged diagnostic bag"
  }

  maintains {
    non_invalidated_diagnostics_stable "Diagnostics for files outside the invalidation set are not modified"
  }

  // These three events form a sequential chain, not a parallel fan-in:
  // incremental_rebuild_complete → graph_delta_computed → incremental_validators_dispatched
  // Listing all three as consumed events is a completeness declaration,
  // not a parallel join. The behavior activates on the last event.
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
  verify contract    "requires/ensures consistency for incremental diagnostics"

}

behavior debounce_file_changes "Debounce File Changes" {
  invariants [incremental_correctness, diagnostic_determinism, watch_mode_response_latency]
  types      [FileEntry, CompilerConfig]
  consumes   [file_changed]
  produces   [file_changes_coalesced]

  requires {
    file_changed_fired "At least one file_changed event has been received from the file watcher"
  }

  ensures {
    coalesced_batch_produced "file_changes_coalesced event fires with the union of all changed files within the debounce window"
    redundant_recompilation_prevented "Multiple rapid changes to the same file result in a single recompilation"
  }

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
  verify contract "requires/ensures consistency for file change debouncing"

}

behavior track_import_dag_incrementally "Track Import DAG Incrementally" {
  // Runs synchronously before rebuild_affected_subgraph — the import DAG
  // must be up-to-date before any subgraph rebuild begins.
  invariants [import_dag, incremental_correctness]
  types      [Graph, FileEntry]
  consumes   [subgraph_invalidated]
  produces   [import_dag_updated]

  requires {
    subgraph_invalidated_fired "subgraph_invalidated event has fired, identifying the set of files to re-parse"
  }

  ensures {
    import_dag_updated_emitted "import_dag_updated event fires after the DAG reflects added and removed use imports"
    cycle_detection_rerun "Import cycle detection (E003) has been re-run across the full import DAG"
  }

  contract """
    When a file is re-parsed during incremental compilation, the system
    MUST update the file-level import DAG to reflect any added or removed
    use import statements. Added imports MUST create new edges in the file
    dependency graph. Removed imports MUST delete the corresponding edges.
    After updating, the system MUST re-check for import cycles (E003)
    across the full import DAG (not just the affected subgraph), because
    a local edge change may create or break a cycle through nodes outside
    the invalidation set. The import DAG MUST remain consistent with the
    result of a full rebuild.
  """

  verify unit "added use import creates file dependency edge"
  verify unit "removed use import deletes file dependency edge"
  verify unit "cycle detection re-runs after import DAG update"
  verify property "incremental import DAG matches full rebuild import DAG"
  verify contract "requires/ensures consistency for incremental import DAG tracking"

}

// ── Incremental Graph Delta ───────────────────────────────────

behavior compute_graph_delta "Compute Graph Delta" {
  invariants [incremental_correctness, graph_traversal_integrity, diagnostic_determinism, graph_delta_determinism]
  types      [Graph, GraphDelta, NodeChange, ModifiedNodeChange]
  consumes   [incremental_rebuild_complete]
  produces   [graph_delta_computed]

  requires {
    previous_graph_available "Previous graph snapshot is available for comparison"
    new_graph_available      "Newly compiled graph is available for comparison"
  }

  ensures {
    complete_diff          "GraphDelta is a complete symmetric diff of the two graphs"
    deterministic_sort     "All arrays in GraphDelta are sorted by EntityId.raw (lexicographic)"
  }

  maintains {
    delta_equivalence      "Applying the delta to the previous graph produces a state identical to the new graph"
  }

  contract """
    After an incremental rebuild completes, the system MUST diff the
    previous graph state against the new graph state to produce a
    GraphDelta. The delta MUST enumerate all added nodes, removed nodes,
    modified nodes (with changed field names), added edges, removed edges,
    and the list of affected files. added_nodes, removed_nodes, and
    modified_nodes MUST be sorted by EntityId.raw to guarantee
    deterministic output. The delta MUST be computed before any
    subscribers are notified. ModifiedNodeChange old_value and new_value
    fields MUST be populated when delta_include_values is true in
    CompilerConfig (default false for token efficiency per P3). In debug
    mode (the compiler's debug build configuration or --verify-incremental), old_value and
    new_value MUST always be populated regardless of configuration.
  """

  verify unit "added nodes appear in delta"
  verify unit "removed nodes appear in delta"
  verify unit "modified nodes list changed fields"
  verify unit "added and removed edges appear in delta"
  verify unit "affected files listed in delta"
  verify unit "delta_include_values=true populates old_value and new_value"
  verify unit "delta_include_values=false omits old_value and new_value"
  verify contract "requires/ensures consistency for graph delta computation"

}

behavior dispatch_incremental_validators "Dispatch Incremental Validators" {
  invariants [incremental_correctness, diagnostic_determinism, zero_domain_knowledge_core]
  types      [GraphDelta, Graph, ManifestEntityKind]
  ports      [WasmRuntime]
  consumes   [graph_delta_computed]
  produces   [incremental_validators_dispatched]

  requires {
    delta_computed         "graph_delta_computed event has fired and GraphDelta is available"
  }

  ensures {
    all_validators_invoked "All extension validators invoked with appropriate input (delta or full graph)"
    event_produced         "incremental_validators_dispatched event produced on completion"
  }

  maintains {
    topological_order      "Dispatch follows topological extension order regardless of delta content"
  }

  contract """
    After a graph delta is computed, the system MUST dispatch validation
    to extensions. Extensions that declare incremental=true in their
    manifest MUST receive only the GraphDelta. Extensions without
    incremental support MUST receive the full graph for re-validation.
    The incremental: false flag is per-kind, not per-extension. When a
    delta contains entities of a kind marked incremental: false, the
    dispatcher MUST invoke the owning extension with the full graph for
    those entities, even if other kinds from the same extension support
    incremental validation. Dispatch MUST follow the topological
    extension order.
  """

  verify unit "incremental extension receives delta only"
  verify unit "non-incremental extension receives full graph"
  verify unit "dispatch follows topological order"
  verify unit "kind with incremental=false triggers full graph validation for that kind"
  verify unit "mixed incremental and non-incremental kinds dispatch separately"
  verify contract "requires/ensures consistency for incremental dispatch"

}

behavior notify_delta_subscribers "Notify Delta Subscribers" {
  invariants [incremental_correctness, diagnostic_determinism, graph_traversal_integrity]
  types      [GraphDelta, DiagnosticsDelta]
  // MCP notification is handled by notify_graph_delta_via_mcp in behaviors/mcp-server.spec
  ports      [LspProtocol]
  consumes   [graph_delta_computed]
  produces   [delta_subscribers_notified]

  requires {
    graph_delta_computed_fired "graph_delta_computed event has fired, providing the GraphDelta for notification"
  }

  ensures {
    lsp_notified "LSP subscribers receive semantic token staleness notifications for affected files"
    diagnostics_delta_delivered "DiagnosticsDelta (added and removed diagnostics) is delivered to subscribers"
    delta_subscribers_notified_emitted "delta_subscribers_notified event fires after all notifications are dispatched"
  }

  contract """
    After a graph delta is computed, the system MUST notify LSP
    subscribers. The LSP MUST receive notification that semantic tokens
    for affected files are stale; the LSP client then re-requests full
    semantic tokens (SemanticTokenDelta push is deferred to a future
    iteration). Diagnostics MUST be updated as a DiagnosticsDelta
    (added and removed diagnostics). Notification delivery MUST be
    non-blocking — a slow subscriber MUST NOT delay the compilation
    pipeline. MCP notification is handled separately by
    notify_graph_delta_via_mcp (behaviors/mcp-server.spec).
  """

  verify unit "LSP receives semantic token updates for affected files"
  verify unit "diagnostics delta includes added and removed"
  verify unit "slow subscriber does not block pipeline"
  verify contract "requires/ensures consistency for delta subscriber notification"

}

behavior validate_delta_correctness "Validate Delta Correctness" {
  invariants [incremental_correctness, graph_delta_determinism]
  types      [Graph, GraphDelta]
  consumes   [graph_delta_computed]
  produces   [delta_validation_failed, delta_validation_passed]

  requires {
    graph_delta_available "graph_delta_computed event has fired and GraphDelta is available for verification"
    debug_mode_active "Debug build configuration or --verify-incremental CLI flag is active"
  }

  ensures {
    delta_verified "Delta applied to previous graph produces state identical to new graph, or assertion failure raised"
    validation_event_emitted "delta_validation_passed or delta_validation_failed event is produced"
  }

  contract """
    In debug mode, after computing a graph delta, the system MUST verify
    correctness by applying the delta to the previous graph state and
    comparing the result with the new graph state. Any discrepancy MUST
    trigger a debug assertion failure with a descriptive message identifying
    the inconsistent nodes or edges. Discrepancies include: different node
    count, different edge count, node ID mismatch, or edge mismatch
    between applied-delta and new graph state.

    Debug mode is activated by the compiler's debug build configuration or the
    --verify-incremental CLI flag. The CLI flag enables delta validation
    in release builds for CI use. This check MUST be disabled in release
    builds (without --verify-incremental) to avoid performance overhead.
  """

  verify unit "delta applied to old graph equals new graph"
  verify unit "discrepancy triggers debug assertion with descriptive message"
  verify unit "check disabled in release builds"
  verify unit "successful validation emits delta_validation_passed with node and edge counts"
  verify contract "requires/ensures consistency for delta correctness validation"

}
