// @specforge/rust extension invariants

invariant entity_mapping_precedence "Entity Mapping Precedence" {
  guarantee """
    Test-to-entity resolution MUST follow strict precedence: tests field
    (1st) > proc macro attribute (2nd) > naming convention (3rd). A higher
    level MUST always override a lower level. No ambiguous mappings MUST
    exist after resolution — conflicts MUST produce diagnostics.
  """
  risk high

  verify property "tests field always overrides proc macro and convention"
  verify unit "ambiguous mappings produce diagnostics"

}
