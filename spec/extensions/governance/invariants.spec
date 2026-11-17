requirement INVARIANT-124 "RPN Arithmetic Integrity" {
  category "Invariant"
  tier     v2
  text     """
    When a failure_mode block declares severity, occurrence, detection, and
    rpn fields, the rpn value MUST equal severity * occurrence * detection.
    The same MUST hold for the post_mitigation sub-block. The compiler MUST
    emit E005 on mismatch.
  """
}
