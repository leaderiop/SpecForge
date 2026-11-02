// Trace route behaviors — trace follows the routes extensions declare (ADR 0330)

use "invariants/core"
use "ports/inbound"

defaults behavior {
  features [trace_routes]
  ports    [CompilerApi]
}

feature trace_routes "Trace Routes" {
  status     proposed
  priority   high
  problem    """
    `specforge trace` walks every edge both ways from an entity. On a requirement it lists dozens of rows and says
    nothing of what the requirement's domain means by its thread: its plans, tasks, commits, tests and the verdict
    on what they prove.
  """
  solution   """
    An extension declares routes: from a start kind, ordered steps in the judged rules' selector grammar, each with
    annotations from fields, facts and judgements. The kernel's trace follows the route of the entity's kind and
    prints its thread on one line, or as a tree, on the CLI, MCP and the LSP; a kind with no route keeps the reach.
  """
  acceptance [
    "a requirement's trace prints its thread through its plan, task, commit, test and truth",
    "a route that cannot be followed is reported when the extension loads, not at the first trace",
    "a project whose extensions declare no route traces as before",
  ]
}

behavior declare_trace_route "Declare a Trace Route" {
  invariants [entity_id_uniqueness]
  category   validation
  contract   """
    An extension declares `routes` (protocol 1.15): each a kebab name, a start kind, ordered steps that go on from
    the start or an earlier step through selector steps, and annotations that print a field, a value reached
    through a selector, or a judgement (ADR 0330). A guest older than 1.15 is never asked for the category. The
    registry checks each route once: one that cannot be followed is E118 and is not registered, several from one
    kind without one default are W195, and a part of an optional peer the project does not load is skipped.
  """
  verify unit "a routes category round trips on the wire and a 1.14 guest is never asked for it"
  verify unit "an unknown key of a route, a step or an annotation is reported"
  verify unit "an SDK route builds the descriptor the host reads"
  verify unit "a route that cannot be followed is E118 and is not registered"
  verify unit "a route step of an optional peer the project does not load is skipped, not reported"
  verify unit "several routes from one kind without one default are W195, and the first in load order is followed"
}
