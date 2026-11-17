requirement FEATURE-118 "Gherkin Bridge" {
  category "Feature"
  tier     v2
  text     """
    Behavior entities need a way to reference Gherkin .feature files for
    BDD traceability. This is a domain-specific concern — not all spec
    domains use Cucumber/BDD — so it must be an extension responsibility,
    not a core grammar construct.

    The @specforge/software extension declares a gherkin field with type
    string_list on behavior entities via the FieldRegistry. The field is
    parsed as a standard StringList value — no dedicated grammar rule or
    AST type is needed. A project checks that each file exists by
    attaching `@specforge/checks:file` to it (E016).
  """
}

requirement FEATURE-117 "Core Entity Kind Registration" {
  category "Feature"
  tier     v2
  text     """
    The @specforge/software extension must register 6 entity kinds with
    full metadata, 9 edge types, field definitions, validation rules,
    verify kinds, and LSP metadata. Without this registration, the
    compiler has zero knowledge of software engineering domain concepts.

    A comprehensive manifest declaration provides all entity kinds with
    testability flags, verify kind allowlists, LSP metadata (semantic
    tokens, icons), DOT shapes, typed field definitions with edge
    mappings, and declarative validation rules. Registration follows
    the zero-entity core protocol defined in ExtensionDeclaration.
  """
}

requirement FEATURE-119 "Software Validation Suite" {
  category "Feature"
  tier     v2
  text     """
    Without domain-specific validation rules, the compiler can only
    perform structural checks. Unreferenced entities, unverified testables,
    invalid trigger references, and type annotation errors go undetected.
    Users receive no warnings about specification quality issues.

    Declarative validation rules (W001-W005, W007-W010)
    detect common specification quality issues: unreferenced entities without
    incoming edges, testable entities without verify statements,
    features with empty behavior lists, unknown field
    annotations, unenforced invariants, and invalid verify kinds for entity
    types. Each rule uses the declarative pattern engine.
  """
}
