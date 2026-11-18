requirement FEATURE-050 "Incremental Compilation" {
  category "Feature"
  tier     v1
  text     """
    Full recompilation on every file change is too slow for interactive
    development. With 500+ .spec files, users need sub-100ms feedback
    when editing a single file.

    Watch mode monitors the filesystem for changes, debounces rapid edits,
    re-parses only the changed files (references resolve across the
    project without use, so no importer needs it), patches the graph with
    them, resolves every file's imports again, and re-validates. The
    incremental rebuild lives in the project session that CLI watch mode,
    the LSP and MCP each hold, to ensure identical behavior. Target:
    <100ms file-change-to-diagnostics.
  """
}

requirement FEATURE-051 "Incremental Graph Deltas" {
  category "Feature"
  tier     v1
  text     """
    After incremental recompilation, subscribers (MCP, agents) receive
    the full graph and must diff it themselves to determine what changed.
    This wastes computation and token budget. Agents in live workflows
    need precise change information to update their context incrementally
    rather than re-reading the entire graph.

    First-class GraphDelta events after incremental rebuilds. The compiler
    diffs previous and new graph states, producing a delta with added/removed/
    modified nodes and edges. Extensions with incremental support receive only
    the delta. The LSP does not subscribe to deltas: it recompiles on its
    own document changes and asks the editor to refresh semantic tokens
    when the graph changed. Debug mode validates delta correctness by
    round-tripping.
  """
}
