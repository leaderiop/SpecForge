requirement INVARIANT-126 "Edge-Field Mapping Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Every field definition with an edge mapping MUST have a corresponding
    edgeType declaration in the manifest. The field's targetKind MUST
    reference an entity kind declared in this manifest or a peer
    dependency. No dangling edge mappings MUST exist.
  """
}

requirement INVARIANT-129 "Port Direction Constraint" {
  category "Invariant"
  tier     v2
  text     """
    A port's direction field MUST be one of: inbound, outbound. No
    other values are permitted. Missing direction MUST produce an
    error diagnostic. This enforces the hexagonal architecture
    boundary model.
  """
}
