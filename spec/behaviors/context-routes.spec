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
  verify unit "a section digests as the judged input of the same selector does"
  verify unit "a files section lists paths and digests and a settings section the values of its settings"
  verify unit "the pack digest covers the sections served and recomputes from the pack"
}

behavior declare_context_route "Declare a Context Route" {
  invariants [entity_id_uniqueness]
  category   validation
  contract   """
    An extension MAY declare `contexts` (protocol 2.1): each a kebab name, a subject kind, an optional agent of its
    own, a default budget, ordered sections of named selectors with a tier, and setting keys it carries (ADR 0356). A
    guest older than 2.1 is never asked for the category. The registry checks each route once: one that cannot be
    served is E123 and is not registered, an agent binding that cannot hold is E124, and a section of an optional peer
    the project does not load is skipped.
  """
  verify unit "a contexts category round trips on the wire and a 2.0 guest is never asked for it"
  verify unit "an unknown key of a context route or a section is reported"
  verify unit "an SDK context route builds the descriptor the host reads"
  verify unit "a context route that cannot be served is E123 and is not registered"
  verify unit "a context section of an optional peer the project does not load is skipped and listed"
  verify unit "a context route's agent its extension does not declare, or another route binds, is E124 and the route stays"
}
