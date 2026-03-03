// Incremental compilation behaviors — watch mode and file change handling

use invariants/core
use types/core
use types/graph
use types/diagnostics
use ports/outbound

behavior watch_file_system_for_changes "Watch File System for Changes" {
  invariants [incremental_correctness]
  ports      [FileSystem]

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

  contract """
    When a file change is detected, the system MUST compute the
    invalidation set: the changed file plus all files that transitively
    import it. Only invalidated files MUST be re-parsed. The system
    MUST NOT re-parse files outside the invalidation set.
  """

  verify unit "changed file is in invalidation set"
  verify unit "direct importers are in invalidation set"
  verify unit "transitive importers are in invalidation set"
  verify unit "unrelated files are not re-parsed"
}

behavior rebuild_affected_subgraph "Rebuild Affected Subgraph" {
  invariants [incremental_correctness]
  types      [Graph, Subgraph]

  contract """
    After re-parsing invalidated files, the system MUST remove stale nodes
    and edges from the graph, then add new nodes and edges from the
    re-parsed ASTs. The result MUST be identical to a full cold rebuild.
  """

  verify unit        "stale nodes are removed"
  verify unit        "new nodes are added"
  verify property    "incremental rebuild equals cold rebuild"
}

behavior emit_incremental_diagnostics "Emit Incremental Diagnostics" {
  invariants [multi_error_collection, incremental_correctness, diagnostic_determinism]
  types      [DiagnosticBag]

  contract """
    After incremental rebuild, the system MUST re-validate the affected
    subgraph and emit updated diagnostics. Diagnostics from invalidated
    files MUST be replaced with fresh results. Diagnostics from
    non-invalidated files MUST be preserved unchanged.
  """

  verify unit "diagnostics from changed files are refreshed"
  verify unit "diagnostics from unchanged files are preserved"
  verify unit "total diagnostic set matches full rebuild"
}
