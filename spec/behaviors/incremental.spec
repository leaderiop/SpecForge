// Incremental compilation behaviors — watch mode and file change handling

use "events/compilation"
use "invariants/core"
use "invariants/validation"
use "invariants/zero-entity-core"
use "ports/inbound"
use "ports/outbound"
use "types/config"
use "types/core"
use "types/diagnostics"
use "types/graph"
use "types/zero-entity-core"

behavior watch_file_system_for_changes "Watch File System for Changes" {
  features   [incremental_compilation]
  invariants [incremental_correctness, watch_mode_response_latency]
  category   command
  types      [FileEntry]
  ports      [FileSystem]
  produces   [file_changed]
  requires {
    watch_mode_active "specforge watch command is active and the file watcher is initialized on the spec root"
  }
  ensures {
    file_changed_emitted "file_changed event is produced for every detected file creation, modification, or deletion"
  }
  contract   """
    When specforge watch is active, the system MUST monitor all .spec files
    under the spec root for changes using the OS file watching API.
    File creation, modification, and deletion MUST each trigger
    recompilation of affected files. Changed paths are classified by the
    project session (classify_project_changes); after an environment
    reload the watcher follows the session's new watch roots.
  """
  verify unit "file modification triggers recompilation"
  verify unit "file creation triggers recompilation"
  verify unit "file deletion triggers recompilation"
  verify integration "watch detects changes within 100ms"
  verify contract "Watch File System for Changes: file system watching holds for the declared obligations"
  verify integration "a specforge.lock change reloads the environment"
  verify integration "a .wasm file no extension loads changes nothing"
  verify integration "after spec_root changes, files under the new spec root are watched"
}

behavior classify_project_changes "Classify Project Changes" {
  features   [incremental_compilation]
  invariants [incremental_correctness]
  category   command
  ports      [FileSystem]
  contract   """
    A project session MUST classify a changed path by what the project is
    built from: a .spec file that discovery finds under the spec root is a
    source change, keyed relative to the spec root; specforge.json,
    specforge.lock and every extension module the environment loaded (an
    installed extension's extension.wasm, a local .wasm entry) are
    environment changes; specforge-cache.json, which check-phase passes
    read, and every file a file_reference field or a file_exists rule
    names, which the checks look for, are check-input changes; any other
    path changes nothing.
    Watch, the LSP and MCP MUST classify through the session.
  """
  verify unit "a discovered .spec file is a source change keyed relative to the spec root"
  verify unit "specforge.json, specforge.lock and a loaded extension module are environment changes"
  verify unit "a .wasm file no extension loads changes nothing"
  verify unit "specforge-cache.json re-runs the checks without re-parsing"
  verify unit "a file a file_reference field names re-runs the checks"
  verify unit "a file a file_exists rule names re-runs the checks"
  verify unit "an excluded or undiscovered .spec file changes nothing"
  verify unit "a detached session classifies a .spec buffer as a source and nothing else as an input"
  verify unit "a session's watch roots cover every input it classifies"
}

behavior bring_session_up_to_date "Bring a Session Up to Date with Disk" {
  features   [incremental_compilation]
  invariants [incremental_correctness]
  category   command
  ports      [FileSystem]
  contract   """
    A project session opened from disk MUST bring itself up to date
    without a file watcher: it compares every discovered source and every
    environment and check input with what it last built from (size and
    modification time; a file modified within the timestamp granularity of
    the last build counts as changed unless its content is unchanged) and
    applies exactly those changes: sources by an update, environment
    inputs by an environment reload, check inputs by re-running the
    checks. Afterwards its graph and diagnostics MUST be those a fresh
    compile of the files on disk produces. specforge.json MUST be read
    once per environment load, the extension runtime and the environment
    both built from that read, and every input MUST be stamped before
    anything reads it, the extension runtime included, so a file written
    while the session loads is seen next time.
  """
  verify unit "an up-to-date session reports no change and re-parses nothing"
  verify unit "edits, creations and deletions since the last build are applied as one update"
  verify unit "a file rewritten within the timestamp granularity of the last build is still seen"
  verify unit "a specforge.lock change reloads the environment"
  verify unit "after bringing itself up to date a session matches a fresh compile"
  verify unit "a specforge.json or module written while the extension runtime loads is seen next time"
}

