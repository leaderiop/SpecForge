// @specforge/governance extension invariants
//
// Runtime guarantees specific to the governance entity model:
// decision, constraint, failure_mode.

invariant rpn_arithmetic_integrity "RPN Arithmetic Integrity" {
  guarantee """
    When a failure_mode block declares severity, occurrence, detection, and
    rpn fields, the rpn value MUST equal severity * occurrence * detection.
    The same MUST hold for the post_mitigation sub-block. The compiler MUST
    emit E005 on mismatch.
  """
  enforced_by [validate_rpn_arithmetic]
  risk low

  verify property "rpn equals severity times occurrence times detection when all fields are present"
  verify unit "E005 is emitted when rpn does not match the arithmetic product"

}
