// @specforge/software extension invariants — guarantees on entity behavior

use "extensions/software/types"

invariant se_edge_consistency "Edge-Field Mapping Consistency" {
  guarantee """
    Every field definition with an edge mapping MUST have a corresponding
    edgeType declaration in the manifest. The field's targetKind MUST
    reference an entity kind declared in this manifest or a peer
    dependency. No orphan edge mappings MUST exist.
  """
  risk      medium
  verify property "every field edge mapping has a corresponding edgeType"
  verify unit "orphan edge mapping detected and reported"
}

invariant se_event_trigger_validity "Event Trigger Validity" {
  guarantee """
    An event's trigger field MUST reference a behavior entity. Events
    are caused by behaviors — they cannot trigger themselves or reference
    non-behavior entity kinds. Invalid trigger references MUST produce
    E051 error diagnostics.
  """
  risk      high
  verify unit "event trigger referencing behavior passes"
  verify unit "event trigger referencing non-behavior produces E051"
}

invariant se_port_direction_constraint "Port Direction Constraint" {
  guarantee """
    A port's direction field MUST be one of: inbound, outbound. No
    other values are permitted. Missing direction MUST produce an
    error diagnostic. This enforces the hexagonal architecture
    boundary model.
  """
  risk      low
  verify unit "port with direction inbound passes"
  verify unit "port with direction outbound passes"
  verify unit "port with invalid direction produces error"
}