behavior invalidate_changed_files "Invalidate Changed Files" {
  features   [incremental_compilation]
  invariants [incremental_correctness, graph_traversal_integrity]
  category   validation
  types      [Graph, Subgraph, FileEntry]
  consumes   [file_changes_coalesced]
  produces   [subgraph_invalidated]
  requires {
    file_changes_coalesced_fired "file_changes_coalesced event has fired, providing a batch of changed files from the debounce stage"
  }
  ensures {
    invalidation_set_computed    "Invalidation set is exactly the changed files"
    subgraph_invalidated_emitted "subgraph_invalidated event is produced with the computed invalidation set"
    unrelated_files_untouched    "Files outside the invalidation set are not re-parsed"
  }
  contract   """
    When a coalesced batch of file changes is received from the debounce
    stage (or an editor buffer changes), the system MUST compute the
    invalidation set: exactly the changed files. A parse depends only on
    its own file's text, and references resolve across the project
    without use (ADR 0004 D1-a), so an importer of a changed file parses
    the same as before and MUST NOT be re-parsed; the cross-file effects
    of a change (references, duplicates, cycles, imports) are recomputed
    over the whole graph and every cached parse instead. For file
    deletions, all entities declared in the deleted file MUST be removed
    from the graph along with their edges; no re-parse is attempted.
    For file creations, the new file MUST be parsed and its entities
    added to the graph.
  """
  verify unit "only the changed files are re-parsed"
  verify unit "an importer of a changed file is not re-parsed"
  verify unit "unrelated files are not re-parsed"
  verify unit "deleted file entities removed from graph"
  verify unit "new file entities added to graph"
  verify contract "Invalidate Changed Files: file invalidation holds — file_changes_coalesced_fired, invalidation_set_computed, subgraph_invalidated_emitted, unrelated_files_untouched"
}

behavior rebuild_affected_subgraph "Rebuild Affected Subgraph" {
  features   [incremental_compilation]
  invariants [incremental_correctness, graph_traversal_integrity, zero_domain_knowledge_core]
  category   command
  types      [Graph, Subgraph, FileEntry]
  ports      [SourceParser]
  // Sequential dependency: resolve_imports_on_update consumes
  // subgraph_invalidated and produces import_dag_updated. Therefore
  // import_dag_updated always arrives AFTER subgraph_invalidated.
  // This is a correctness gate, not a parallel join barrier.
  consumes   [subgraph_invalidated, import_dag_updated]
  produces   [incremental_rebuild_complete]
  requires {
    subgraph_invalidated "subgraph_invalidated event has fired, providing the set of invalidated files"
    import_dag_updated   "import_dag_updated event has fired, confirming every file's imports were resolved again"
  }
  ensures {
    graph_reflects_reparse "In-memory graph reflects the re-parsed state of all invalidated files"
    stale_removed          "Stale nodes and edges from invalidated files are removed"
    new_added              "New nodes and edges from re-parsed files are added"
    rebuild_event_fired    "incremental_rebuild_complete event fires with accurate rebuilt file and node counts"
  }
  maintains {
    unaffected_subgraph_intact "Nodes and edges from non-invalidated files remain unchanged throughout rebuild"
  }
  contract   """
    After re-parsing invalidated files using SourceParser.parseIncremental,
    the system MUST remove stale nodes and edges from the graph using
    the mutable graph interface (per [maintain_mutable_graph]), then add
    new nodes from the re-parsed ASTs (each ID going to its first
    declaration in path order, as in a cold build) and re-link references
    over the whole graph. The result MUST be identical to a full cold
    rebuild — identical means same node set, same edge set, same field
    values, same diagnostic set (order-independent comparison). With
    --verify-incremental, and always in a debug build, each rebuild is
    compared with a full cold rebuild of the same sources, and its delta
    is checked by validate_delta_correctness. The
    rebuild MUST operate on generic entity nodes — it MUST NOT contain
    logic specific to any entity kind. All kind-specific validation is
    deferred to the extension validation phase after the subgraph is
    rebuilt.
  """
  verify unit "stale nodes are removed"
  verify unit "new nodes are added"
  verify property "incremental rebuild equals cold rebuild"
  verify unit "debug --verify-incremental performs cold rebuild comparison"
  verify contract "Rebuild Affected Subgraph: affected subgraph rebuild holds — subgraph_invalidated, import_dag_updated, graph_reflects_reparse, stale_removed, new_added, rebuild_event_fired, unaffected_subgraph_intact"
}

