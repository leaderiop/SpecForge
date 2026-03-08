// Graph building behaviors — constructing the in-memory graph

use invariants/core
use types/core
use types/graph
use events/compilation

behavior build_in_memory_graph "Build In-Memory Graph" {
  invariants [string_interning_consistency, entity_id_uniqueness]
  types      [Graph, Node, Edge, SpecFile, EdgeType, FileIndex]
  consumes  [resolution_complete]
  produces   [graph_built]

  requires {
    resolution_complete "resolution_complete event has fired, confirming all use imports are resolved and entity references are linked"
  }

  ensures {
    one_node_per_entity "Graph contains exactly one node per declared entity"
    one_edge_per_reference "Graph contains one edge per resolved reference"
    no_orphan_edges "No orphan edges exist (every edge connects two existing nodes)"
  }

  contract """
    After resolution, the compiler MUST construct an in-memory directed
    graph where each entity becomes a node and each resolved reference
    becomes a typed edge. The graph builder materializes the pending
    edges recorded by link_entity_references. The graph MUST contain
    exactly one node per declared entity and one edge per resolved
    reference.
  """

  verify unit "graph contains one node per entity"
  verify unit "graph contains one edge per resolved reference"
  verify unit "edge types match relationship semantics"
  verify contract "requires/ensures consistency for in-memory graph construction"

}

// No produces — passive API behavior, graph mutations are observed via rebuild events
behavior maintain_mutable_graph "Maintain Mutable Graph" {
  invariants [incremental_correctness, graph_traversal_integrity]
  types      [Graph, Subgraph]

  requires {
    graph_initialized "An in-memory graph instance exists and is accessible for mutation"
  }

  ensures {
    mutations_applied "Node and edge additions and removals are reflected in the graph"
    no_dangling_edges_enforced "After any mutation, no edges point to removed nodes"
  }

  maintains {
    graph_consistency "Graph remains structurally consistent across all incremental mutations"
  }

  contract """
    The in-memory graph MUST support incremental mutation: adding nodes,
    removing nodes, adding edges, and removing edges. After mutation,
    the graph MUST remain consistent — no dangling edges pointing to
    removed nodes. This mutability is required for watch mode.
  """

  verify unit "add and remove nodes from graph"
  verify unit "removing a node removes its edges"
  verify unit "graph consistency after batch mutations"
  verify contract "requires/ensures consistency for mutable graph maintenance"

}

behavior compute_subgraph_for_invalidation "Compute Subgraph for Invalidation" {
  invariants [incremental_correctness, graph_traversal_integrity]
  types      [Graph, Subgraph, FileEntry]

  requires {
    graph_built_ready "A fully constructed in-memory graph exists from a prior build"
    changed_file_identified "The changed file path has been identified by the file watcher"
  }

  ensures {
    invalidation_subgraph_computed "The subgraph containing the changed file and all transitive dependents is identified"
    only_affected_rebuilt "Only nodes and edges from invalidated files are removed and rebuilt"
  }

  maintains {
    unaffected_subgraphs_intact "Subgraphs not reachable from the changed file remain unchanged"
  }

  contract """
    Given a changed file, the compiler MUST compute the invalidation
    subgraph: the changed file plus all files that transitively depend
    on it via use imports. Only nodes and edges from invalidated files
    MUST be removed and rebuilt. Unaffected subgraphs MUST remain intact.
    Called by invalidate_changed_files during incremental rebuilds.
  """

  verify unit        "changed file and direct dependents are invalidated"
  verify unit        "transitive dependents are included in subgraph"
  verify unit        "unaffected files are not invalidated"
  verify integration "subgraph rebuild matches full rebuild result"
  verify contract "requires/ensures consistency for subgraph invalidation"

}
