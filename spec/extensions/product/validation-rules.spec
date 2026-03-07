// @specforge/product extension validation rules
//
// These behaviors describe validation rules declared as
// ValidationRulePattern entries in the @specforge/product manifest.
// The core declarative validation engine executes these patterns.

use invariants/core
use invariants/validation
use extensions/product/invariants
use types/diagnostics
use types/graph

behavior detect_orphan_features "Detect Orphan Features" {
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST declare a no_incoming_edges
    validation pattern that detects features not referenced by any
    capability. Orphan features MUST produce a W041 warning. The core
    executes this pattern by checking the "implements" edge type.
  """

  verify unit "feature not in any capability produces W041"
  verify unit "feature in a capability suppresses W041"

}

behavior detect_library_cycles "Detect Library Cycles" {
  invariants [library_dag]
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST declare a cycle_detection
    validation pattern for the "depends_on" edge type among library
    entities. Cycles MUST produce an E007 diagnostic naming the
    libraries in the cycle.
  """

  verify unit "library cycle produces E007"
  verify unit "acyclic library graph passes"

}

behavior validate_behavior_ranges_in_roadmaps "Validate Behavior Ranges in Roadmaps" {
  invariants [reference_resolution_completeness]
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST declare a field_value_constraint
    validation pattern that, when a roadmap declares a behaviors range,
    verifies start <= end and that the expanded range contains only
    existing behavior IDs. Invalid ranges MUST produce an E010 diagnostic.
  """

  verify unit "valid range passes"
  verify unit "start > end produces E010"
  verify unit "range with non-existent behaviors produces E010"

}

behavior detect_orphan_capabilities "Detect Orphan Capabilities" {
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST declare a no_incoming_edges
    validation pattern that detects capabilities not referenced by any
    deliverable. Orphan capabilities MUST produce a W042 warning.
  """

  verify unit "capability not in any deliverable produces W042"
  verify unit "capability in a deliverable suppresses W042"

}

behavior detect_deliverables_with_no_capabilities "Detect Deliverables with No Capabilities" {
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST declare a field_value_constraint
    validation pattern that detects deliverables with an empty capabilities
    list. Deliverables with no capabilities MUST produce a W043 warning.
  """

  verify unit "deliverable with no capabilities produces W043"
  verify unit "deliverable with capabilities suppresses W043"

}

behavior detect_orphan_libraries "Detect Orphan Libraries" {
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST declare a no_incoming_edges
    validation pattern that detects libraries not bundled in any
    deliverable. Orphan libraries MUST produce a W044 warning.
  """

  verify unit "library not in any deliverable produces W044"
  verify unit "library in a deliverable suppresses W044"

}

behavior detect_unused_glossary_terms "Detect Unused Glossary Terms" {
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST declare a validation pattern
    (implemented via Wasm validate() for complex text scanning) that
    detects glossary terms not referenced by any entity's description
    or contract text. Unused terms MUST produce an I010 info diagnostic.
  """

  verify unit "glossary term referenced in a contract suppresses I010"
  verify unit "glossary term not referenced anywhere produces I010"

}

behavior validate_persona_references "Validate Persona References" {
  invariants [diagnostic_code_uniqueness]
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST validate that all persona
    references in entity fields resolve to persona entities declared
    in the project. References to undeclared personas MUST produce an
    E008 diagnostic. Valid persona references MUST pass without
    diagnostics.
  """

  verify unit "references to undeclared personas produce E008"
  verify unit "valid persona references pass without diagnostics"

}

behavior validate_surface_references "Validate Surface References" {
  invariants [diagnostic_code_uniqueness]
  types      [Diagnostic]

  contract """
    The @specforge/product extension MUST validate that all surface
    references in entity fields resolve to surface entities declared
    in the project. References to undeclared surfaces MUST produce an
    E009 diagnostic. Valid surface references MUST pass without
    diagnostics.
  """

  verify unit "references to undeclared surfaces produce E009"
  verify unit "valid surface references pass without diagnostics"

}