behavior emit_incremental_diagnostics "Emit Incremental Diagnostics" {
  features   [incremental_compilation]
  invariants [
    multi_error_collection,
    incremental_correctness,
    diagnostic_determinism,
    zero_domain_knowledge_core,
    watch_mode_response_latency,
  ]
  category   command
  types      [DiagnosticBag, DiagnosticsDelta]
  consumes   [incremental_rebuild_complete, graph_delta_computed, incremental_validators_dispatched]
  produces   [incremental_diagnostics_complete]
  requires {
    incremental_rebuild_complete_fired      "incremental_rebuild_complete event has fired, confirming subgraph rebuild is done"
    graph_delta_computed_fired              "graph_delta_computed event has fired, providing the diff between old and new graph"
    incremental_validators_dispatched_fired "incremental_validators_dispatched event has fired, confirming extension validators have run"
  }
  ensures {
    diagnostics_refreshed           "Diagnostics from invalidated files are replaced with fresh validation results"
    unchanged_diagnostics_preserved "Diagnostics from non-invalidated files remain unchanged"
    incremental_diagnostics_emitted "incremental_diagnostics_complete event fires with the merged diagnostic bag"
  }
  maintains {
    non_invalidated_diagnostics_stable "Diagnostics the change does not affect are not modified"
  }
  // These three events form a sequential chain, not a parallel fan-in:
  // incremental_rebuild_complete → graph_delta_computed → incremental_validators_dispatched
  // Listing all three as consumed events is a completeness declaration,
  // not a parallel join. The behavior activates on the last event.
  contract   """
    After incremental rebuild, the system MUST re-validate the affected
    subgraph and emit updated diagnostics. This behavior MUST wait for
    both compute_graph_delta and dispatch_incremental_validators to
    complete (via graph_delta_computed and
    incremental_validators_dispatched respectively) before emitting.
    Extension diagnostics MUST be collected into the final bag only
    after both prerequisites are satisfied. Re-validation MUST include all
    registered core validation passes and all extension-contributed
    validation passes; they run over the whole patched graph, since
    references cross files without use. The diagnostic set MUST be the
    one a full rebuild reports: diagnostics from invalidated files are
    replaced with fresh results, and a diagnostic in another file changes
    only when the change affects it (an unresolved reference to an
    entity that went away, a duplicate, a cycle) — if a change to file A
    does not affect file B, B's diagnostics MUST remain unchanged. This
    behavior orchestrates the diagnostic collection, not the extension
    invocation directly.
  """
  verify unit "diagnostics from changed files are refreshed"
  verify unit "diagnostics from unchanged files are preserved"
  verify unit "total diagnostic set matches full rebuild"
  verify performance "file change to diagnostics emitted within 100ms"
  verify contract "Emit Incremental Diagnostics: incremental diagnostics holds for the declared obligations"
}

