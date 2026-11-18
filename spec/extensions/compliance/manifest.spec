requirement INVARIANT-003 "Four Entity Kinds" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/compliance manifest MUST declare exactly 4 entity kinds:
    regulation, control, evidence, audit. These represent the minimal
    complete set for regulatory compliance specification.
  """
}

requirement INVARIANT-002 "Four Edge Types" {
  category "Invariant"
  tier     v2
  text     """
    The @specforge/compliance manifest MUST declare exactly 4 edge types:
    Governs (regulation -> control), ImplementedBy (control -> evidence),
    ProvidedBy (evidence -> audit), Audits (audit -> regulation). These
    edges model the compliance traceability chain.
  """
}
