// @specforge/compliance extension validation rules
//
// Declarative validation patterns interpreted by the core engine's
// execute_validation_pattern behavior. These rules are declared in the
// extension manifest and registered into the validation rule set.

use "extensions/compliance/manifest"
// W101: A control entity with no incoming Governs edge from any regulation.
// Indicates an orphan control not governed by any regulation.
//
// Pattern: no_incoming_edges
//   kind: control
//   edge_type: Governs
//   severity: warning
//   code: W101
//   message: "Control '{id}' is not governed by any regulation"

// W102: A regulation entity with no outgoing Governs edge to any control.
// Indicates a regulation with no implementing controls.
//
// Pattern: no_outgoing_edges
//   kind: regulation
//   edge_type: Governs
//   severity: warning
//   code: W102
//   message: "Regulation '{id}' has no implementing controls"

// W103: A control entity with maturity 'implemented' or higher but no
// incoming ProvidedBy edge from any evidence entity.
// Indicates a control claiming implementation without supporting evidence.
//
// Pattern: missing_field_on_condition
//   kind: control
//   condition: maturity in [implemented, tested, certified]
//   required_edge: ProvidedBy (incoming from evidence)
//   severity: warning
//   code: W103
//   message: "Control '{id}' claims '{maturity}' but has no supporting evidence"

// W104: An evidence entity whose expires field is set and the date has passed.
// Indicates expired evidence that may no longer be valid.
//
// Pattern: field_date_expired
//   kind: evidence
//   field: expires
//   severity: warning
//   code: W104
//   message: "Evidence '{id}' expired on {expires}"
