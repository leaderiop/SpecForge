decision D-023 "Gherkin Bridge for Traceability" {
  settled true
  origin  project
  text    """
    The @specforge/software extension declares a gherkin field with type
    string_list and file_reference=true on behavior entities. This field
    references external .feature files. Feature files use
    `@specforge:entity_id` tags to bind to spec entities. `verify` remains
    as a core grammar construct for unit/property/contract test intent
    declarations. `specforge collect cucumber` command consumes Cucumber
    JSON reports for traceability proof. Two traceability paths: gherkin
    field for behavior entities, convention-based matching for
    invariant/event/constraint entities.
  """
}
