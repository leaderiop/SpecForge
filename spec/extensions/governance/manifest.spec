requirement INVARIANT-051 "Four Entity Kinds" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/governance manifest MUST declare exactly 4 entity kinds:
    decision, constraint, failure_mode, invariant (ADR 0280).
  """
}

requirement INVARIANT-050 "Four Edge Types" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/governance manifest MUST declare exactly 4 edge types:
    DecisionInvariant (decision->invariant), ConstrainsBehavior
    (constraint->behavior, cross-extension), ProtectsInvariant
    (constraint->invariant), FailureModeInvariant (failure_mode->invariant).
  """
}
