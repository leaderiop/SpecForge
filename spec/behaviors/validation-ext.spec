// Extended validation behaviors — plugin-specific validation rules

use invariants/core
use invariants/validation
use types/diagnostics
use types/graph

behavior detect_orphan_features "Detect Orphan Features" {
  types      [Diagnostic]

  contract """
    When @specforge/product is installed, the validator MUST detect features
    not referenced by any capability. Orphan features MUST produce a W002
    warning.
  """

  verify unit "feature not in any capability produces W002"
  verify unit "feature in a capability suppresses W002"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_library_cycles "Detect Library Cycles" {
  invariants [library_dag]
  types      [Diagnostic]

  contract """
    When @specforge/product is installed, the validator MUST detect circular
    dependencies in the library depends_on graph. Cycles MUST produce an
    E007 diagnostic naming the libraries in the cycle.
  """

  verify unit "library cycle produces E007"
  verify unit "acyclic library graph passes"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior validate_behavior_ranges_in_roadmaps "Validate Behavior Ranges in Roadmaps" {
  invariants [reference_resolution_completeness]
  types      [Diagnostic]

  contract """
    When @specforge/product is installed and a roadmap declares a behaviors
    range, the validator MUST verify start <= end and that the expanded
    range contains only existing behavior IDs. Invalid ranges MUST produce
    an E010 diagnostic.
  """

  verify unit "valid range passes"
  verify unit "start > end produces E010"
  verify unit "range with non-existent behaviors produces E010"

  tests ["../crates/specforge-validator/src/passes.rs"]
}

behavior validate_rpn_arithmetic "Validate RPN Arithmetic" {
  invariants [rpn_arithmetic_integrity]
  types      [Diagnostic]

  contract """
    When @specforge/governance is installed and a failure_mode declares
    severity, occurrence, detection, and rpn, the validator MUST verify
    rpn == severity * occurrence * detection. Mismatches MUST produce an
    E005 diagnostic. The same check MUST apply to post_mitigation blocks.
  """

  verify unit "correct RPN passes"
  verify unit "incorrect RPN produces E005"
  verify unit "post_mitigation RPN also validated"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_unmitigated_high_risk_invariants "Detect Unmitigated High-Risk Invariants" {
  types      [Diagnostic]

  contract """
    When @specforge/governance is installed, the validator MUST detect
    invariants with risk: high that have no corresponding failure_mode.
    Unmitigated high-risk invariants MUST produce a W005 warning.
  """

  verify unit "high-risk invariant without failure_mode produces W005"
  verify unit "high-risk invariant with failure_mode suppresses W005"
  verify unit "medium-risk invariant without failure_mode does not produce W005"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_orphan_capabilities "Detect Orphan Capabilities" {
  types      [Diagnostic]

  contract """
    When @specforge/product is installed, the validator MUST detect
    capabilities not referenced by any deliverable. Orphan capabilities
    MUST produce a W006 warning.
  """

  verify unit "capability not in any deliverable produces W006"
  verify unit "capability in a deliverable suppresses W006"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_features_with_empty_behaviors "Detect Features with Empty Behaviors" {
  types      [Diagnostic]

  contract """
    The validator MUST detect features that declare an empty behaviors
    list. Features with no behaviors MUST produce a W008 warning, since
    they represent unimplemented specification.
  """

  verify unit "feature with empty behaviors list produces W008"
  verify unit "feature with at least one behavior suppresses W008"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

// Extended validation (continued) — structural completeness checks

behavior detect_deliverables_with_no_capabilities "Detect Deliverables with No Capabilities" {
  types      [Diagnostic]

  contract """
    When @specforge/product is installed, the validator MUST detect
    deliverables that declare an empty capabilities list. Deliverables
    with no capabilities MUST produce a W009 warning.
  """

  verify unit "deliverable with no capabilities produces W009"
  verify unit "deliverable with capabilities suppresses W009"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_orphan_libraries "Detect Orphan Libraries" {
  types      [Diagnostic]

  contract """
    When @specforge/product is installed, the validator MUST detect
    libraries not bundled in any deliverable. Orphan libraries MUST
    produce a W010 warning.
  """

  verify unit "library not in any deliverable produces W010"
  verify unit "library in a deliverable suppresses W010"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_constraints_with_no_protected_invariants "Detect Constraints with No Protected Invariants" {
  types      [Diagnostic]

  contract """
    When @specforge/governance is installed, the validator MUST detect
    constraints whose invariants list is empty or references no existing
    invariants. Such constraints MUST produce a W011 warning.
  """

  verify unit "constraint with empty protects list produces W011"
  verify unit "constraint with valid protects suppresses W011"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

behavior detect_unused_glossary_terms "Detect Unused Glossary Terms" {
  types      [Diagnostic]

  contract """
    When @specforge/product is installed, the validator MUST detect
    glossary terms that are not referenced by any entity's description
    or contract text. Unused terms MUST produce an I001 info diagnostic.
  """

  verify unit "glossary term referenced in a contract suppresses I001"
  verify unit "glossary term not referenced anywhere produces I001"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}
