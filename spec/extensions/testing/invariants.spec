// @specforge/testing invariants

use "extensions/testing/types"

invariant te_testable_kinds_from_one_table "Testability, Allowed Verify Kinds, and W009 Come From One Table" {
  guarantee """
    Every kind @specforge/testing makes testable, the verify kinds it
    allows, and the W004/W009 rules over it are generated from a single
    table in the extension. The allowlist W009 enforces is exactly the
    kind's registered verify kinds, so the two can never disagree.
    Testable kinds: behavior, invariant, event, type, port
    (@specforge/software), constraint and failure_mode
    (@specforge/governance).
  """
  risk medium

  verify unit "testing makes software and governance kinds testable with their verify kinds"
}

invariant te_owners_declare_no_test_vocabulary "Kind Owners Declare No Test Vocabulary" {
  guarantee """
    Extensions that own entity kinds (@specforge/software,
    @specforge/governance) declare no testability, verify kinds, or
    verify rules of their own. Without @specforge/testing a project has
    no test obligations: W004 and W009 never fire.
  """
  risk medium

  verify unit "software kinds declare no testability of their own"
}
