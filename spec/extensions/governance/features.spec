requirement FEATURE-044 "Governance Entity Kind Registration" {
  category "Feature"
  tier     v2
  text     """
    The @specforge/governance extension must register 3 entity kinds with
    full metadata, 4 edge types, field definitions, and validation rules.
    Without this registration, the compiler has zero knowledge of governance
    concepts: architectural decisions, non-functional constraints, and
    failure mode analysis.

    A comprehensive manifest declaration provides all entity kinds with
    testability flags (all false — governance entities are declarative
    records), LSP metadata (semantic tokens, icons), DOT shapes, typed
    field definitions with edge mappings, and declarative validation rules.
    Registration follows the zero-entity core protocol defined in ExtensionDeclaration.
  """
}

requirement FEATURE-045 "Governance Validation Suite" {
  category "Feature"
  tier     v2
  text     """
    Without domain-specific validation rules, the compiler cannot detect
    governance-level quality issues: incorrect RPN arithmetic, unmitigated
    high-risk invariants, or constraints protecting nothing.

    Declarative validation rules detect common governance specification
    quality issues: E005 validates RPN arithmetic integrity in failure_mode
    entities, W047 detects high-risk invariants without corresponding
    failure_mode analysis, W048 detects constraints with empty or invalid
    protects lists. Each rule uses the declarative pattern engine.
  """
}
