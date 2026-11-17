requirement INVARIANT-042 "Federated Graph Determinism" {
  category "Invariant"
  tier     v2
  text     """
    The order in which remote project graphs are loaded and merged MUST
    NOT affect the resulting federated graph. Given the same set of remote
    graphs with the same content, the merged FederatedGraph MUST be
    identical regardless of load order or merge sequence.
  """
}

requirement INVARIANT-114 "Project-Qualified ID Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    Within a federated graph, every project-qualified entity ID
    (project::entity_id) MUST be unique. Two entities from different
    projects with the same raw entity_id MUST be disambiguated by their
    project prefix. A collision where two projects declare the same
    project prefix MUST be detected and reported as an error.
  """
}

requirement INVARIANT-010 "Cross-Project Reference Safety" {
  category "Invariant"
  tier     v2
  text     """
    Cross-project reference graphs MUST NOT contain cycles across project
    boundaries. A local project graph MUST be independently valid without
    any remote project dependencies — remote projects provide additional
    context but are never required for local validation. Missing remote
    projects MUST degrade gracefully with an info diagnostic, never
    an error.
  """
}

requirement INVARIANT-043 "Federated Graph Traversal Integrity" {
  category "Invariant"
  tier     v2
  text     """
    Federated graph traversal operations (cross-project trace, federated
    subgraph extraction, federated export) MUST produce complete and
    deterministic results. Every reachable node across project boundaries
    along a traversal path MUST be included. Traversal order MUST be
    deterministic for identical federated graph inputs.
  """
}

requirement INVARIANT-044 "Federated Schema Completeness" {
  category "Invariant"
  tier     v2
  text     """
    The schema section of any federated Graph Protocol export MUST include
    every entity kind and edge type from all merged project graphs. No
    registered kind or edge type from any participating project MUST be
    omitted. The schema MUST accurately reflect the union of all project
    schemas.
  """
}
