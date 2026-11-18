requirement INVARIANT-034 "Specification Layering DAG" {
  category "Invariant"
  tier     v2
  text     """
    The refines edges between behaviors MUST form a directed acyclic
    graph (DAG). Cycles in refinement chains MUST produce E041 error
    diagnostics. This ensures well-founded layering from abstract
    specifications to concrete implementations.
  """
}

requirement INVARIANT-033 "Structured Condition Consistency" {
  category "Invariant"
  tier     v2
  text     """
    In an ensures block, condition descriptions MUST NOT reference
    identifiers that are absent from the corresponding requires block
    or the entity's own scope (types, ports, invariants fields). This
    prevents postconditions from depending on undefined state.
  """
}

requirement INVARIANT-037 "Property Entity Reachability" {
  category "Invariant"
  tier     v2
  text     """
    Every property entity MUST be referenced by at least one Satisfies
    edge from a behavior. Unreferenced property entities produce W123
    warning. This prevents unreferenced temporal assertions that exist in
    the graph but no behavior claims to satisfy.
  """
}

requirement INVARIANT-032 "Axiom Entity Reachability" {
  category "Invariant"
  tier     v2
  text     """
    Every axiom entity MUST be referenced by at least one AssumedBy
    edge from a condition. Unreferenced axiom entities produce W126
    warning. This prevents unreferenced assumptions that no condition
    depends on.
  """
}

requirement INVARIANT-038 "Protocol Entity Reachability" {
  category "Invariant"
  tier     v2
  text     """
    Every protocol entity MUST be referenced by at least one
    FollowsProtocol edge from an event. Unreferenced protocol entities
    produce W128 warning. This prevents unreferenced sync contracts that
    no event follows.
  """
}

requirement INVARIANT-040 "Refinement Entity Reachability" {
  category "Invariant"
  tier     v2
  text     """
    Every refinement entity MUST reach behaviors through its
    RefinementRefinesAbstract/RefinementRefinesConcrete edges, or be linked
    via RefinementChainsToRefinement from another refinement. Unreferenced refinement entities produce W131.
  """
}

requirement INVARIANT-039 "Refinement Chain Link DAG" {
  category "Invariant"
  tier     v2
  text     """
    RefinementChainsToRefinement edges between refinement entities MUST form a DAG.
    Cycles produce E041 error.
  """
}

requirement INVARIANT-036 "Process Entity Reachability" {
  category "Invariant"
  tier     v2
  text     """
    Every process entity MUST be referenced by at least one ParticipatesIn
    edge from an event. Unreferenced process entities produce W134.
  """
}

requirement INVARIANT-035 "Process Composition DAG" {
  category "Invariant"
  tier     v2
  text     """
    ProcessComposition edges between process entities MUST form a DAG.
    Cycles produce E042 error.
  """
}
