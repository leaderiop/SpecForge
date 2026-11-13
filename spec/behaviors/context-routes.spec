// Context route behaviors — an extension declares the context a role reads (ADR 0356)

use "invariants/core"
use "ports/inbound"

defaults behavior {
  features [context_routes]
  ports    [CompilerApi]
}

feature context_routes "Context Routes" {
  status     proposed
  priority   high
  problem    """
    An agent gets its context from reads it assembles by hand, and the one budgeted read there is cuts entities by how
    connected they are: a small slice of a plan drops the plan's own first task while it keeps other phases.
  """
  solution   """
    An extension declares context routes: for a subject kind, ordered sections of named selectors, each core or
    enrich, and the settings the role reads. The kernel serves one route for one entity as a pack: its sections in
    declared order under a token budget, core never cut, with a digest anyone can recompute, on the CLI, the MCP tool
    and the context resource.
  """
  acceptance [
    "a budgeted read of a plan for its executor keeps every task of the plan and its phase's context",
    "a route that cannot be served is reported when the extension loads",
    "a project whose extensions declare no context route compiles, exports and installs as before",
  ]
}

behavior serve_context_pack "Serve a Context Pack" {
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  contract   """
    An extension's context route MUST be served for one entity of its subject kind as its sections in declared order,
    each the entities, files, tests or settings its selector or keys pick, with a digest equal to a judged input's for
    the same selector. Under a budget the subject and every core section MUST be kept, enrich sections served whole in
    declared order and, inside the first that does not fit, its most connected entities that fit; what was cut MUST
    be listed. The pack's digest MUST cover exactly what was served and MUST be reproducible from the pack. The CLI,
    the MCP tool and the resource read with a role MUST answer the same bytes (ADR 0356).
  """
  verify unit "a budgeted context read of a plan for its executor keeps every core member and cuts only enrich sections"
}
