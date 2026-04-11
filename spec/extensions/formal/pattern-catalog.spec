// @specforge/formal pattern catalog — concrete trigger patterns for diagnostics
//
// Documents the specific patterns that trigger E030, E031, and E034.
// These are the implementable detection algorithms, not aspirational goals.

use "extensions/formal/types"

behavior fa_pattern_e030_contradiction "E030 Pattern: Contradictory Precondition" {
  category query
  types    [RequiresBlock, ConditionEntry]
  contract """
    E030 fires when a requires block contains structurally contradictory
    conditions. Three concrete patterns are detected:

    1. X/not_X: Two conditions where one name is the negation of the
       other (e.g., "data_valid" and "data_not_valid", or "connected"
       and "not_connected"). Detection: name prefix/suffix matching
       with "not_" and "_not_" variants.

    2. Tautological false: A condition whose description contains
       explicit impossibility language (e.g., "always false", "never
       true", "impossible"). Detection: keyword matching against a
       closed list of impossibility phrases.

    3. Empty domain intersection: Two conditions that reference the
       same entity with mutually exclusive constraints (e.g.,
       "count > 0" and "count == 0"). Detection: same-entity
       reference with opposing comparison operators.
  """
  ensures  {
    x_not_x_detected       "requires with X and not_X condition names produces E030"
    tautological_detected  "requires with explicitly impossible condition produces E030"
    empty_domain_detected  "requires with mutually exclusive constraints on same entity produces E030"
    normal_passes          "requires with non-contradictory conditions passes"
    pattern_documented     "E030 diagnostic message includes which pattern triggered"
  }

  features [fa_structured_conditions]

  verify unit "X/not_X condition pair produces E030"
  verify unit "tautological false condition produces E030"
  verify unit "empty domain intersection produces E030"
  verify unit "non-contradictory conditions pass"
  verify unit "E030 message identifies trigger pattern"
}

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
  ensures  {
    superset_passes        "concrete ensures that is superset of abstract ensures passes"
    subset_error           "concrete ensures missing abstract condition names produces E031"
    exact_match_passes     "concrete ensures with exactly same names as abstract passes"
    additional_ok          "concrete ensures with extra conditions beyond abstract passes"
    missing_documented     "E031 message lists the missing condition names"
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
    E034 fires when the event-behavior bipartite graph contains a
    strongly connected component (SCC) with no mitigations. The
    detection algorithm:

    1. Run Tarjan's SCC algorithm on the bipartite graph.
    2. For each SCC with >1 node, check for mitigations:
       a. sync.timeout on any event in the cycle
       b. @idempotent annotation on any behavior in the cycle
       c. circuit_breaker pattern (behavior with retry_limit field)
    3. If NO mitigations found: emit E034 with full cycle path.
    4. If ANY mitigation found: cycle passes silently.

    This eliminates false positives on normal feedback loops that have
    appropriate timeout/idempotency safeguards.
  """
  ensures  {
    unmitigated_error      "SCC with no mitigations produces E034"
    timeout_mitigated      "SCC with sync.timeout on any event passes"
    idempotent_mitigated   "SCC with @idempotent on any behavior passes"
    breaker_mitigated      "SCC with circuit_breaker pattern passes"
    path_documented        "E034 includes full cycle path (node1 -> node2 -> ... -> node1)"
    mitigations_listed     "E034 lists which mitigations would resolve the cycle"
  }

  features [fa_event_graph_linting]

  verify unit "SCC with no mitigations produces E034"
  verify unit "SCC with sync.timeout passes"
  verify unit "SCC with @idempotent passes"
  verify unit "SCC with circuit_breaker passes"
  verify unit "E034 lists full cycle path"
  verify unit "E034 suggests applicable mitigations"
}