behavior debounce_file_changes "Debounce File Changes" {
  features   [incremental_compilation]
  invariants [incremental_correctness, diagnostic_determinism, watch_mode_response_latency]
  category   command
  types      [FileEntry, CompilerConfig]
  consumes   [file_changed]
  produces   [file_changes_coalesced]
  requires {
    file_changed_fired "At least one file_changed event has been received from the file watcher"
  }
  ensures {
    coalesced_batch_produced          "file_changes_coalesced event fires with the union of all changed files within the debounce window"
    redundant_recompilation_prevented "Multiple rapid changes to the same file result in a single recompilation"
  }
  contract   """
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
  verify contract "Debounce File Changes: file change debouncing holds — file_changed_fired, coalesced_batch_produced, redundant_recompilation_prevented"
}

behavior resolve_imports_on_update "Resolve Imports on Every Update" {
  features   [incremental_compilation]
  // Runs synchronously before rebuild_affected_subgraph — every file's
  // imports are resolved again before any subgraph rebuild begins.
  category   command
  invariants [import_dag, incremental_correctness]
  types      [Graph, FileEntry]
  consumes   [subgraph_invalidated]
  produces   [import_dag_updated]
  requires {
    subgraph_invalidated_fired "subgraph_invalidated event has fired, identifying the set of files to re-parse"
  }
  ensures {
    import_dag_updated_emitted "import_dag_updated event fires after every cached file's use imports are resolved again"
    cycle_detection_rerun      "Import cycle detection (W113) has been re-run across the full import graph"
  }
  contract   """
    After every update, the system MUST resolve the use imports of every
    file again, over the cached parses (no file is re-read or re-parsed
    for it), so the import diagnostics (E025, I004, W113, W027) are the
    ones a full rebuild reports. The import graph is rebuilt rather than
    patched: an added or removed import, an import target created or
    deleted, and a cycle closed or broken anywhere are all seen on the
    update that causes them. References resolve across the project
    without use (ADR 0004 D1-a), so the import graph decides no re-parse.
  """
  verify unit "an added use import is resolved on the next update"
  verify unit "a removed use import no longer reports"
  verify unit "cycle detection re-runs after an update"
  verify unit "import diagnostics after an update match a full rebuild"
  verify contract "Resolve Imports on Every Update: import resolution after each update holds — subgraph_invalidated_fired, import_dag_updated_emitted, cycle_detection_rerun"
}

// ── Incremental Graph Delta ───────────────────────────────────

behavior compute_graph_delta "Compute Graph Delta" {
  features   [incremental_graph_deltas]
  invariants [
    incremental_correctness,
    graph_traversal_integrity,
    diagnostic_determinism,
    graph_delta_determinism,
  ]
  category   query
  types      [Graph, GraphDelta, NodeChange, ModifiedNodeChange]
  consumes   [incremental_rebuild_complete]
  produces   [graph_delta_computed]
  requires {
    previous_graph_available "Previous graph snapshot is available for comparison"
    new_graph_available      "Newly compiled graph is available for comparison"
  }
  ensures {
    complete_diff      "GraphDelta is a complete symmetric diff of the two graphs"
    deterministic_sort "All arrays in GraphDelta are sorted by EntityId.raw (lexicographic)"
  }
  maintains {
    delta_equivalence "Applying the delta to the previous graph produces a state identical to the new graph"
  }
  contract   """
    After an incremental rebuild completes, the system MUST diff the
    previous graph state against the new graph state to produce a
    GraphDelta. The delta MUST enumerate all added nodes, removed nodes,
    modified nodes (with what changed: field names, or kind, title,
    methods, edges), added edges, removed edges, and the list of affected
    files. Source positions are ignored: an entity that only moved is not
    modified. added_nodes, removed_nodes, and modified_nodes MUST be
    sorted by EntityId.raw to guarantee deterministic output. The delta
    MUST be computed before any subscribers are notified. One delta
    serves every surface: watch prints it, MCP notifies it.
  """
  verify unit "added nodes appear in delta"
  verify unit "removed nodes appear in delta"
  verify unit "modified nodes list changed fields"
  verify unit "added and removed edges appear in delta"
  verify unit "affected files listed in delta"
  verify contract "Compute Graph Delta: graph delta computation holds — previous_graph_available"
}

// Not built (ADR 0006): every check runs over the whole graph after each
// update. The obligations stay unproven until extensions receive deltas.
behavior dispatch_incremental_validators "Dispatch Incremental Validators" {
  features   [incremental_graph_deltas]
  invariants [incremental_correctness, diagnostic_determinism, zero_domain_knowledge_core]
  category   command
  types      [GraphDelta, Graph, EntityKindDescriptor]
  ports      [WasmRuntime]
  consumes   [graph_delta_computed]
  produces   [incremental_validators_dispatched]
  requires {
    delta_computed "graph_delta_computed event has fired and GraphDelta is available"
  }
  ensures {
    all_validators_invoked "All extension validators invoked with appropriate input (delta or full graph)"
    event_produced         "incremental_validators_dispatched event produced on completion"
  }
  maintains {
    topological_order "Dispatch follows topological extension order regardless of delta content"
  }
  contract   """
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
  verify contract "Dispatch Incremental Validators: incremental dispatch holds — all_validators_invoked"
}

