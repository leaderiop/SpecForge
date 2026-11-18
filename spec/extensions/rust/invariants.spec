requirement INVARIANT-027 "Entity Mapping Precedence" {
  category "Invariant"
  tier     v2
  text     """
    Test-to-entity resolution MUST follow strict precedence: the
    #[specforge_test] attribute (1st) > naming convention (2nd). The
    explicit attribute MUST always override the convention. No ambiguous
    mappings MUST exist after resolution — conflicts MUST produce
    diagnostics. Spec files carry no test paths (ADR 0002).
  """
}
