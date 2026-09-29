// @specforge/rust extension invariants

invariant entity_mapping_precedence "Entity Mapping Precedence" {
  guarantee """
    Test-to-entity resolution MUST follow strict precedence: the
    #[specforge_test] attribute (1st) > naming convention (2nd). The
    explicit attribute MUST always override the convention. No ambiguous
    mappings MUST exist after resolution — conflicts MUST produce
    diagnostics. Spec files carry no test paths (ADR 0002).
  """
  risk high

  verify property "the proc macro attribute always overrides the naming convention"
  verify unit "ambiguous mappings produce diagnostics"
}
