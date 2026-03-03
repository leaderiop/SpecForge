// Graph building behaviors — constructing the in-memory graph

use invariants/core
use types/core
use types/graph

behavior build_in_memory_graph "Build In-Memory Graph" {
  invariants [string_interning_consistency, entity_id_uniqueness]
  types      [Graph, Node, Edge, SpecFile, EdgeType, FileIndex]

  contract """
    After resolution, the compiler MUST construct an in-memory directed
    graph where each entity becomes a node and each cross-reference
    becomes a typed edge. The graph MUST contain exactly one node per
    declared entity and one edge per resolved reference.
  """

  verify unit "graph contains one node per entity"
  verify unit "graph contains one edge per resolved reference"
  verify unit "edge types match relationship semantics"

  tests ["../crates/specforge-graph/src/builder.rs"]
}

behavior maintain_mutable_graph "Maintain Mutable Graph" {
  invariants [incremental_correctness]
  types      [Graph, Subgraph]

  contract """
    The in-memory graph MUST support incremental mutation: adding nodes,
    removing nodes, adding edges, and removing edges. After mutation,
    the graph MUST remain consistent — no dangling edges pointing to
    removed nodes. This mutability is required for watch mode.
  """

  verify unit "add and remove nodes from graph"
  verify unit "removing a node removes its edges"
  verify unit "graph consistency after batch mutations"

  tests ["../crates/specforge-graph/src/builder.rs"]
}

behavior compute_subgraph_for_invalidation "Compute Subgraph for Invalidation" {
  invariants [incremental_correctness]
  types      [Graph, Subgraph, FileEntry]

  contract """
    Given a changed file, the compiler MUST compute the invalidation
    subgraph: the changed file plus all files that transitively depend
    on it via use imports. Only nodes and edges from invalidated files
    MUST be removed and rebuilt. Unaffected subgraphs MUST remain intact.
  """

  verify unit        "changed file and direct dependents are invalidated"
  verify unit        "transitive dependents are included in subgraph"
  verify unit        "unaffected files are not invalidated"
  verify integration "subgraph rebuild matches full rebuild result"

  tests ["../crates/specforge-graph/src/builder.rs"]
}
