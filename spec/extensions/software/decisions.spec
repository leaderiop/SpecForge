// @specforge/software extension Architecture Decision Records
//
// Decisions specific to the software engineering entity model
// and its traceability mechanisms.

use invariants/validation

decision gherkin_bridge_for_traceability "Gherkin Bridge for Traceability" {
  status   accepted
  date     2026-03-04

  context """
    Inline scenario blocks inside .spec files cannot be executed by any test
    runner — they are structured prompts for AI agents. Real Cucumber/Gherkin
    .feature files are executable by standard runners (cucumber-rs, pytest-bdd,
    cucumber-js) and already have mature tooling. Bridging to .feature files
    provides actual executable traceability.
  """

  decision """
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

  consequences [
    "Scenarios are executable via standard Cucumber runners",
    "Feature files are the single source of truth for acceptance criteria",
    "@specforge:entity_id tags provide bidirectional traceability",
    "Requires .feature file authoring alongside specs",
  ]

  invariants [traceability_chain_integrity]
}
