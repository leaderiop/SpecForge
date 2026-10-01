// @specforge/product extension validation rules
//
// These behaviors describe validation rules declared as
// ValidationRulePattern entries in the @specforge/product manifest.
// The core declarative validation engine executes these patterns.

use "extensions/product/invariants"
use "invariants/core"
use "invariants/validation"
use "types/diagnostics"
use "types/graph"

behavior detect_orphan_features "Detect Orphan Features" {
  features [pe_validation_suite]
  category validation
  types    [Diagnostic]
  contract """
    The @specforge/product extension MUST declare a no_incoming_edges
    validation pattern that detects features no journey, milestone or
    module references. Orphan features MUST produce a W041 warning.
  """
  verify unit "a feature no journey, milestone or module references produces W041"
  verify unit "a referenced feature suppresses W041"
}

behavior validate_persona_references "Validate Persona References" {
  features   [pe_validation_suite]
  category   validation
  invariants [diagnostic_code_uniqueness]
  types      [Diagnostic]
  contract   """
    A persona reference in an entity field MUST resolve to a persona
    declared in the project. The @specforge/product extension declares
    persona fields as references, so the core resolves them: a reference
    to an undeclared persona is an unresolved reference (E003).
  """
  verify unit "a reference to an undeclared persona produces E003"
  verify unit "valid persona references pass without diagnostics"
}
