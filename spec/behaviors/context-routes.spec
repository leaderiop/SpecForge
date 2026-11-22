requirement FEATURE-012 "Context Routes" {
  category "Feature"
  tier     v2
  text     """
    An agent gets its context from reads it assembles by hand, and the one budgeted read there is cuts entities by how
    connected they are: a small slice of a plan drops the plan's own first task while it keeps other phases.

    An extension declares context routes: for a subject kind, ordered sections of named selectors, each core or
    enrich, and the settings the role reads. The kernel serves one route for one entity as a pack: its core sections
    whole whatever they cost, then its enrich sections in declared order within the route's allowance or a cap the read
    states, with a digest anyone can recompute, on the CLI, the MCP tool and the context resource.
  """
}