behavior notify_delta_subscribers "Notify Delta Subscribers" {
  features   [incremental_graph_deltas]
  invariants [incremental_correctness, diagnostic_determinism, graph_traversal_integrity]
  category   command
  types      [GraphDelta]
  // MCP notification is handled by notify_graph_delta_via_mcp in behaviors/mcp-server.spec.
  // The LSP is not a delta subscriber: provide_semantic_tokens in behaviors/lsp.spec
  // owns its semantic token refresh.
  consumes   [graph_delta_computed]
  produces   [delta_subscribers_notified]
  requires {
    graph_delta_computed_fired "graph_delta_computed event has fired, providing the GraphDelta for notification"
  }
  ensures {
    affected_files_delivered           "Each update reports its GraphDelta and the files it affects"
    delta_subscribers_notified_emitted "delta_subscribers_notified event fires once the update is reported"
  }
  contract   """
    After every update, the project session MUST report the GraphDelta
    and the files it affects with the update, for whoever holds the
    session to deliver: specforge watch prints its counts, and MCP
    notifies its subscribed clients (notify_graph_delta_via_mcp) and diffs
    the diagnostics into a DiagnosticsDelta itself
    (notify_diagnostics_delta_via_mcp). The session never waits on a
    subscriber, so a slow one cannot delay the compilation pipeline. The
    LSP does not use these deltas: it recompiles on its own document
    changes and asks its client to refresh semantic tokens
    (provide_semantic_tokens, behaviors/lsp.spec).
  """
  verify unit "an update reports its delta and the files it affects"
  verify contract "Notify Delta Subscribers: delta reporting holds — graph_delta_computed_fired, affected_files_delivered, delta_subscribers_notified_emitted"
}

behavior validate_delta_correctness "Validate Delta Correctness" {
  features   [incremental_graph_deltas]
  invariants [incremental_correctness, graph_delta_determinism]
  category   validation
  types      [Graph, GraphDelta]
  consumes   [graph_delta_computed]
  produces   [delta_validation_failed, delta_validation_passed]
  requires {
    graph_delta_available "graph_delta_computed event has fired and GraphDelta is available for verification"
    debug_mode_active     "Debug build configuration or --verify-incremental CLI flag is active"
  }
  ensures {
    delta_verified           "Delta applied to previous graph produces state identical to new graph, or the discrepancy is reported"
    validation_event_emitted "delta_validation_passed or delta_validation_failed event is produced"
  }
  contract   """
    In debug mode, after computing a graph delta, the system MUST verify
    correctness: the delta MUST equal a full comparison of the previous
    and new graphs, and applying it to the previous graph's node IDs and
    edges MUST yield the new graph's. Any discrepancy MUST be reported
    with the rebuild (specforge watch reports the verification as
    failed) with a descriptive message identifying the inconsistent nodes
    or edges: a node ID or edge mismatch between the applied delta and
    the new graph, or a modified node missing from either graph.

    Debug mode is activated by the compiler's debug build configuration or the
    --verify-incremental CLI flag. The CLI flag enables delta validation
    in release builds for CI use. This check MUST be disabled in release
    builds (without --verify-incremental) to avoid performance overhead.
  """
  verify unit "delta applied to old graph equals new graph"
  verify unit "a discrepancy is reported with a message naming what differs"
  verify unit "check disabled in release builds"
  verify integration "a debug build checks each rebuild without the flag"
  verify unit "a rebuild that passes the check is reported as passed"
  verify contract "Validate Delta Correctness: delta correctness validation holds — graph_delta_available, debug_mode_active, delta_verified, validation_event_emitted"
}
