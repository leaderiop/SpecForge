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

behavior follow_trace_route "Follow a Trace Route" {
  invariants [graph_traversal_integrity, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  contract   """
    When an entity is traced, the system MUST follow the route its kind's extensions declare (ADR 0330): from the
    entity, each step in declared order, from each match of the step it goes on from, selects entities in id order,
    each a match under the match it came from. An annotation prints a field's values (lists flattened, a reference as
    its target's name), the values of a field reached through a selector, or a judgement's outcome and freshness, and
    its absent text when there is none. A kind with no route MUST be traced by the reach both ways, byte for byte as
    before. A named route traces its start kind's entities, and the reach every entity's chain; a route no loaded
    extension declares, or one of another kind, is invalid input about the route. A step or annotation the registry
    skipped for an unloaded optional peer is an I210 notice beside the trace.
  """
  verify unit "trace follows the route its kind declares, step by step, each match under the match it came from"
  verify unit "an annotation prints a field, a value reached through a selector, or a judgement's outcome and freshness"
  verify unit "a kind with no route is traced by the generic reach, unchanged"
  verify unit "a named route, the reach, and every entity are traced as asked, and a wrong route is refused"
  verify unit "a step skipped for an unloaded optional peer is an I210 notice"
  verify unit "a route step reaches collector-written facts through a scheme reference, and an unheld one is no match"
}

behavior render_trace_thread "Render a Trace Thread" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  contract   """
    A route trace MUST render its thread as one line when each step has at most one match: the start, then each
    step's match, its label, name and annotations, or `no <step>` when the step it goes on from has a match. With
    more matches it MUST render a tree, depth first, two spaces per step it goes on from. A match is kept when every
    step that goes on from its step has a kept match under it; the thread is complete when the start is kept. By
    default only a complete thread prints, over its kept matches; an incomplete one prints one line naming what it
    lacks, and `--partial` prints every match. The summary names each step's one match or counts its matches.
  """
  verify unit "a thread with at most one match per step prints on one line, a step with none as no <step>"
  verify unit "several matches of a step print as branches of a tree"
  verify unit "only a complete thread prints by default, its dead-end branches left out, and a partial one with --partial"
  verify unit "the summary counts a step's matches"
  verify unit "specforge trace prints a route trace's thread, and --reach the generic chain"
  verify unit "specforge trace prints a route's thread as human text unless --format json is given, and any other trace as json"
}

behavior show_trace_thread "Show a Trace Thread" {
  invariants [lsp_response_latency]
  category   query
  contract   """
    A code lens at the name of an entity of a kind a route is followed from MUST say its thread's summary, and the
    hover on it MUST show the route's name and the thread over every match. One selection serves all of a file's
    lenses. An entity of a kind no route starts at has neither (ADR 0330).
  """
  verify unit "a code lens on a routed entity says its thread's summary"
  verify unit "an entity of a kind no route starts at has no thread lens"
  verify unit "the hover on a routed entity shows its thread"
}
