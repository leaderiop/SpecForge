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
  verify unit "core sections are never cut, and a budget below them is E062 naming the route"
  verify unit "enrich sections fill the budget whole in declared order, and inside the first that does not fit its most central entities are kept"
  verify unit "a route no loaded extension declares, or a subject of another kind, is invalid input"
  verify unit "specforge context prints a route's pack and --pick reads its digest"
  verify unit "specforge.context answers the pack specforge context prints"
  verify unit "specforge://context/{+entity_id}?role= answers the role's pack, and depth or kinds beside role are refused naming the key"
}

behavior declare_context_route "Declare a Context Route" {
  invariants [entity_id_uniqueness]
  category   validation
  contract   """
    An extension MAY declare `contexts` (protocol 2.1): each a kebab name, a subject kind, an optional agent of its
    own, a default budget, ordered sections of named selectors with a tier, and setting keys it carries (ADR 0356). A
    guest older than 2.1 is never asked for the category. The registry checks each route once: one that cannot be
    served is E123 and is not registered, an agent binding that cannot hold is E124, and a section of an optional peer
    the project does not load is skipped. A route MAY record its pack in context-written fields of its own extension,
    each with an entity chain from the recording kind to its subject, and MUST then name a code its extension
    catalogues that a stale record raises; a record that cannot hold is E123.
  """
  verify unit "a contexts category round trips on the wire and a 2.0 guest is never asked for it"
  verify unit "an unknown key of a context route or a section is reported"
  verify unit "a route's records and stale code round trip on the wire, and an unknown record key is reported"
  verify unit "an SDK route records context fields and names its stale code"
  verify unit "a context record that cannot be held makes its route E123, and the route is not registered"
  verify unit "a context route records context fields of its own extension, each with a subject selector to its subject kind, and names a code its extension catalogues"
  verify unit "an SDK context route builds the descriptor the host reads"
  verify unit "a context route that cannot be served is E123 and is not registered"
  verify unit "a file section follows its referrers on to their file field, and one that ends at the reference is E123"
  verify unit "a context section of an optional peer the project does not load is skipped and listed"
  verify unit "a context route's agent its extension does not declare, or another route binds, is E124 and the route stays"
}

behavior render_context_section "Render a Context Section" {
  invariants [zero_domain_knowledge_core]
  category   query
  contract   """
    An installed agent that a context route names MUST carry, after its body and the generated tools section, a Context
    section naming the route, its subject kind, its sections in order (core marked), its budget and the CLI line, the
    MCP tool and the resource that read the pack; an agent no route names MUST carry none (ADR 0356).
  """
  verify unit "an installed agent a context route names gets a Context section naming the route, its sections and how to read it"
}

behavior record_context_pack "Record a Context Pack" {
  invariants [zero_domain_knowledge_core, diagnostic_determinism]
  category   command
  contract   """
    A context route's extension, or a person or agent at a core surface, MAY ask the host to record the route's pack
    on an entity of a kind the route records. The host MUST derive the subject from the entity by the route's declared
    selector, compute the pack over the view the write applies to at the given budget or the route's, and write the
    route, the pack digest and the budget with its ledger line; it MUST refuse an unknown route, a kind the route does
    not record, another extension's command, no or several subjects, and a budget below the pack's core (ADR 0356).
  """
  verify unit "a record_context op round trips, its budget optional"
  verify unit "a record_context op names the entity, the route and the budget, and no revision"
  verify unit "a recorded pack holds the route, the digest the pack read gives and its budget, with its ledger line"
  verify unit "a record of an unknown route, a kind it does not record, from another extension or with no subject is refused record_refused"
  verify unit "recording a pack stales no verdict"
  verify unit "specforge context --record writes the digest it prints"
}

behavior report_stale_context_record "Report a Stale Context Record" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  category   validation
  contract   """
    On each check the host MUST re-evaluate, at its recorded budget, the pack of every record a route with a stale code
    holds, when the field's owner gate admits it, and report a record whose pack has another digest, no or several
    subjects, or a core above its budget with the route's stale code at the record. A project holding no record MUST
    evaluate no pack (ADR 0356).
  """
  verify unit "a record whose pack has moved raises its route's stale code at the record"
  verify unit "a record is checked only when its field's owner gate admits it"
  verify unit "a record with no subject, or whose core no longer fits its budget, is stale"
  verify unit "a check evaluates one pack per held record and none for a project without one"
}
