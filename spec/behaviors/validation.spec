// Validation behaviors — core graph validation rules

use invariants/core
use invariants/validation
use types/core
use types/graph
use types/diagnostics
use types/errors

behavior detect_dangling_references "Detect Dangling References" {
  invariants [reference_resolution_completeness]
  types      [Diagnostic, EntityId, ValidationCode, Severity, ValidationError]

  contract """
    The validator MUST check every entity ID in every reference list
    against the graph's node registry. IDs that do not resolve to any
    declared entity MUST produce an E001 diagnostic with source location
    and a "did you mean?" suggestion.
  """

  verify unit "missing reference produces E001"
  verify unit "valid reference passes silently"
  verify unit "close match produces suggestion"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_duplicate_entity_ids "Detect Duplicate Entity IDs" {
  invariants [string_interning_consistency, entity_id_uniqueness]
  types      [Diagnostic, DuplicateIdError]

  contract """
    The validator MUST detect entity IDs declared more than once across
    all .spec files. Duplicate IDs MUST produce an E002 diagnostic that
    names both declaration sites (file, line, column).
  """

  verify unit "duplicate ID in same file produces E002"
  verify unit "duplicate ID across files produces E002"
  verify unit "E002 includes both source locations"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_import_cycles "Detect Import Cycles" {
  invariants [import_dag]
  types      [Diagnostic, CycleError]

  contract """
    The validator MUST verify the import graph is acyclic. Import cycles
    MUST produce an E003 diagnostic listing the cycle participants in order.
  """

  verify unit "import cycle produces E003"
  verify unit "E003 lists all cycle participants"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_orphan_behaviors "Detect Orphan Behaviors" {
  types      [Diagnostic, EntityId]

  contract """
    The validator MUST detect behaviors not referenced by any feature.
    Orphan behaviors MUST produce a W001 warning with the behavior's
    source location.
  """

  verify unit "behavior not in any feature produces W001"
  verify unit "behavior in a feature suppresses W001"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_unused_invariants "Detect Unused Invariants" {
  types      [Diagnostic, EntityId]

  contract """
    The validator MUST detect invariants not referenced by any behavior.
    Unused invariants MUST produce a W003 warning.
  """

  verify unit "invariant not referenced by any behavior produces W003"
  verify unit "referenced invariant suppresses W003"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_unverified_behaviors "Detect Unverified Behaviors" {
  types      [Diagnostic, EntityId]

  contract """
    The validator MUST detect behaviors that lack any verify statement.
    Behaviors without verify statements MUST produce a W004 warning.
  """

  verify unit "behavior without verify produces W004"
  verify unit "behavior with verify suppresses W004"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_orphan_events "Detect Orphan Events" {
  types      [Diagnostic, EntityId]

  contract """
    The validator MUST detect events with no consumers in their consumers
    list. Events without consumers MUST produce a W007 warning.
  """

  verify unit "event without consumers produces W007"
  verify unit "event with consumers suppresses W007"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_event_triggers "Validate Event Triggers" {
  invariants [reference_resolution_completeness]
  types      [Diagnostic, EntityId]

  contract """
    The validator MUST verify that every event's trigger field references
    an existing behavior. Invalid triggers MUST produce an E006 diagnostic.
  """

  verify unit "valid trigger passes"
  verify unit "non-existent trigger produces E006"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_persona_references "Validate Persona References" {
  types      [Diagnostic, EntityId]

  contract """
    When the spec root defines personas, the validator MUST verify that
    every capability's persona matches a declared persona. Undeclared
    personas MUST produce an E008 diagnostic.
  """

  verify unit "valid persona passes"
  verify unit "undeclared persona produces E008"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_surface_references "Validate Surface References" {
  types      [Diagnostic, EntityId]

  contract """
    When the spec root defines surfaces, the validator MUST verify that
    every capability's surface values match declared surfaces. Undeclared
    surfaces MUST produce an E009 diagnostic.
  """

  verify unit "valid surface passes"
  verify unit "undeclared surface produces E009"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_orphan_refs "Detect Orphan Refs" {
  types      [Diagnostic, EntityId]

  contract """
    The validator MUST detect ref entities declared but never referenced
    by any entity's refs field. Orphan refs MUST produce a W012 warning.
  """

  verify unit "unreferenced ref produces W012"
  verify unit "referenced ref suppresses W012"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_empty_scenario "Validate Empty Scenario" {
  invariants [traceability_chain_integrity]
  types      [Diagnostic, Scenario]

  contract """
    The validator MUST detect scenario blocks that contain no steps.
    An empty scenario block MUST produce an E004 diagnostic with
    the scenario title and source location.
  """

  verify unit "scenario with no steps produces E004"
  verify unit "scenario with steps suppresses E004"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_duplicate_scenario_titles "Validate Duplicate Scenario Titles" {
  invariants [traceability_chain_integrity]
  types      [Diagnostic, Scenario]

  contract """
    The validator MUST detect duplicate scenario titles within the
    same entity. When two or more scenarios share the same title,
    the validator MUST produce an E015 diagnostic for each duplicate.
  """

  verify unit "duplicate scenario title in same entity produces E015"
  verify unit "same scenario title in different entities is allowed"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_scenario_steps "Validate Scenario Steps" {
  invariants [traceability_chain_integrity]
  types      [Diagnostic, Scenario, ScenarioStep]

  contract """
    The validator MUST detect scenarios missing required step kinds.
    A scenario without a when step MUST produce a W015 warning.
    A scenario without a then step MUST produce a W016 warning.
    A scenario MAY omit a given step without warning.
  """

  verify unit "scenario without when step produces W015"
  verify unit "scenario without then step produces W016"
  verify unit "scenario with all step kinds produces no warnings"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_tests_field_references "Validate Tests Field References" {
  invariants [traceability_chain_integrity]
  types      [Diagnostic, TestFileRef]

  contract """
    The validator MUST check that every path in a tests field references
    an existing file. Non-existent test files MUST produce an E016
    diagnostic. Testable entities that have verify or scenario declarations
    but no tests field MUST produce a W018 warning.
  """

  verify unit "non-existent test file path produces E016"
  verify unit "existing test file path passes silently"
  verify unit "testable entity with verify but no tests field produces W018"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_plugin_testability "Validate Plugin Testability" {
  invariants [testable_entity_classification]
  types      [Diagnostic]

  contract """
    The validator MUST detect inconsistencies between plugin entity
    testability declarations and grammar support. A plugin entity marked
    testable but whose grammar lacks verify/scenario support MUST produce
    a W017 warning. An entity that supports verify but is not marked
    testable MUST produce an I006 info diagnostic.
  """

  verify unit "testable plugin entity without verify support produces W017"
  verify unit "entity with verify support but not marked testable produces I006"

  tests ["../crates/specforge-validator/src/passes.rs"]
}
