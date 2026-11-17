// Incremental compilation feature

use "behaviors/graph"
use "behaviors/incremental"
use "behaviors/lsp"

feature incremental_compilation "Incremental Compilation" {
  status   done
  // Bridge: shared_incremental_pipeline (peer behavior, also listed in live_diagnostics in features/lsp.spec)
  problem  """
    Full recompilation on every file change is too slow for interactive
    development. With 500+ .spec files, users need sub-100ms feedback
    when editing a single file.
  """
  solution """
    Watch mode monitors the filesystem for changes, debounces rapid edits,
    re-parses only the changed files (references resolve across the
    project without use, so no importer needs it), patches the graph with
    them, resolves every file's imports again, and re-validates. The
    incremental rebuild lives in the project session that CLI watch mode,
    the LSP and MCP each hold, to ensure identical behavior. Target:
    <100ms file-change-to-diagnostics.
  """
}

feature incremental_graph_deltas "Incremental Graph Deltas" {
  status   done
  // notify_graph_delta_via_mcp is part of the MCP feature, not this one.
  // See behaviors/mcp-server.spec for the MCP delta notification behavior.
  // Cross-feature: emit_incremental_diagnostics (incremental_compilation) consumes
  // graph_delta_computed as a sequential prerequisite before emitting updated diagnostics.
  problem  """
    After incremental recompilation, subscribers (MCP, agents) receive
    the full graph and must diff it themselves to determine what changed.
    This wastes computation and token budget. Agents in live workflows
    need precise change information to update their context incrementally
    rather than re-reading the entire graph.
  """
  solution """
    First-class GraphDelta events after incremental rebuilds. The compiler
    diffs previous and new graph states, producing a delta with added/removed/
    modified nodes and edges. Extensions with incremental support receive only
    the delta. The LSP does not subscribe to deltas: it recompiles on its
    own document changes and asks the editor to refresh semantic tokens
    when the graph changed. Debug mode validates delta correctness by
    round-tripping.
  """
}
