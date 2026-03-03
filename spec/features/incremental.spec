// Incremental compilation feature

use behaviors/incremental
use behaviors/graph

feature incremental_compilation "Incremental Compilation" {
  behaviors [maintain_mutable_graph, compute_subgraph_for_invalidation, watch_file_system_for_changes, invalidate_changed_files, rebuild_affected_subgraph, emit_incremental_diagnostics]

  problem """
    Full recompilation on every file change is too slow for interactive
    development. With 500+ .spec files, users need sub-100ms feedback
    when editing a single file.
  """

  solution """
    Watch mode monitors the filesystem for changes, computes the minimal
    invalidation set (changed file + transitive dependents), re-parses
    only invalidated files, rebuilds affected subgraph edges, and
    re-validates. Target: <100ms file-change-to-diagnostics.
  """
}
