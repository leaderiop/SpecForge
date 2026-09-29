// @specforge/testing behaviors

use "types/zero-entity-core"
use "extensions/testing/types"
use "extensions/testing/invariants"

behavior te_contribute_testability "Contribute Testability to Owned Kinds" {
  category command
  invariants [te_testable_kinds_from_one_table, te_owners_declare_no_test_vocabulary]
  types [TestingVerifyKind]

  contract """
    @specforge/testing MUST contribute testability as entity enhancements
    carrying verify kinds, one per testable kind, each naming the kind's
    owner. Applying such an enhancement marks the target kind testable
    with exactly those verify kinds. An enhancement whose owner the
    project does not use is skipped silently.
  """

  requires {
    kinds_registered "the owning extensions' kinds are registered before enhancements apply"
  }

  ensures {
    kinds_testable   "each table kind is testable with the table's verify kinds"
    owner_optional   "a kind whose owner is not loaded produces no diagnostic"
  }

  verify unit "testing makes software and governance kinds testable with their verify kinds"
  verify unit "an enhancement with verify kinds makes its target kind testable"
}

behavior te_validate_unverified_testable "W004: Unverified Testable Entities" {
  category query
  invariants [te_testable_kinds_from_one_table]
  types [ValidationRulePattern]

  contract """
    Detect entities of a testable software kind that declare no verify
    obligations and no gherkin scenario. Entities marked `abstract true`
    are exempt: their obligations are carried by the concretes that
    refine them. Governance constraints are testable but not required to
    declare obligations.
  """

  ensures {
    unverified_detected "testable entity with no verify and no gherkin produces W004"
    verified_passes     "testable entity with verify produces no diagnostic"
    abstract_exempt     "an abstract entity never produces W004"
  }

  verify unit "testable behavior with no verify produces W004"
  verify unit "testable behavior with verify passes"
}

behavior te_validate_verify_kind_allowlist "W009: Verify Kind Outside the Kind's Allowlist" {
  category query
  invariants [te_testable_kinds_from_one_table]
  types [ValidationRulePattern, TestingVerifyKind]

  contract """
    Detect `verify <kind> "..."` statements whose kind is not among the
    verify kinds @specforge/testing registered for the entity's kind.
    A bare `verify "..."` has no kind and always passes.
  """

  ensures {
    allowed_passes    "verify kind in the allowlist produces no diagnostic"
    disallowed_warned "verify kind outside the allowlist produces W009"
  }

  verify unit "behavior with verify unit passes (unit in allowedVerifyKinds)"
  verify unit "invariant with verify load produces W009 (load not in allowedVerifyKinds)"
  verify unit "W009 message includes allowed set"
}
