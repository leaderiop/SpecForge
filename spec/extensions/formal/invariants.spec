// @specforge/formal extension invariants — guarantees on formal analysis behavior
//
// Moved from @specforge/software per 10-expert panel recommendation.

use "extensions/formal/types"

invariant fa_layering_dag "Specification Layering DAG" {
  guarantee   """
    The refines edges between behaviors MUST form a directed acyclic
    graph (DAG). Cycles in refinement chains MUST produce E032 error
    diagnostics. This ensures well-founded layering from abstract
    specifications to concrete implementations.
  """
  risk        high

  verify property "refines edges form a DAG with no cycles"
  verify unit "cycle in refinement chain produces E032"
}

invariant fa_condition_consistency "Structured Condition Consistency" {
  guarantee   """
    In an ensures block, condition descriptions MUST NOT reference
    identifiers that are absent from the corresponding requires block
    or the entity's own scope (types, ports, invariants fields). This
    prevents postconditions from depending on undefined state. Applies
    to both inline conditions and referenced condition entities.
  """
  risk        medium

  verify unit "ensures referencing unknown identifier detected"
  verify unit "ensures referencing requires identifier passes"
  verify unit "referenced condition entity included in scope check"
}

invariant fa_condition_entity_reachability "Condition Entity Reachability" {
  guarantee   """
    Every condition entity MUST be referenced by at least one
    RequiresCondition, EnsuresCondition, or MaintainsCondition edge.
    Unreferenced condition entities produce W059 warning. This prevents
    orphaned conditions that exist in the graph but are never used.
  """
  risk        low

  verify unit "condition with RequiresCondition edge passes"
  verify unit "condition with no incoming edges produces W059"
}

invariant fa_property_entity_reachability "Property Entity Reachability" {
  guarantee   """
    Every property entity MUST be referenced by at least one Satisfies
    edge from a behavior. Unreferenced property entities produce W061
    warning. This prevents orphaned temporal assertions that exist in
    the graph but no behavior claims to satisfy.
  """
  risk        low

  verify unit "property with Satisfies edge passes"
  verify unit "property with no incoming edges produces W061"
}

invariant fa_axiom_entity_reachability "Axiom Entity Reachability" {
  guarantee   """
    Every axiom entity MUST be referenced by at least one AssumedBy
    edge from a condition. Unreferenced axiom entities produce W064
    warning. This prevents orphaned assumptions that no condition
    depends on.
  """
  risk        low

  verify unit "axiom with AssumedBy edge passes"
  verify unit "axiom with no incoming edges produces W064"
}

invariant fa_protocol_entity_reachability "Protocol Entity Reachability" {
  guarantee   """
    Every protocol entity MUST be referenced by at least one
    FollowsProtocol edge from an event. Unreferenced protocol entities
    produce W066 warning. This prevents orphaned sync contracts that
    no event follows.
  """
  risk        low

  verify unit "protocol with FollowsProtocol edge passes"
  verify unit "protocol with no incoming edges produces W066"
}

invariant fa_refinement_entity_reachability "Refinement Entity Reachability" {
  guarantee """
    Every refinement entity MUST be referenced by at least one RefinesTo
    edge targeting a behavior, or be linked via RefinementChainLink from
    another refinement. Unreferenced refinement entities produce W069.
  """
  risk low

  verify unit "refinement with RefinesTo edge passes"
  verify unit "refinement with only RefinementChainLink incoming passes"
  verify unit "refinement with no edges produces W069"
}

invariant fa_refinement_chain_dag "Refinement Chain Link DAG" {
  guarantee """
    RefinementChainLink edges between refinement entities MUST form a DAG.
    Cycles produce E041 error.
  """
  risk high

  verify property "RefinementChainLink edges form a DAG"
  verify unit "cycle in RefinementChainLink produces E041"
}

invariant fa_process_entity_reachability "Process Entity Reachability" {
  guarantee """
    Every process entity MUST be referenced by at least one ParticipatesIn
    edge from an event. Unreferenced process entities produce W072.
  """
  risk low

  verify unit "process with ParticipatesIn edge passes"
  verify unit "process with no incoming edges produces W072"
}

invariant fa_process_composition_dag "Process Composition DAG" {
  guarantee """
    ProcessComposition edges between process entities MUST form a DAG.
    Cycles produce E042 error.
  """
  risk high

  verify property "ProcessComposition edges form a DAG"
  verify unit "cycle in ProcessComposition produces E042"
}
