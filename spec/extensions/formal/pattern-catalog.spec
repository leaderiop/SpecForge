// @specforge/formal pattern catalog — concrete trigger patterns for diagnostics
//
// Documents the specific patterns that trigger E031 and E034.
// These are the implementable detection algorithms, not aspirational goals.

use "extensions/formal/types"

behavior fa_pattern_e031_set_inclusion "E031 Pattern: Condition Set Inclusion" {
  category query
  types    [EnsuresBlock, ConditionEntry, RefinementChain]
  contract """
    E031 fires when a refined behavior's ensures conditions do not
    include all condition names from the abstract behavior's ensures
    block. This is a named-condition set inclusion check, not logical
    entailment.

    Pattern: For each abstract behavior A with ensures conditions
    {c1, c2, c3}, every concrete behavior B that refines A MUST have
    ensures conditions that include {c1, c2, c3} (by condition name).
    B MAY add additional ensures conditions (strengthening postconditions
    is permitted). B MUST NOT remove any of A's ensures conditions
    (weakening postconditions violates the layering guarantee).

    Detection algorithm: set difference (abstract.ensures.names -
    concrete.ensures.names). Non-empty difference triggers E031.
  """
  ensures {
    superset_passes    "concrete ensures that is superset of abstract ensures passes"
    subset_error       "concrete ensures missing abstract condition names produces E031"
    exact_match_passes "concrete ensures with exactly same names as abstract passes"
    additional_ok      "concrete ensures with extra conditions beyond abstract passes"
    missing_documented "E031 message lists the missing condition names"
  }
  features [fa_specification_layering]
  verify unit "concrete ensures superset of abstract ensures passes"
  verify unit "concrete ensures missing condition names produces E031"
  verify unit "concrete ensures with additional conditions passes"
  verify unit "E031 message lists missing condition names"
}

behavior fa_pattern_e034_unmitigated_cycle "E034 Pattern: Unmitigated Cycle" {
  category query
  types    [SyncBlock, EventGraphAnalysisReport]
  contract """
    E034 fires when the event flow graph contains a strongly connected
    component (SCC) through two or more behaviors with no mitigation.
    The detection algorithm:

    1. Build the graph: an edge from each behavior to each event it
       produces, and from each event to each behavior that consumes it.
    2. Run Tarjan's SCC algorithm on it.
    3. For each SCC holding two or more behaviors, look for a
       mitigation: a non-empty sync on any behavior or event in it.
    4. If NO mitigation is found: emit E034 with a cycle path, found by
       a breadth-first search from the SCC's first behavior back to
       itself, and suggest declaring sync on one of its members.
    5. If ANY mitigation is found: the cycle passes silently.

    A declared sync stands for a timeout, barrier or delivery bound;
    what it says is not checked. The data model has no idempotency or
    circuit-breaker marker, so neither mitigates a cycle.
  """
  ensures {
    unmitigated_error "SCC through two or more behaviors with no sync produces E034"
    sync_mitigated    "SCC with a non-empty sync on any behavior or event passes"
    path_documented   "E034 includes a cycle path (node1 -> node2 -> ... -> node1)"
    mitigation_named  "E034 suggests declaring sync on one of the cycle's members"
  }
  features [fa_event_graph_linting]
  verify unit "SCC with no mitigations produces E034"
  verify unit "SCC with a sync on any member passes"
  verify unit "E034 lists full cycle path"
  verify unit "E034 suggests applicable mitigations"
}
