// Federation events — signals emitted during cross-project operations

use "extensions/federation/types"
use "types/config"
event federation_config_loaded "Federation Config Loaded" {
  channel   "federation.federation_config_loaded"

  payload {
    dependencyCount  integer
    projectNames     string[]
    timestamp        timestamp
  }


  verify integration "emits federation_config_loaded with correct dependencyCount and projectNames"

}

event remote_graph_loaded "Remote Graph Loaded" {
  channel   "federation.remote_graph_loaded"

  payload {
    projectName     string
    graphNodeCount  integer
    schemaVersion   string
    timestamp       timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to remote graph loading (e.g., caching, analytics)

  verify integration "emits remote_graph_loaded with correct projectName and graphNodeCount"

}

event cross_project_refs_resolved "Cross-Project Refs Resolved" {
  channel   "federation.cross_project_refs_resolved"

  payload {
    resolvedCount   integer
    unresolvedCount integer
    timestamp       timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to reference resolution (e.g., reporting, dashboards)

  verify integration "emits cross_project_refs_resolved with correct resolvedCount and unresolvedCount"

}

event cross_project_edges_validated "Cross-Project Edges Validated" {
  channel   "federation.cross_project_edges_validated"

  payload {
    compatibleCount     integer
    incompatibleCount   integer
    normalizedCount     integer
    timestamp           timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to edge validation (e.g., compatibility reporting)

  verify integration "emits cross_project_edges_validated with correct compatibleCount and incompatibleCount"

}

event federated_graph_exported "Federated Graph Exported" {
  channel   "federation.federated_graph_exported"

  payload {
    totalNodes      integer
    totalEdges      integer
    projectCount    integer
    timestamp       timestamp
  }

  // consumers are extension points — federation consumers will be registered
  // by extensions that react to federated exports (e.g., CI hooks, notifications)

  verify integration "emits federated_graph_exported with correct totalNodes, totalEdges, and projectCount"

}
