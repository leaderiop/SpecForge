// Incremental compilation feature

use behaviors/incremental
use behaviors/graph
use behaviors/lsp

feature incremental_compilation "Incremental Compilation" {
  // Bridge: shared_incremental_pipeline (owned by live_diagnostics in features/lsp.spec)
  behaviors [maintain_mutable_graph, watch_file_system_for_changes, debounce_file_changes, compute_subgraph_for_invalidation, invalidate_changed_files, track_import_dag_incrementally, rebuild_affected_subgraph, emit_incremental_diagnostics, shared_incremental_pipeline]

  problem """
    Full recompilation on every file change is too slow for interactive
    development. With 500+ .spec files, users need sub-100ms feedback
    when editing a single file.
  """

  solution """
    Watch mode monitors the filesystem for changes, debounces rapid edits,
    computes the minimal invalidation set (changed file + transitive
    dependents), updates the import DAG, re-parses only invalidated files,
    rebuilds affected subgraph edges, and re-validates. The incremental
    pipeline is shared between CLI watch mode and LSP to ensure identical
    behavior. Target: <100ms file-change-to-diagnostics.
  """
}

feature incremental_graph_deltas "Incremental Graph Deltas" {
  behaviors [compute_graph_delta, dispatch_incremental_validators, notify_delta_subscribers, validate_delta_correctness]
  // notify_graph_delta_via_mcp is part of the MCP feature, not this one.
  // See behaviors/mcp-server.spec for the MCP delta notification behavior.
  // Cross-feature: emit_incremental_diagnostics (incremental_compilation) consumes
  // graph_delta_computed as a join barrier before emitting updated diagnostics.

  problem """
    After incremental recompilation, subscribers (LSP, MCP, agents) receive
    the full graph and must diff it themselves to determine what changed.
    This wastes computation and token budget. Agents in live workflows
    need precise change information to update their context incrementally
    rather than re-reading the entire graph.
  """

  solution """
    First-class GraphDelta events after incremental rebuilds. The compiler
    diffs previous and new graph states, producing a delta with added/removed/
    modified nodes and edges. Extensions with incremental support receive only
    the delta. LSP gets targeted semantic token updates. Debug mode validates
    delta correctness by round-tripping.
  """
}
