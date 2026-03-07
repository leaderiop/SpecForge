// Federation events — signals emitted during cross-project operations

use extensions/federation/types
use types/config
use extensions/federation/behaviors

event federation_config_loaded "Federation Config Loaded" {
  trigger   load_federation_config
  channel   "federation.federation_config_loaded"

  payload {
    dependencyCount  integer
    projectNames     string[]
    timestamp        timestamp
  }

  consumers []

  verify integration "emits federation_config_loaded with correct dependencyCount and projectNames"

}

event remote_graph_loaded "Remote Graph Loaded" {
  trigger   load_remote_project_graph
  channel   "federation.remote_graph_loaded"

  payload {
    projectName     string
    graphNodeCount  integer
    schemaVersion   string
    timestamp       timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to remote graph loading (e.g., caching, analytics)
  consumers []

  verify integration "emits remote_graph_loaded with correct projectName and graphNodeCount"

}

event cross_project_refs_resolved "Cross-Project Refs Resolved" {
  trigger   resolve_cross_project_references
  channel   "federation.cross_project_refs_resolved"

  payload {
    resolvedCount   integer
    unresolvedCount integer
    timestamp       timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to reference resolution (e.g., reporting, dashboards)
  consumers []

  verify integration "emits cross_project_refs_resolved with correct resolvedCount and unresolvedCount"

}

event cross_project_edges_validated "Cross-Project Edges Validated" {
  trigger   validate_cross_project_edge_consistency
  channel   "federation.cross_project_edges_validated"

  payload {
    compatibleCount     integer
    incompatibleCount   integer
    normalizedCount     integer
    timestamp           timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to edge validation (e.g., compatibility reporting)
  consumers []

  verify integration "emits cross_project_edges_validated with correct compatibleCount and incompatibleCount"

}

event federated_graph_exported "Federated Graph Exported" {
  trigger   export_federated_graph
  channel   "federation.federated_graph_exported"

  payload {
    totalNodes      integer
    totalEdges      integer
    projectCount    integer
    timestamp       timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to federated exports (e.g., CI hooks, notifications)
  consumers []

  verify integration "emits federated_graph_exported with correct totalNodes, totalEdges, and projectCount"

}
