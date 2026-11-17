requirement INVARIANT-127 "Four Entity Kinds" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/software manifest MUST declare exactly 4 entity kinds:
    behavior, event, type, port. Features are product's and invariants
    governance's (ADR 0280).
  """
}

requirement INVARIANT-128 "Nine Edge Types" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/software manifest MUST declare exactly 9 edge types:
    References, Implements, Produces, Consumes, UsesType, UsesPort,
    Enforces, Imports, LinksTo. These edges model all relationships
    between the 6 entity kinds in the software engineering domain.
  """
}
